// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bundled skill registry — skills embedded in the binary at compile time.

use serde::Deserialize;

/// A single skill entry in the registry.
#[derive(Debug, Clone, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub version: String,
    pub description: String,
    pub path: String,
}

/// The bundled skill registry parsed from skills/registry.json.
#[derive(Debug, Deserialize)]
pub struct BundledRegistry {
    pub skills: Vec<SkillEntry>,
}

// Embed the registry and skill files at compile time
const REGISTRY_JSON: &str = include_str!("../../../skills/registry.json");
const SKILL_USING_RAPS_MCP: &str = include_str!("../../../skills/using-raps-mcp/SKILL.md");

impl BundledRegistry {
    /// Load the embedded registry.
    pub fn load() -> Self {
        serde_json::from_str(REGISTRY_JSON).expect("bundled registry.json must be valid")
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<&SkillEntry> {
        self.skills.iter().find(|s| s.name == name)
    }

    /// Get the raw SKILL.md content for a bundled skill.
    pub fn get_content(&self, name: &str) -> Option<&'static str> {
        match name {
            "using-raps-mcp" => Some(SKILL_USING_RAPS_MCP),
            _ => None,
        }
    }

    /// Search skills by keyword (case-insensitive match on name + description).
    pub fn search(&self, query: &str) -> Vec<&SkillEntry> {
        let query_lower = query.to_lowercase();
        self.skills
            .iter()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundled_registry_parses() {
        let registry = BundledRegistry::load();
        assert!(!registry.skills.is_empty(), "registry should have at least one skill");
    }

    #[test]
    fn test_bundled_registry_has_using_raps_mcp() {
        let registry = BundledRegistry::load();
        let skill = registry.get("using-raps-mcp");
        assert!(skill.is_some(), "using-raps-mcp should be in registry");
        let skill = skill.unwrap();
        assert_eq!(skill.name, "using-raps-mcp");
        assert_eq!(skill.version, "1.0");
        assert!(!skill.description.is_empty());
    }

    #[test]
    fn test_bundled_skill_content_exists() {
        let registry = BundledRegistry::load();
        let content = registry.get_content("using-raps-mcp");
        assert!(content.is_some(), "using-raps-mcp content should be embedded");
        let content = content.unwrap();
        assert!(content.contains("# Using RAPS MCP Tools"));
    }

    #[test]
    fn test_search_finds_by_name() {
        let registry = BundledRegistry::load();
        let results = registry.search("mcp");
        assert!(!results.is_empty());
        assert_eq!(results[0].name, "using-raps-mcp");
    }

    #[test]
    fn test_search_finds_by_description() {
        let registry = BundledRegistry::load();
        let results = registry.search("batch tool fetching");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_no_match() {
        let registry = BundledRegistry::load();
        let results = registry.search("xyznonexistent");
        assert!(results.is_empty());
    }
}
