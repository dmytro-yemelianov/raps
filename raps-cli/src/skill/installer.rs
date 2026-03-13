// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Skill installer — install/uninstall bundled skills to ~/.claude/skills/.

use std::fs;
use std::path::{Path, PathBuf};

use super::registry::BundledRegistry;

/// Default skills installation directory.
pub fn skills_dir() -> PathBuf {
    let base = directories::BaseDirs::new().expect("could not determine home directory");
    base.home_dir().join(".claude").join("skills")
}

/// List skill names installed at the given path.
pub fn list_installed_at(skills_path: &Path) -> Vec<String> {
    let mut installed = Vec::new();
    if let Ok(entries) = fs::read_dir(skills_path) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                let skill_md = entry.path().join("SKILL.md");
                if skill_md.exists() {
                    if let Some(name) = entry.file_name().to_str() {
                        installed.push(name.to_string());
                    }
                }
            }
        }
    }
    installed.sort();
    installed
}

/// List skill names installed in the default directory.
pub fn list_installed() -> Vec<String> {
    list_installed_at(&skills_dir())
}

/// Install a bundled skill to a specific directory.
pub fn install_skill_to(name: &str, force: bool, skills_path: &Path) -> Result<String, String> {
    let registry = BundledRegistry::load();
    let entry = registry.get(name).ok_or_else(|| {
        format!(
            "Unknown skill '{}'. Run 'raps skill list' to see available skills.",
            name
        )
    })?;
    let content = registry.get_content(name).ok_or_else(|| {
        format!(
            "Skill '{}' is in registry but content is not bundled.",
            name
        )
    })?;

    let skill_dir = skills_path.join(name);
    let skill_file = skill_dir.join("SKILL.md");

    if skill_file.exists() && !force {
        return Ok(format!(
            "Skill '{}' is already installed at {}. Use --force to overwrite.",
            name,
            skill_file.display()
        ));
    }

    fs::create_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to create directory {}: {}", skill_dir.display(), e))?;
    fs::write(&skill_file, content)
        .map_err(|e| format!("Failed to write {}: {}", skill_file.display(), e))?;

    Ok(format!(
        "Installed skill '{}' v{} to {}",
        name,
        entry.version,
        skill_file.display()
    ))
}

/// Install a bundled skill to the default directory.
pub fn install_skill(name: &str, force: bool) -> Result<String, String> {
    install_skill_to(name, force, &skills_dir())
}

/// Uninstall a skill from a specific directory.
pub fn uninstall_skill_from(name: &str, skills_path: &Path) -> Result<String, String> {
    let skill_dir = skills_path.join(name);
    if !skill_dir.exists() {
        return Err(format!("Skill '{}' is not installed.", name));
    }
    fs::remove_dir_all(&skill_dir)
        .map_err(|e| format!("Failed to remove {}: {}", skill_dir.display(), e))?;
    Ok(format!(
        "Removed skill '{}' from {}",
        name,
        skills_path.display()
    ))
}

/// Uninstall a skill from the default directory.
pub fn uninstall_skill(name: &str) -> Result<String, String> {
    uninstall_skill_from(name, &skills_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_skills_dir_path() {
        let path = skills_dir();
        assert!(path.to_string_lossy().contains(".claude"));
        assert!(path.to_string_lossy().contains("skills"));
    }

    #[test]
    fn test_install_and_uninstall_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_path = tmp.path().join("skills");

        let result = install_skill_to("using-raps-mcp", false, &skills_path);
        assert!(result.is_ok(), "install should succeed: {:?}", result);

        let skill_file = skills_path.join("using-raps-mcp").join("SKILL.md");
        assert!(skill_file.exists(), "SKILL.md should be written");

        let content = fs::read_to_string(&skill_file).unwrap();
        assert!(content.contains("# Using RAPS MCP Tools"));

        // Install again without force should say already installed
        let result = install_skill_to("using-raps-mcp", false, &skills_path);
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert!(
            msg.contains("already installed"),
            "should report already installed: {msg}"
        );

        // Install with force should overwrite
        let result = install_skill_to("using-raps-mcp", true, &skills_path);
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert!(msg.contains("Installed") || msg.contains("installed"));

        // Uninstall
        let result = uninstall_skill_from("using-raps-mcp", &skills_path);
        assert!(result.is_ok());
        assert!(!skill_file.exists(), "SKILL.md should be removed");

        // Uninstall non-existent should error
        let result = uninstall_skill_from("nonexistent", &skills_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_unknown_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let result = install_skill_to("nonexistent-skill", false, &tmp.path().join("skills"));
        assert!(result.is_err());
    }

    #[test]
    fn test_list_installed_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let skills_path = tmp.path().join("skills");

        let installed = list_installed_at(&skills_path);
        assert!(installed.is_empty());

        install_skill_to("using-raps-mcp", false, &skills_path).unwrap();
        let installed = list_installed_at(&skills_path);
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0], "using-raps-mcp");
    }
}
