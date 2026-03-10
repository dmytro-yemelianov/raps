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
const SKILL_ADDING_MCP_TOOL: &str = include_str!("../../../skills/adding-mcp-tool/SKILL.md");
const SKILL_ADDING_RAPS_CLI_COMMAND: &str =
    include_str!("../../../skills/adding-raps-cli-command/SKILL.md");
const SKILL_COMPOSING_RAPS_LINKEDIN_POSTS: &str =
    include_str!("../../../skills/composing-raps-linkedin-posts/SKILL.md");
const SKILL_CUTTING_RAPS_RELEASE: &str =
    include_str!("../../../skills/cutting-raps-release/SKILL.md");
const SKILL_RAPS_CI_CD_TEMPLATES: &str =
    include_str!("../../../skills/raps-ci-cd-templates/SKILL.md");
const SKILL_RAPS_SECURITY_AUDIT: &str =
    include_str!("../../../skills/raps-security-audit/SKILL.md");
const SKILL_SHORTEN_URL: &str = include_str!("../../../skills/shorten-url/SKILL.md");
const SKILL_UPDATING_DEVCON_MATERIALS: &str =
    include_str!("../../../skills/updating-devcon-materials/SKILL.md");
const SKILL_UPDATING_RAPS_MARKETING: &str =
    include_str!("../../../skills/updating-raps-marketing/SKILL.md");
const SKILL_UPDATING_RAPS_WEBSITE: &str =
    include_str!("../../../skills/updating-raps-website/SKILL.md");
const SKILL_VERIFYING_RAPS_RELEASE: &str =
    include_str!("../../../skills/verifying-raps-release/SKILL.md");
const SKILL_WRITING_COOKBOOK_RECIPE: &str =
    include_str!("../../../skills/writing-cookbook-recipe/SKILL.md");
const SKILL_WRITING_RAPS_BLOG_POST: &str =
    include_str!("../../../skills/writing-raps-blog-post/SKILL.md");

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
            "adding-mcp-tool" => Some(SKILL_ADDING_MCP_TOOL),
            "adding-raps-cli-command" => Some(SKILL_ADDING_RAPS_CLI_COMMAND),
            "composing-raps-linkedin-posts" => Some(SKILL_COMPOSING_RAPS_LINKEDIN_POSTS),
            "cutting-raps-release" => Some(SKILL_CUTTING_RAPS_RELEASE),
            "raps-ci-cd-templates" => Some(SKILL_RAPS_CI_CD_TEMPLATES),
            "raps-security-audit" => Some(SKILL_RAPS_SECURITY_AUDIT),
            "shorten-url" => Some(SKILL_SHORTEN_URL),
            "updating-devcon-materials" => Some(SKILL_UPDATING_DEVCON_MATERIALS),
            "updating-raps-marketing" => Some(SKILL_UPDATING_RAPS_MARKETING),
            "updating-raps-website" => Some(SKILL_UPDATING_RAPS_WEBSITE),
            "verifying-raps-release" => Some(SKILL_VERIFYING_RAPS_RELEASE),
            "writing-cookbook-recipe" => Some(SKILL_WRITING_COOKBOOK_RECIPE),
            "writing-raps-blog-post" => Some(SKILL_WRITING_RAPS_BLOG_POST),
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
        assert_eq!(registry.skills.len(), 14, "registry should have 14 skills");
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
    fn test_all_bundled_skills_have_content() {
        let registry = BundledRegistry::load();
        for skill in &registry.skills {
            let content = registry.get_content(&skill.name);
            assert!(
                content.is_some(),
                "skill '{}' should have embedded content",
                skill.name
            );
            assert!(
                content.unwrap().contains(&format!("name: {}", skill.name)),
                "skill '{}' content should contain its name in frontmatter",
                skill.name
            );
        }
    }

    #[test]
    fn test_search_finds_by_name() {
        let registry = BundledRegistry::load();
        let results = registry.search("mcp");
        assert!(results.len() >= 2); // using-raps-mcp + adding-mcp-tool
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
