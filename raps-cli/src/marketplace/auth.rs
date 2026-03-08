// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Marketplace license key storage using the system keyring.

use anyhow::{Context, Result};
use keyring::Entry;

const SERVICE: &str = "raps-marketplace";
const ACCOUNT_LICENSE: &str = "license-key";

/// Manages marketplace license key storage in the system keyring.
pub struct MarketplaceAuth;

impl MarketplaceAuth {
    /// Store a license key in the system keyring.
    pub fn store_license_key(key: &str) -> Result<()> {
        let entry = Entry::new(SERVICE, ACCOUNT_LICENSE)
            .context("Failed to create keyring entry")?;
        entry.set_password(key).context("Failed to store license key in keyring")?;
        Ok(())
    }

    /// Retrieve the stored license key from the system keyring.
    /// Returns `None` if no key has been stored.
    pub fn get_license_key() -> Result<Option<String>> {
        let entry = Entry::new(SERVICE, ACCOUNT_LICENSE)
            .context("Failed to create keyring entry")?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("Failed to retrieve license key from keyring"),
        }
    }

    /// Remove the stored license key from the system keyring.
    pub fn clear_license_key() -> Result<()> {
        let entry = Entry::new(SERVICE, ACCOUNT_LICENSE)
            .context("Failed to create keyring entry")?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()), // Already cleared
            Err(e) => Err(e).context("Failed to clear license key from keyring"),
        }
    }

    /// Returns `true` if a license key is stored in the keyring.
    pub fn is_authenticated() -> bool {
        Self::get_license_key().ok().flatten().is_some()
    }

    /// Returns the stored license key (alias for `get_license_key`).
    /// Used by the HTTP client to get the bearer token.
    pub fn get_access_token() -> Result<Option<String>> {
        Self::get_license_key()
    }

    /// Alias for `clear_license_key` — clears all stored credentials.
    pub fn clear_tokens() -> Result<()> {
        Self::clear_license_key()
    }

    /// Marketplace does not use username/password login.
    /// Users authenticate via license key (`raps marketplace license <key>`).
    pub fn login(_email: &str, _password: &str) -> Result<()> {
        anyhow::bail!(
            "Marketplace authentication uses a license key.\n\
             Run `raps marketplace license <key>` to store your license key."
        )
    }

    /// Load stored tokens — returns the license key if present.
    pub fn load_tokens() -> Result<Option<String>> {
        Self::get_license_key()
    }
}
