//! Token storage abstraction supporting both file-based and OS keychain storage

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::api::auth::StoredToken;

/// Storage backend type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    /// File-based storage (default)
    File,
    /// OS keychain storage (Windows Credential Manager, macOS Keychain, Linux Secret Service)
    Keychain,
}

impl StorageBackend {
    /// Determine storage backend from environment variable or config
    pub fn from_env() -> Self {
        std::env::var("RAPS_USE_KEYCHAIN")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(StorageBackend::Keychain),
                _ => None,
            })
            .unwrap_or(StorageBackend::File)
    }
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

    /// Save token to file
    fn save_file(&self, token: &StoredToken) -> Result<()> {
        let path = Self::token_file_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(token)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load token from file
    fn load_file(&self) -> Result<Option<StoredToken>> {
        let path = Self::token_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path)?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_backend_from_env() {
        // Test default (file)
        std::env::remove_var("RAPS_USE_KEYCHAIN");
        assert_eq!(StorageBackend::from_env(), StorageBackend::File);

        // Test keychain enabled
        std::env::set_var("RAPS_USE_KEYCHAIN", "true");
        assert_eq!(StorageBackend::from_env(), StorageBackend::Keychain);

        std::env::set_var("RAPS_USE_KEYCHAIN", "1");
        assert_eq!(StorageBackend::from_env(), StorageBackend::Keychain);

        std::env::set_var("RAPS_USE_KEYCHAIN", "yes");
        assert_eq!(StorageBackend::from_env(), StorageBackend::Keychain);

        // Test disabled
        std::env::set_var("RAPS_USE_KEYCHAIN", "false");
        assert_eq!(StorageBackend::from_env(), StorageBackend::File);

        std::env::remove_var("RAPS_USE_KEYCHAIN");
    }
}
