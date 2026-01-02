// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Token storage implementation

use crate::auth::StoredToken;
use crate::error::{RapsError, Result};
use std::path::PathBuf;

use super::StorageBackend;

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
            std::fs::create_dir_all(parent).map_err(|e| RapsError::Storage {
                message: format!("Failed to create config directory: {}", e),
                source: Some(anyhow::anyhow!("{}", e)),
            })?;
        }
        let json = serde_json::to_string_pretty(token).map_err(|e| RapsError::Internal {
            message: format!("Failed to serialize token: {}", e),
        })?;
        std::fs::write(&path, json).map_err(|e| RapsError::Storage {
            message: format!("Failed to write token file: {}", e),
            source: Some(anyhow::anyhow!("{}", e)),
        })?;
        Ok(())
    }

    /// Load token from file
    fn load_file(&self) -> Result<Option<StoredToken>> {
        let path = Self::token_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let contents = std::fs::read_to_string(&path).map_err(|e| RapsError::Storage {
            message: format!("Failed to read token file: {}", e),
            source: Some(anyhow::anyhow!("{}", e)),
        })?;
        let token: StoredToken =
            serde_json::from_str(&contents).map_err(|e| RapsError::Internal {
                message: format!("Failed to parse token file: {}", e),
            })?;
        Ok(Some(token))
    }

    /// Delete token file
    fn delete_file(&self) -> Result<()> {
        let path = Self::token_file_path();
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| RapsError::Storage {
                message: format!("Failed to delete token file: {}", e),
                source: Some(anyhow::anyhow!("{}", e)),
            })?;
        }
        Ok(())
    }

    /// Save token to OS keychain
    fn save_keychain(&self, token: &StoredToken) -> Result<()> {
        // Serialize token to JSON
        let json = serde_json::to_string(token).map_err(|e| RapsError::Internal {
            message: format!("Failed to serialize token: {}", e),
        })?;

        // Store in keychain
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(e) => {
                // If keyring is not available, fall back to file storage
                tracing::warn!(
                    "Keychain not available ({}), falling back to file storage",
                    e
                );
                return self.save_file(token);
            }
        };

        match entry.set_password(&json) {
            Ok(()) => Ok(()),
            Err(e) => {
                // If keychain save fails, fall back to file storage
                tracing::warn!("Keychain save failed ({}), falling back to file storage", e);
                self.save_file(token)
            }
        }
    }

    /// Load token from OS keychain
    fn load_keychain(&self) -> Result<Option<StoredToken>> {
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(_) => {
                // If keyring is not available, try file storage
                return self.load_file();
            }
        };

        match entry.get_password() {
            Ok(json) => {
                let token: StoredToken =
                    serde_json::from_str(&json).map_err(|e| RapsError::Internal {
                        message: format!("Failed to parse token from keychain: {}", e),
                    })?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => {
                // No entry in keychain, try file storage
                self.load_file()
            }
            Err(e) => {
                // Other keyring error, try file storage
                tracing::warn!("Keychain load failed ({}), trying file storage", e);
                self.load_file()
            }
        }
    }

    /// Delete token from OS keychain
    fn delete_keychain(&self) -> Result<()> {
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(_) => {
                // If keyring is not available, try file storage
                return self.delete_file();
            }
        };

        match entry.delete_password() {
            Ok(()) => {
                // Also delete file if it exists
                self.delete_file().ok();
                Ok(())
            }
            Err(keyring::Error::NoEntry) => {
                // No entry in keychain, try file storage
                self.delete_file()
            }
            Err(e) => {
                // Other keyring error, try file storage
                tracing::warn!("Keychain delete failed ({}), trying file storage", e);
                self.delete_file()
            }
        }
    }
}
