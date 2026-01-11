// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Token storage abstraction supporting both file-based and OS keychain storage

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::types::StoredToken;

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    /// File-based storage (DEPRECATED - use only as fallback)
    File,
    /// OS keychain storage (Windows Credential Manager, macOS Keychain, Linux Secret Service) - DEFAULT
    Keychain,
}

impl StorageBackend {
    /// Determine storage backend from profile configuration or environment
    /// Defaults to Keychain for security, falls back to File only if explicitly disabled
    pub fn from_env() -> Self {
        // First check profile configuration
        if is_keychain_disabled_in_profile() {
            eprintln!(
                "WARNING: File-based token storage enabled in profile. Tokens will be stored in plaintext."
            );
            eprintln!("Consider enabling keychain storage: raps config set use_keychain true");
            return StorageBackend::File;
        }

        // Fall back to environment variable for backward compatibility
        let use_file = std::env::var("RAPS_USE_FILE_STORAGE")
            .ok()
            .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
            .unwrap_or(false);

        if use_file {
            eprintln!(
                "WARNING: Using file-based token storage. Tokens will be stored in plaintext."
            );
            eprintln!(
                "Consider using keychain storage for better security (remove RAPS_USE_FILE_STORAGE env var)."
            );
            StorageBackend::File
        } else {
            // Default to keychain for security
            StorageBackend::Keychain
        }
    }
}

/// Helper function to check if keychain is disabled in profile configuration
fn is_keychain_disabled_in_profile() -> bool {
    // Avoid circular dependency by checking the profile file directly
    let proj_dirs = match directories::ProjectDirs::from("com", "autodesk", "raps") {
        Some(dirs) => dirs,
        None => return false,
    };

    let profiles_path = proj_dirs.config_dir().join("profiles.json");
    if !profiles_path.exists() {
        return false;
    }

    let content = match std::fs::read_to_string(&profiles_path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    // Parse JSON to check use_keychain setting
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(active) = data["active_profile"].as_str()
        && let Some(profile) = data["profiles"][active].as_object()
        && let Some(use_keychain) = profile.get("use_keychain")
    {
        return use_keychain.as_bool() == Some(false);
    }

    false
}

/// Token storage abstraction
pub struct TokenStorage {
    backend: StorageBackend,
    service_name: String,
    username: String,
}

impl TokenStorage {
    /// Create a new token storage instance
    pub fn new(backend: StorageBackend) -> Self {
        Self {
            backend,
            service_name: "raps".to_string(),
            username: "aps_token".to_string(),
        }
    }

    /// Get the file path for file-based storage
    fn token_file_path() -> PathBuf {
        directories::ProjectDirs::from("com", "autodesk", "raps")
            .expect("Failed to get project directories")
            .config_dir()
            .join("tokens.json")
    }

    /// Save token using the configured backend
    pub fn save(&self, token: &StoredToken) -> Result<()> {
        match self.backend {
            StorageBackend::File => self.save_file(token),
            StorageBackend::Keychain => self.save_keychain(token),
        }
    }

    /// Load token using the configured backend
    pub fn load(&self) -> Result<Option<StoredToken>> {
        match self.backend {
            StorageBackend::File => self.load_file(),
            StorageBackend::Keychain => self.load_keychain(),
        }
    }

    /// Delete token using the configured backend
    pub fn delete(&self) -> Result<()> {
        match self.backend {
            StorageBackend::File => self.delete_file(),
            StorageBackend::Keychain => self.delete_keychain(),
        }
    }

    /// Save token to file (INSECURE - logs warning)
    fn save_file(&self, token: &StoredToken) -> Result<()> {
        eprintln!("Storing token in plaintext file. Use keychain for better security.");
        let path = Self::token_file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Add a warning to the file itself
        let json = serde_json::json!({
            "_warning": "This file contains sensitive authentication tokens in plaintext. Consider using keychain storage.",
            "access_token": token.access_token,
            "refresh_token": token.refresh_token,
            "expires_at": token.expires_at,
            "scopes": token.scopes,
        });

        let json_string = serde_json::to_string_pretty(&json)?;
        std::fs::write(&path, json_string)?;

        // Set restrictive permissions on Unix-like systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path)?.permissions();
            perms.set_mode(0o600); // Read/write for owner only
            std::fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Load token from file
    fn load_file(&self) -> Result<Option<StoredToken>> {
        let path = Self::token_file_path();
        if !path.exists() {
            return Ok(None);
        }

        eprintln!("Loading token from plaintext file. Consider migrating to keychain storage.");

        let contents = std::fs::read_to_string(&path)?;

        // Try to parse as our new format with warning field
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&contents) {
            // Extract the token fields, ignoring the _warning field
            let token = StoredToken {
                access_token: json_value["access_token"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing access_token"))?
                    .to_string(),
                refresh_token: json_value["refresh_token"].as_str().map(|s| s.to_string()),
                expires_at: json_value["expires_at"].as_i64().unwrap_or(0),
                scopes: json_value["scopes"]
                    .as_array()
                    .and_then(|arr| {
                        arr.iter()
                            .map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Option<Vec<_>>>()
                    })
                    .unwrap_or_default(),
            };
            return Ok(Some(token));
        }

        // Fall back to parsing as the old format
        let token: StoredToken =
            serde_json::from_str(&contents).context("Failed to parse token file")?;
        Ok(Some(token))
    }

    /// Delete token file
    fn delete_file(&self) -> Result<()> {
        let path = Self::token_file_path();
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Save token to OS keychain
    fn save_keychain(&self, token: &StoredToken) -> Result<()> {
        // Serialize token to JSON
        let json = serde_json::to_string(token).context("Failed to serialize token")?;

        // Store in keychain
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(e) => {
                // If keyring is not available, fall back to file storage
                crate::logging::log_verbose(&format!(
                    "Keychain not available ({}), falling back to file storage",
                    e
                ));
                return self.save_file(token);
            }
        };

        match entry.set_password(&json) {
            Ok(()) => Ok(()),
            Err(e) => {
                // If keychain save fails, fall back to file storage
                crate::logging::log_verbose(&format!(
                    "Keychain save failed ({}), falling back to file storage",
                    e
                ));
                self.save_file(token)
            }
        }
    }

    /// Load token from OS keychain
    fn load_keychain(&self) -> Result<Option<StoredToken>> {
        let entry = keyring::Entry::new(&self.service_name, &self.username)
            .context("Failed to create keyring entry")?;

        match entry.get_password() {
            Ok(json) => {
                let token: StoredToken =
                    serde_json::from_str(&json).context("Failed to parse token from keychain")?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to load token from keychain: {}", e)),
        }
    }

    /// Delete token from OS keychain
    fn delete_keychain(&self) -> Result<()> {
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(_) => {
                // If keyring is not available, try deleting file storage
                return self.delete_file();
            }
        };

        match entry.delete_password() {
            Ok(()) => {
                // Also delete file storage if it exists (for migration)
                self.delete_file().ok();
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // Already deleted, also try file storage
                self.delete_file()
            }
            Err(e) => {
                // If keychain delete fails, try file storage
                crate::logging::log_verbose(&format!(
                    "Keychain delete failed ({}), trying file storage",
                    e
                ));
                self.delete_file()
            }
        }
    }

    /// Get the current backend being used
    #[allow(dead_code)]
    pub fn backend(&self) -> StorageBackend {
        self.backend
    }

    /// Migrate tokens from file storage to keychain storage
    #[allow(dead_code)]
    pub fn migrate_to_keychain() -> Result<()> {
        println!("Migrating tokens from file storage to secure keychain storage...");

        // First, try to load from file storage
        let file_storage = TokenStorage::new(StorageBackend::File);
        let token = match file_storage.load()? {
            Some(t) => t,
            None => {
                println!("No tokens found in file storage.");
                return Ok(());
            }
        };

        // Save to keychain
        let keychain_storage = TokenStorage::new(StorageBackend::Keychain);
        keychain_storage.save(&token)?;
        println!("Token successfully migrated to keychain storage.");

        // Delete the file storage
        file_storage.delete_file()?;
        println!("Removed plaintext token file.");

        println!("Migration complete! Your tokens are now securely stored in the OS keychain.");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_from_env() {
        // Note: Environment variable manipulation is not thread-safe in Rust.
        // These tests should be run with --test-threads=1 for safety.
        // However, since we're only modifying test-specific variables,
        // the risk is minimal in practice.

        // Helper to temporarily set environment variables for testing
        struct EnvGuard {
            key: String,
            original: Option<String>,
        }

        impl EnvGuard {
            fn new(key: &str, value: Option<&str>) -> Self {
                let original = std::env::var(key).ok();
                match value {
                    Some(v) => unsafe { std::env::set_var(key, v) },
                    None => unsafe { std::env::remove_var(key) },
                }
                EnvGuard {
                    key: key.to_string(),
                    original,
                }
            }
        }

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                match &self.original {
                    Some(v) => unsafe { std::env::set_var(&self.key, v) },
                    None => unsafe { std::env::remove_var(&self.key) },
                }
            }
        }

        // Test default (now keychain for security)
        {
            let _guard = EnvGuard::new("RAPS_USE_FILE_STORAGE", None);
            assert_eq!(StorageBackend::from_env(), StorageBackend::Keychain);
        }

        // Test file storage enabled (insecure)
        {
            let _guard = EnvGuard::new("RAPS_USE_FILE_STORAGE", Some("true"));
            assert_eq!(StorageBackend::from_env(), StorageBackend::File);
        }

        {
            let _guard = EnvGuard::new("RAPS_USE_FILE_STORAGE", Some("1"));
            assert_eq!(StorageBackend::from_env(), StorageBackend::File);
        }

        {
            let _guard = EnvGuard::new("RAPS_USE_FILE_STORAGE", Some("yes"));
            assert_eq!(StorageBackend::from_env(), StorageBackend::File);
        }

        // Test file storage disabled (default to keychain)
        {
            let _guard = EnvGuard::new("RAPS_USE_FILE_STORAGE", Some("false"));
            assert_eq!(StorageBackend::from_env(), StorageBackend::Keychain);
        }
    }
}
