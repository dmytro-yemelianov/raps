// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Plugin download, verification, and installation.

use anyhow::{Context, Result};
use raps_kernel::marketplace::Installation;
use std::path::PathBuf;

use super::{auth::MarketplaceAuth, client::MarketplaceClient, subscription::SubscriptionManager};
use crate::plugins::{PluginConfig, PluginEntry};

/// Ed25519 public key for verifying marketplace plugin binaries.
/// Set via RAPS_MARKETPLACE_ED25519_PUBKEY env var at build time.
/// Uses a zero placeholder in development builds.
const ED25519_PUBLIC_KEY_HEX: &str = {
    match option_env!("RAPS_MARKETPLACE_ED25519_PUBKEY") {
        Some(key) => key,
        None => "0000000000000000000000000000000000000000000000000000000000000000",
    }
};

/// Returns the platform string for the current build target.
pub fn detect_platform() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "linux-x64";
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    return "darwin-arm64";
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    return "darwin-x64";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "win-x64";
    #[cfg(not(any(
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "x86_64"),
    )))]
    return "unknown";
}

/// Returns the default plugin install directory.
pub fn install_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(home) = std::env::var_os("USERPROFILE") {
            PathBuf::from(home).join(".raps").join("bin")
        } else {
            PathBuf::from("C:\\raps\\bin")
        }
    }
    #[cfg(not(windows))]
    {
        if let Some(home) = std::env::var_os("HOME") {
            PathBuf::from(home).join(".local").join("bin")
        } else {
            PathBuf::from("/usr/local/bin")
        }
    }
}

/// Compute SHA-256 hex digest of bytes.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verify an Ed25519 signature.
fn verify_ed25519(public_key_hex: &str, message: &[u8], signature_hex: &str) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let key_bytes = hex::decode(public_key_hex).context("Invalid public key hex")?;
    let key_arr: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Public key must be 32 bytes"))?;
    let verifying_key = VerifyingKey::from_bytes(&key_arr).context("Invalid Ed25519 public key")?;
    let sig_bytes = hex::decode(signature_hex).context("Invalid signature hex")?;
    let sig_arr: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Signature must be 64 bytes"))?;
    let signature = Signature::from_bytes(&sig_arr);
    verifying_key
        .verify(message, &signature)
        .context("Ed25519 signature verification failed")
}

/// Manages marketplace plugin installation.
pub struct PluginInstaller {
    install_dir: PathBuf,
}

impl PluginInstaller {
    /// Create a new installer using the default install directory.
    pub fn new() -> Self {
        Self {
            install_dir: install_dir(),
        }
    }

    /// Create an installer with a custom install directory (for testing).
    #[allow(dead_code)]
    pub fn with_install_dir(dir: PathBuf) -> Self {
        Self { install_dir: dir }
    }

    /// Install or update a marketplace plugin.
    ///
    /// 1. Gets license key from keyring
    /// 2. Validates license and checks entitlement for the plugin
    /// 3. Downloads binary
    /// 4. Verifies SHA-256 and Ed25519 signature
    /// 5. Atomically installs to install_dir
    /// 6. Updates plugins.json
    pub async fn install(&self, slug: &str) -> Result<Installation> {
        // Get license key
        let key = MarketplaceAuth::get_license_key()?.ok_or_else(|| {
            anyhow::anyhow!(
                "No license key found. Run `raps marketplace license <key>` first."
            )
        })?;

        // Validate license and check entitlement
        let cached = SubscriptionManager::validate(&key).await?;
        if !cached.plugins.iter().any(|p| p == slug) {
            anyhow::bail!(
                "Your license does not include plugin '{}'. Check your subscription at https://buy.rapscli.xyz",
                slug
            );
        }

        let platform = detect_platform();
        if platform == "unknown" {
            anyhow::bail!("Unsupported platform — marketplace plugins are available for linux-x64, darwin-arm64, and win-x64");
        }

        // Download
        let client = MarketplaceClient::new()?;
        let (bytes, sha256_header, sig_header, version) =
            client.download_plugin(slug, platform, &key).await?;

        // Verify SHA-256
        let computed_sha256 = sha256_hex(&bytes);
        if !sha256_header.is_empty() && computed_sha256 != sha256_header {
            anyhow::bail!(
                "SHA-256 mismatch for plugin '{}': expected {}, got {}",
                slug,
                sha256_header,
                computed_sha256
            );
        }

        // Verify Ed25519 signature (skip if using zero placeholder key in dev)
        if ED25519_PUBLIC_KEY_HEX
            != "0000000000000000000000000000000000000000000000000000000000000000"
        {
            if sig_header.is_empty() {
                anyhow::bail!("Plugin '{}' has no Ed25519 signature", slug);
            }
            verify_ed25519(ED25519_PUBLIC_KEY_HEX, &bytes, &sig_header)
                .context(format!("Signature verification failed for plugin '{}'", slug))?;
        }

        // Atomic install
        std::fs::create_dir_all(&self.install_dir)
            .context("Failed to create plugin install directory")?;

        let bin_name = if cfg!(windows) {
            format!("raps-{}.exe", slug)
        } else {
            format!("raps-{}", slug)
        };
        let final_path = self.install_dir.join(&bin_name);
        let tmp_path = self.install_dir.join(format!(".raps-{}.tmp", slug));

        std::fs::write(&tmp_path, &bytes).context("Failed to write plugin binary")?;

        // chmod 755 on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
                .context("Failed to set executable permissions")?;
        }

        std::fs::rename(&tmp_path, &final_path)
            .context("Failed to move plugin binary to install location")?;

        let installation = Installation {
            slug: slug.to_string(),
            version: version.clone(),
            platform: platform.to_string(),
            sha256: computed_sha256.clone(),
            signature: sig_header.clone(),
            install_path: final_path.to_string_lossy().to_string(),
        };

        // Update plugins.json
        self.update_plugin_registry(&installation)?;

        Ok(installation)
    }

    /// Uninstall a marketplace plugin.
    pub fn uninstall(&self, slug: &str) -> Result<()> {
        let bin_name = if cfg!(windows) {
            format!("raps-{}.exe", slug)
        } else {
            format!("raps-{}", slug)
        };
        let path = self.install_dir.join(&bin_name);

        if path.exists() {
            std::fs::remove_file(&path).context("Failed to remove plugin binary")?;
        }

        // Remove from plugins.json
        let mut config = PluginConfig::load().unwrap_or_default();
        config.plugins.remove(slug);
        config.save()?;

        Ok(())
    }

    /// Update a plugin with rollback on failure.
    pub async fn update_with_rollback(&self, slug: &str) -> Result<Installation> {
        let bin_name = if cfg!(windows) {
            format!("raps-{}.exe", slug)
        } else {
            format!("raps-{}", slug)
        };
        let final_path = self.install_dir.join(&bin_name);
        let backup_path = self.install_dir.join(format!(".raps-{}.bak", slug));

        // Backup existing binary if present
        let has_backup = if final_path.exists() {
            std::fs::copy(&final_path, &backup_path).ok();
            true
        } else {
            false
        };

        match self.install(slug).await {
            Ok(installation) => {
                // Clean up backup on success
                if has_backup {
                    let _ = std::fs::remove_file(&backup_path);
                }
                Ok(installation)
            }
            Err(e) => {
                // Restore backup on failure
                if has_backup {
                    let _ = std::fs::rename(&backup_path, &final_path);
                }
                Err(e)
            }
        }
    }

    /// Load installed marketplace plugins from the plugin registry.
    pub fn load_registry(&self) -> Vec<String> {
        let config = PluginConfig::load().unwrap_or_default();
        config
            .plugins
            .iter()
            .filter_map(|(name, entry)| {
                // Marketplace plugins have a signature set by the marketplace
                if entry.signature.is_some() {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if the current raps version is compatible with a plugin version.
    /// Currently always returns true — semver compatibility check placeholder.
    #[allow(dead_code)]
    pub fn check_raps_compatibility(&self, _plugin_version: &str) -> bool {
        true
    }

    fn update_plugin_registry(&self, installation: &Installation) -> Result<()> {
        let mut config = PluginConfig::load().unwrap_or_default();
        config.plugins.insert(
            installation.slug.clone(),
            PluginEntry {
                enabled: true,
                path: Some(installation.install_path.clone()),
                description: None,
                sha256: Some(installation.sha256.clone()),
                public_key: Some(ED25519_PUBLIC_KEY_HEX.to_string()),
                signature: Some(installation.signature.clone()),
                trusted: true,
            },
        );
        config.save()?;
        Ok(())
    }
}

impl Default for PluginInstaller {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_platform_returns_known_value() {
        let platform = detect_platform();
        // On CI (linux-x64), this should be linux-x64
        // Just verify it returns a non-empty, known-format value
        assert!(!platform.is_empty());
        let valid = [
            "linux-x64",
            "darwin-arm64",
            "darwin-x64",
            "win-x64",
            "unknown",
        ];
        assert!(valid.contains(&platform), "Unknown platform: {}", platform);
    }

    #[test]
    fn check_raps_compatibility_always_true() {
        let installer = PluginInstaller::new();
        assert!(installer.check_raps_compatibility("1.0.0"));
        assert!(installer.check_raps_compatibility("99.0.0"));
    }
}
