// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Plugin System
//!
//! Extend RAPS with external commands, hooks, and aliases.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Plugin {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
    /// Description
    pub description: Option<String>,
    /// Path to executable
    pub path: PathBuf,
    /// Commands provided
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Command alias
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Alias {
    /// Alias name
    pub name: String,
    /// Command to expand to
    pub command: String,
}

/// Plugin manager
pub struct PluginManager {
    /// Discovered plugins
    plugins: Vec<Plugin>,
    /// Defined aliases
    aliases: HashMap<String, String>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            aliases: HashMap::new(),
        }
    }

    /// Discover plugins in the plugin directory
    pub fn discover(&mut self) -> anyhow::Result<()> {
        // TODO: Implement plugin discovery
        // Look in ~/.raps/plugins/ for executable files
        // Parse plugin manifests
        Ok(())
    }

    /// List discovered plugins
    pub fn list_plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Add an alias
    pub fn add_alias(&mut self, name: String, command: String) {
        self.aliases.insert(name, command);
    }

    /// Remove an alias
    pub fn remove_alias(&mut self, name: &str) -> bool {
        self.aliases.remove(name).is_some()
    }

    /// List aliases
    pub fn list_aliases(&self) -> Vec<Alias> {
        self.aliases
            .iter()
            .map(|(name, command)| Alias {
                name: name.clone(),
                command: command.clone(),
            })
            .collect()
    }

    /// Resolve an alias
    pub fn resolve_alias(&self, name: &str) -> Option<&str> {
        self.aliases.get(name).map(|s| s.as_str())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
