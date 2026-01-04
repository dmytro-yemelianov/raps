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
use serde::{Deserialize, Serialize};
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
        let mut plugins = Vec::new();

        // Get PATH environment variable
        if let Ok(path_var) = std::env::var("PATH") {
            let paths: Vec<&str> = if cfg!(windows) {
                path_var.split(';').collect()
            } else {
                path_var.split(':').collect()
            };

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
        }

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
                anyhow::bail!("Plugin '{}' is disabled", name);
            }
            if let Some(ref path) = entry.path {
                return self.run_plugin(path, args);
            }
        }

        // Try to find in discovered plugins
        let discovered = self.discover_plugins();
        if let Some(plugin) = discovered.iter().find(|p| p.name == name) {
            return self.run_plugin(&plugin.path.to_string_lossy(), args);
        }

        anyhow::bail!("Plugin '{}' not found", name)
    }

    /// Run a plugin executable
    fn run_plugin(&self, path: &str, args: &[&str]) -> Result<i32> {
        let output = Command::new(path)
            .args(args)
            .status()
            .with_context(|| format!("Failed to execute plugin: {}", path))?;

        Ok(output.code().unwrap_or(-1))
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
                let status = if cfg!(windows) {
                    Command::new("cmd").args(["/C", hook_cmd]).status()
                } else {
                    Command::new("sh").args(["-c", hook_cmd]).status()
                };

                match status {
                    Ok(s) if !s.success() => {
                        crate::logging::log_verbose(&format!(
                            "Hook '{}' failed with exit code {:?}",
                            hook_cmd,
                            s.code()
                        ));
                    }
                    Err(e) => {
                        crate::logging::log_verbose(&format!(
                            "Hook '{}' failed to execute: {}",
                            hook_cmd, e
                        ));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
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
        let manager = PluginManager::default();
        // Should not panic
        assert!(manager.config.plugins.is_empty());
    }
}
