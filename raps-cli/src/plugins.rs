// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Plugin and Extension System
//!
//! Provides a mechanism for extending RAPS CLI with external commands and hooks.
//!
//! # Plugin Types
//!
//! 1. **External Command Plugins**: Executables named `raps-<name>` in PATH
//! 2. **Workflow Hooks**: Pre/post command hooks for automation
//! 3. **Custom Command Groups**: User-defined command aliases and groups
//!
//! # Plugin Discovery
//!
//! External plugins are discovered by searching PATH for executables matching:
//! - Windows: `raps-<name>.exe`
//! - Unix: `raps-<name>`
//!
//! # Configuration
//!
//! Plugins are configured in `~/.config/raps/plugins.json`:
//! ```json
//! {
//!   "plugins": {
//!     "my-plugin": {
//!       "enabled": true,
//!       "path": "/path/to/raps-my-plugin"
//!     }
//!   },
//!   "hooks": {
//!     "pre_upload": ["echo 'Starting upload'"],
//!     "post_translate": ["notify-send 'Translation complete'"]
//!   },
//!   "aliases": {
//!     "quick-upload": "object upload --resume"
//!   }
//! }
//! ```

use anyhow::{Context, Result};
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    /// Discovered and configured plugins
    #[serde(default)]
    pub plugins: HashMap<String, PluginEntry>,
    /// Workflow hooks
    #[serde(default)]
    pub hooks: HashMap<String, Vec<String>>,
    /// Command aliases
    #[serde(default)]
    pub aliases: HashMap<String, String>,
}

/// Individual plugin entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    /// Whether the plugin is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Path to the plugin executable (optional, auto-discovered if not set)
    pub path: Option<String>,
    /// Plugin description
    pub description: Option<String>,
    /// SHA-256 hash of the plugin binary (TOFU — trust on first use)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Ed25519 public key (hex-encoded) for signature verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Ed25519 signature (hex-encoded) of the plugin binary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Whether the user has explicitly trusted this plugin
    #[serde(default)]
    pub trusted: bool,
}

fn default_true() -> bool {
    true
}

/// Discovered plugin information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DiscoveredPlugin {
    pub name: String,
    pub path: PathBuf,
    pub enabled: bool,
}

#[allow(dead_code)]
impl PluginConfig {
    /// Load plugin configuration from file
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path).context("Failed to read plugin config")?;
        let config: Self =
            serde_json::from_str(&content).context("Failed to parse plugin config")?;
        Ok(config)
    }

    /// Save plugin configuration to file
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get the config file path
    fn config_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
            .context("Failed to get project directories")?;
        Ok(proj_dirs.config_dir().join("plugins.json"))
    }

    /// Get an alias command if defined
    pub fn get_alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(|s| s.as_str())
    }
}

/// Result of verifying a plugin's integrity
#[derive(Debug)]
pub struct PluginVerifyResult {
    pub name: String,
    pub path: PathBuf,
    pub current_hash: String,
    pub recorded_hash: Option<String>,
    pub hash_match: bool,
    pub has_signature: bool,
    pub signature_valid: bool,
    pub trusted: bool,
}

/// Compute SHA-256 hash of a file
pub fn compute_binary_hash(path: &Path) -> Result<String> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read plugin binary: {}", path.display()))?;
    let hash = Sha256::digest(&data);
    Ok(hex::encode(hash))
}

/// Verify ed25519 signature of a plugin binary
fn verify_ed25519_signature(path: &Path, sig_hex: &str, pubkey_hex: &str) -> Result<()> {
    let data = std::fs::read(path)
        .with_context(|| format!("Failed to read plugin binary: {}", path.display()))?;

    let pubkey_bytes = hex::decode(pubkey_hex).context("Invalid public key hex")?;
    let pubkey_array: [u8; 32] = pubkey_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Public key must be 32 bytes"))?;
    let verifying_key =
        VerifyingKey::from_bytes(&pubkey_array).context("Invalid ed25519 public key")?;

    let sig_bytes = hex::decode(sig_hex).context("Invalid signature hex")?;
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Signature must be 64 bytes"))?;
    let signature = Signature::from_bytes(&sig_array);

    verifying_key
        .verify_strict(&data, &signature)
        .context("Ed25519 signature verification failed")?;

    Ok(())
}

/// Plugin manager for discovering and executing plugins
#[allow(dead_code)]
pub struct PluginManager {
    config: PluginConfig,
}

#[allow(dead_code)]
impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Result<Self> {
        let config = PluginConfig::load().unwrap_or_default();
        Ok(Self { config })
    }

    /// Discover plugins in PATH
    pub fn discover_plugins(&self) -> Vec<DiscoveredPlugin> {
        let start = std::time::Instant::now();
        let mut plugins = Vec::new();

        // Get PATH environment variable
        let mut paths: Vec<String> = if let Ok(path_var) = std::env::var("PATH") {
            if cfg!(windows) {
                path_var.split(';').map(|s| s.to_string()).collect()
            } else {
                path_var.split(':').map(|s| s.to_string()).collect()
            }
        } else {
            Vec::new()
        };

        // Also check the directory where the current executable is located
        if let Ok(exe_path) = std::env::current_exe()
            && let Some(parent) = exe_path.parent()
        {
            paths.push(parent.to_string_lossy().to_string());
        }

        for dir in paths {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    if let Some(plugin) = self.check_plugin_entry(&entry.path()) {
                        // Avoid duplicates
                        if !plugins
                            .iter()
                            .any(|p: &DiscoveredPlugin| p.name == plugin.name)
                        {
                            plugins.push(plugin);
                        }
                    }
                }
            }
        }

        raps_kernel::profiler::mark_plugins_loaded(start.elapsed());
        plugins
    }

    /// Check if a path is a raps plugin
    fn check_plugin_entry(&self, path: &Path) -> Option<DiscoveredPlugin> {
        let file_name = path.file_name()?.to_str()?;

        // Check for raps-* pattern
        let plugin_name = if cfg!(windows) {
            if file_name.starts_with("raps-") && file_name.ends_with(".exe") {
                Some(file_name.strip_prefix("raps-")?.strip_suffix(".exe")?)
            } else {
                None
            }
        } else {
            file_name.strip_prefix("raps-")
        }?;

        // Check if enabled in config
        let enabled = self
            .config
            .plugins
            .get(plugin_name)
            .map(|e| e.enabled)
            .unwrap_or(true);

        Some(DiscoveredPlugin {
            name: plugin_name.to_string(),
            path: path.to_path_buf(),
            enabled,
        })
    }

    /// Execute a plugin by name
    pub fn execute_plugin(&self, name: &str, args: &[&str]) -> Result<i32> {
        // Check configured plugins first
        if let Some(entry) = self.config.plugins.get(name) {
            if !entry.enabled {
                anyhow::bail!("Plugin '{name}' is disabled");
            }
            if let Some(ref path) = entry.path {
                return self.run_plugin(path, name, args);
            }
        }

        // Try to find in discovered plugins
        let discovered = self.discover_plugins();
        if let Some(plugin) = discovered.iter().find(|p| p.name == name) {
            return self.run_plugin(&plugin.path.to_string_lossy(), name, args);
        }

        anyhow::bail!("Plugin '{name}' not found")
    }

    /// Run a plugin executable after verifying integrity
    fn run_plugin(&self, path: &str, name: &str, args: &[&str]) -> Result<i32> {
        self.verify_plugin_integrity(name, Path::new(path))?;

        let output = Command::new(path)
            .args(args)
            .status()
            .with_context(|| format!("Failed to execute plugin: {}", path))?;

        Ok(output.code().unwrap_or(-1))
    }

    /// Verify plugin integrity via TOFU hash and optional ed25519 signature.
    ///
    /// - First run: compute hash, record it, warn user
    /// - Subsequent runs: verify hash matches recorded value
    /// - If signature + public_key present: verify ed25519 signature
    fn verify_plugin_integrity(&self, name: &str, path: &Path) -> Result<()> {
        let current_hash = compute_binary_hash(path)?;

        if let Some(entry) = self.config.plugins.get(name) {
            // Check ed25519 signature if present
            if let (Some(sig_hex), Some(key_hex)) = (&entry.signature, &entry.public_key) {
                verify_ed25519_signature(path, sig_hex, key_hex).with_context(|| {
                    format!("Plugin '{name}' signature verification failed — binary may have been tampered with")
                })?;
                return Ok(());
            }

            // Check TOFU hash
            if let Some(ref recorded_hash) = entry.sha256 {
                if *recorded_hash != current_hash {
                    anyhow::bail!(
                        "Plugin '{name}' binary has changed since it was trusted!\n\
                         Recorded: {recorded_hash}\n\
                         Current:  {current_hash}\n\
                         Run `raps plugin trust {name}` to re-trust the new version."
                    );
                }
                return Ok(());
            }
        }

        // First execution — record hash (TOFU)
        tracing::warn!(
            plugin = name,
            hash = %current_hash,
            "First execution of plugin '{}' — recording SHA-256 hash (TOFU)",
            name
        );
        eprintln!(
            "Warning: First execution of plugin '{}'. SHA-256: {}",
            name, current_hash
        );

        let mut config = PluginConfig::load().unwrap_or_default();
        let entry = config.plugins.entry(name.to_string()).or_insert(PluginEntry {
            enabled: true,
            path: Some(path.to_string_lossy().to_string()),
            description: None,
            sha256: None,
            public_key: None,
            signature: None,
            trusted: false,
        });
        entry.sha256 = Some(current_hash);
        entry.trusted = true;
        let _ = config.save();

        Ok(())
    }

    /// Trust a plugin by recording its current hash
    pub fn trust_plugin(&self, name: &str) -> Result<String> {
        // Find plugin path
        let path = self.resolve_plugin_path(name)?;
        let hash = compute_binary_hash(&path)?;

        let mut config = PluginConfig::load().unwrap_or_default();
        let entry = config.plugins.entry(name.to_string()).or_insert(PluginEntry {
            enabled: true,
            path: Some(path.to_string_lossy().to_string()),
            description: None,
            sha256: None,
            public_key: None,
            signature: None,
            trusted: false,
        });
        entry.sha256 = Some(hash.clone());
        entry.trusted = true;
        config.save()?;

        Ok(hash)
    }

    /// Verify a plugin's integrity and return status
    pub fn verify_plugin(&self, name: &str) -> Result<PluginVerifyResult> {
        let path = self.resolve_plugin_path(name)?;
        let current_hash = compute_binary_hash(&path)?;

        if let Some(entry) = self.config.plugins.get(name) {
            let hash_match = entry.sha256.as_ref() == Some(&current_hash);

            // Check signature
            if let (Some(sig_hex), Some(key_hex)) = (&entry.signature, &entry.public_key) {
                let sig_ok = verify_ed25519_signature(&path, sig_hex, key_hex).is_ok();
                return Ok(PluginVerifyResult {
                    name: name.to_string(),
                    path,
                    current_hash,
                    recorded_hash: entry.sha256.clone(),
                    hash_match,
                    has_signature: true,
                    signature_valid: sig_ok,
                    trusted: entry.trusted,
                });
            }

            // Check TOFU hash
            return Ok(PluginVerifyResult {
                name: name.to_string(),
                path,
                current_hash,
                recorded_hash: entry.sha256.clone(),
                hash_match,
                has_signature: false,
                signature_valid: false,
                trusted: entry.trusted,
            });
        }

        Ok(PluginVerifyResult {
            name: name.to_string(),
            path,
            current_hash,
            recorded_hash: None,
            hash_match: false,
            has_signature: false,
            signature_valid: false,
            trusted: false,
        })
    }

    /// Resolve a plugin name to its filesystem path
    fn resolve_plugin_path(&self, name: &str) -> Result<PathBuf> {
        if let Some(entry) = self.config.plugins.get(name)
            && let Some(ref path) = entry.path
        {
            return Ok(PathBuf::from(path));
        }
        let discovered = self.discover_plugins();
        discovered
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.path.clone())
            .ok_or_else(|| anyhow::anyhow!("Plugin '{name}' not found"))
    }

    /// Run pre-command hooks
    pub fn run_pre_hooks(&self, command: &str) -> Result<()> {
        let hook_key = format!("pre_{}", command);
        self.run_hooks(&hook_key)
    }

    /// Run post-command hooks
    pub fn run_post_hooks(&self, command: &str) -> Result<()> {
        let hook_key = format!("post_{}", command);
        self.run_hooks(&hook_key)
    }

    /// Run hooks for a given key
    fn run_hooks(&self, key: &str) -> Result<()> {
        if let Some(hooks) = self.config.hooks.get(key) {
            for hook_cmd in hooks {
                // Parse the command to prevent shell injection
                let parsed = self.parse_hook_command(hook_cmd)?;
                if parsed.is_empty() {
                    continue;
                }

                let mut cmd = Command::new(&parsed[0]);
                if parsed.len() > 1 {
                    cmd.args(&parsed[1..]);
                }

                match cmd.status() {
                    Ok(s) if !s.success() => {
                        tracing::warn!("Hook '{}' failed with exit code {:?}", hook_cmd, s.code());
                    }
                    Err(e) => {
                        tracing::warn!("Hook '{}' failed to execute: {}", hook_cmd, e);
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Parse a hook command safely, preventing shell injection
    fn parse_hook_command(&self, cmd: &str) -> Result<Vec<String>> {
        // Simple command parsing without shell interpretation
        // This prevents command injection by not allowing shell metacharacters
        let mut args = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut escape_next = false;

        for ch in cmd.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
            } else if ch == '\\' && in_quotes {
                escape_next = true;
            } else if ch == '"' {
                in_quotes = !in_quotes;
            } else if ch.is_whitespace() && !in_quotes {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            args.push(current);
        }

        if in_quotes {
            anyhow::bail!("Unclosed quote in hook command: {cmd}");
        }

        // Validate that the command is allowed (whitelist approach)
        if !args.is_empty() {
            self.validate_hook_command(&args[0])?;
        }

        Ok(args)
    }

    /// Validate that a hook command is allowed
    fn validate_hook_command(&self, command: &str) -> Result<()> {
        // Define allowed commands - this should be configurable
        const ALLOWED_COMMANDS: &[&str] = &[
            "echo",
            "notify-send",
            "curl",
            "wget",
            "git",
            "npm",
            "cargo",
            "python",
            "node",
            "raps",
        ];

        // Check if command is in the allowed list or is a raps plugin
        let cmd_name = Path::new(command)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(command);

        if ALLOWED_COMMANDS.contains(&cmd_name) || cmd_name.starts_with("raps-") {
            Ok(())
        } else if command.contains('/') || command.contains('\\') {
            // Allow absolute paths but warn about them
            tracing::warn!(
                "Hook uses absolute path '{}'. Consider adding to the allowed commands list.",
                command
            );
            Ok(())
        } else {
            anyhow::bail!(
                "Command '{}' is not in the allowed list. Add it to ALLOWED_COMMANDS if needed.",
                command
            )
        }
    }

    /// List all discovered and configured plugins
    pub fn list_plugins(&self) -> Vec<DiscoveredPlugin> {
        let mut all_plugins = self.discover_plugins();

        // Add configured plugins that weren't discovered
        for (name, entry) in &self.config.plugins {
            if !all_plugins.iter().any(|p| &p.name == name)
                && let Some(ref path) = entry.path
            {
                all_plugins.push(DiscoveredPlugin {
                    name: name.clone(),
                    path: PathBuf::from(path),
                    enabled: entry.enabled,
                });
            }
        }

        all_plugins
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            config: PluginConfig::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_config_default() {
        let config = PluginConfig::default();
        assert!(config.plugins.is_empty());
        assert!(config.hooks.is_empty());
        assert!(config.aliases.is_empty());
    }

    #[test]
    fn test_plugin_entry_default_enabled() {
        let json = r#"{"path": "/usr/bin/raps-test"}"#;
        let entry: PluginEntry = serde_json::from_str(json).unwrap();
        assert!(entry.enabled); // default_true()
        assert!(entry.sha256.is_none());
        assert!(entry.public_key.is_none());
        assert!(entry.signature.is_none());
        assert!(!entry.trusted);
    }

    #[test]
    fn test_plugin_config_serialization() {
        let mut config = PluginConfig::default();
        config
            .aliases
            .insert("up".to_string(), "object upload".to_string());
        config.hooks.insert(
            "pre_upload".to_string(),
            vec!["echo 'starting'".to_string()],
        );

        let json = serde_json::to_string(&config).unwrap();
        let parsed: PluginConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.aliases.get("up"), Some(&"object upload".to_string()));
        assert_eq!(parsed.hooks.get("pre_upload").unwrap().len(), 1);
    }

    #[test]
    fn test_get_alias() {
        let mut config = PluginConfig::default();
        config
            .aliases
            .insert("quick-up".to_string(), "object upload --resume".to_string());

        assert_eq!(config.get_alias("quick-up"), Some("object upload --resume"));
        assert_eq!(config.get_alias("nonexistent"), None);
    }

    #[test]
    fn test_discovered_plugin_struct() {
        let plugin = DiscoveredPlugin {
            name: "test-plugin".to_string(),
            path: PathBuf::from("/usr/bin/raps-test-plugin"),
            enabled: true,
        };

        assert_eq!(plugin.name, "test-plugin");
        assert!(plugin.enabled);
    }

    #[test]
    fn test_plugin_manager_default() {
        let manager = PluginManager {
            config: PluginConfig::default(),
        };
        // Should not panic
        assert!(manager.config.plugins.is_empty());
    }

    #[test]
    fn test_parse_hook_command_basic() {
        let manager = PluginManager::default();

        // Test basic command parsing
        let result = manager.parse_hook_command("echo hello").unwrap();
        assert_eq!(result, vec!["echo", "hello"]);
    }

    #[test]
    fn test_parse_hook_command_with_quotes() {
        let manager = PluginManager::default();

        // Test quoted arguments
        let result = manager.parse_hook_command("echo \"hello world\"").unwrap();
        assert_eq!(result, vec!["echo", "hello world"]);

        // Test mixed quotes
        let result = manager
            .parse_hook_command("notify-send \"Build Complete\" success")
            .unwrap();
        assert_eq!(result, vec!["notify-send", "Build Complete", "success"]);
    }

    #[test]
    fn test_parse_hook_command_unclosed_quote() {
        let manager = PluginManager::default();

        // Test unclosed quote error
        let result = manager.parse_hook_command("echo \"unclosed quote");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unclosed quote"));
    }

    #[test]
    fn test_validate_hook_command_allowed() {
        let manager = PluginManager::default();

        // Test allowed commands
        assert!(manager.validate_hook_command("echo").is_ok());
        assert!(manager.validate_hook_command("curl").is_ok());
        assert!(manager.validate_hook_command("git").is_ok());
        assert!(manager.validate_hook_command("raps").is_ok());
        assert!(manager.validate_hook_command("raps-plugin").is_ok()); // raps- prefix
    }

    #[test]
    fn test_validate_hook_command_denied() {
        let manager = PluginManager::default();

        // Test denied commands
        let result = manager.validate_hook_command("rm");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("not in the allowed list")
        );

        let result = manager.validate_hook_command("sudo");
        assert!(result.is_err());

        let result = manager.validate_hook_command("sh");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_hook_command_absolute_path() {
        let manager = PluginManager::default();

        // Test absolute paths (should be allowed with warning)
        assert!(manager.validate_hook_command("/usr/bin/echo").is_ok());
        assert!(
            manager
                .validate_hook_command("C:\\Windows\\System32\\cmd.exe")
                .is_ok()
        );
    }

    #[test]
    fn test_parse_hook_command_empty() {
        let manager = PluginManager::default();

        // Test empty command
        let result = manager.parse_hook_command("").unwrap();
        assert!(result.is_empty());

        // Test whitespace only
        let result = manager.parse_hook_command("   ").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_binary_hash() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello world").unwrap();
        let hash = compute_binary_hash(tmp.path()).unwrap();
        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_binary_hash_changes_on_modification() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"version 1").unwrap();
        let hash1 = compute_binary_hash(tmp.path()).unwrap();

        std::fs::write(tmp.path(), b"version 2").unwrap();
        let hash2 = compute_binary_hash(tmp.path()).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_ed25519_signature_verification() {
        use ed25519_dalek::{Signer, SigningKey};
        use rand::rngs::OsRng;

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"plugin binary content").unwrap();

        // Generate a keypair
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        // Sign the binary
        let data = std::fs::read(tmp.path()).unwrap();
        let signature: ed25519_dalek::Signature = signing_key.sign(&data);

        let sig_hex = hex::encode(signature.to_bytes());
        let key_hex = hex::encode(verifying_key.to_bytes());

        // Verification should succeed
        assert!(verify_ed25519_signature(tmp.path(), &sig_hex, &key_hex).is_ok());

        // Tamper with the file
        std::fs::write(tmp.path(), b"tampered content").unwrap();
        assert!(verify_ed25519_signature(tmp.path(), &sig_hex, &key_hex).is_err());
    }

    #[test]
    fn test_ed25519_invalid_signature_fails() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"plugin binary").unwrap();

        // Valid key format but wrong signature
        let fake_key = "0".repeat(64); // 32 zero bytes in hex
        let fake_sig = "0".repeat(128); // 64 zero bytes in hex

        // Should fail verification (bad key/signature)
        assert!(verify_ed25519_signature(tmp.path(), &fake_sig, &fake_key).is_err());
    }

    #[test]
    fn test_plugin_entry_with_trust_fields() {
        let json = r#"{
            "enabled": true,
            "path": "/usr/bin/raps-test",
            "sha256": "abc123",
            "trusted": true,
            "public_key": "deadbeef",
            "signature": "cafebabe"
        }"#;
        let entry: PluginEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.sha256.as_deref(), Some("abc123"));
        assert!(entry.trusted);
        assert_eq!(entry.public_key.as_deref(), Some("deadbeef"));
        assert_eq!(entry.signature.as_deref(), Some("cafebabe"));
    }

    #[test]
    fn test_plugin_entry_trust_fields_optional() {
        // Old-format JSON without trust fields should still parse
        let json = r#"{"enabled": true}"#;
        let entry: PluginEntry = serde_json::from_str(json).unwrap();
        assert!(entry.sha256.is_none());
        assert!(!entry.trusted);
    }

    #[test]
    fn test_parse_hook_command_complex() {
        let manager = PluginManager::default();

        // Test complex command with multiple quoted sections
        let result = manager
            .parse_hook_command("raps object upload \"my file.txt\" --bucket \"test bucket\"")
            .unwrap();
        assert_eq!(
            result,
            vec![
                "raps",
                "object",
                "upload",
                "my file.txt",
                "--bucket",
                "test bucket"
            ]
        );
    }
}
