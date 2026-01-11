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

use raps_kernel::logging;

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
                anyhow::bail!("Plugin '{name}' is disabled");
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

        anyhow::bail!("Plugin '{name}' not found")
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
                        logging::log_verbose(&format!(
                            "Hook '{}' failed with exit code {:?}",
                            hook_cmd,
                            s.code()
                        ));
                    }
                    Err(e) => {
                        logging::log_verbose(&format!(
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
            logging::log_verbose(&format!(
                "Warning: Hook uses absolute path: {}. Consider adding to allowed commands.",
                command
            ));
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
