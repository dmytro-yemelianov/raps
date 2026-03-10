// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Marketplace license key storage using the system keyring with file fallback.

use anyhow::{Context, Result};
use directories::BaseDirs;
use keyring::Entry;
use std::path::PathBuf;

const SERVICE: &str = "raps-marketplace";
const ACCOUNT_LICENSE: &str = "license-key";

/// Manages marketplace license key storage in the system keyring,
/// with automatic file-based fallback for headless servers and CI/CD.
pub struct MarketplaceAuth;

impl MarketplaceAuth {
    /// Path to the file-based fallback for the license key.
    fn file_path() -> Option<PathBuf> {
        BaseDirs::new().map(|b| b.config_dir().join("raps").join("marketplace_key"))
    }

    /// Store a license key — tries keyring first, falls back to file.
    pub fn store_license_key(key: &str) -> Result<()> {
        match Entry::new(SERVICE, ACCOUNT_LICENSE) {
            Ok(entry) => match entry.set_password(key) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(error = %e, "Keychain not available, using file storage for license key. This is normal on headless servers and CI/CD.");
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Keychain not available, using file storage for license key. This is normal on headless servers and CI/CD.");
            }
        }
        Self::save_file(key)
    }

    /// Retrieve the stored license key — tries keyring first, falls back to file.
    /// Returns `None` if no key has been stored.
    pub fn get_license_key() -> Result<Option<String>> {
        match Entry::new(SERVICE, ACCOUNT_LICENSE) {
            Ok(entry) => match entry.get_password() {
                Ok(key) => return Ok(Some(key)),
                Err(keyring::Error::NoEntry) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "Keychain not available, checking file storage for license key.");
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Keychain not available, checking file storage for license key.");
            }
        }
        Self::load_file()
    }

    /// Remove the stored license key from keyring and file.
    pub fn clear_license_key() -> Result<()> {
        // Try keyring
        if let Ok(entry) = Entry::new(SERVICE, ACCOUNT_LICENSE) {
            let _ = entry.delete_credential();
        }
        // Also delete file if it exists
        if let Some(path) = Self::file_path() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    /// Returns `true` if a license key is stored in the keyring or file.
    pub fn is_authenticated() -> bool {
        Self::get_license_key().ok().flatten().is_some()
    }

    /// Alias for `clear_license_key` — clears all stored credentials.
    pub fn clear_tokens() -> Result<()> {
        Self::clear_license_key()
    }

    fn save_file(key: &str) -> Result<()> {
        let path = Self::file_path().context("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, key)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    fn load_file() -> Result<Option<String>> {
        let path = match Self::file_path() {
            Some(p) => p,
            None => return Ok(None),
        };
        match std::fs::read_to_string(&path) {
            Ok(key) => {
                let key = key.trim().to_string();
                if key.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(key))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("Failed to read license key file"),
        }
    }
}
