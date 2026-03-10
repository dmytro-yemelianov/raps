// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP tool handlers for skill management.

use super::server::RapsServer;
use crate::skill::installer;
use crate::skill::registry::BundledRegistry;

impl RapsServer {
    pub(crate) async fn skill_list(&self, filter: Option<String>) -> String {
        let registry = BundledRegistry::load();
        let installed = installer::list_installed();

        let mut lines: Vec<String> = Vec::new();

        // Bundled skills (with install status)
        for s in &registry.skills {
            let is_installed = installed.contains(&s.name);
            let status = if is_installed { "installed" } else { "available" };

            match filter.as_deref() {
                Some("installed") if !is_installed => continue,
                Some("available") if is_installed => continue,
                _ => {}
            }

            lines.push(format!(
                "  {} (v{}) [{}] — {}",
                s.name, s.version, status, s.description
            ));
        }

        // Installed-but-not-bundled skills (custom/community)
        if filter.as_deref() != Some("available") {
            for name in &installed {
                if registry.get(name).is_none() {
                    lines.push(format!("  {} [installed, custom]", name));
                }
            }
        }

        if lines.is_empty() {
            return "No skills found matching the filter.".to_string();
        }

        format!(
            "Skills ({}):\n\n{}\n\nInstall with: skill_install(name: \"<skill-name>\")",
            lines.len(),
            lines.join("\n")
        )
    }

    pub(crate) async fn skill_install(&self, name: String, force: bool) -> String {
        match installer::install_skill(&name, force) {
            Ok(msg) => msg,
            Err(msg) => format!("Error: {}", msg),
        }
    }

    pub(crate) async fn skill_info(&self, name: String) -> String {
        let registry = BundledRegistry::load();
        let installed = installer::list_installed();

        match registry.get(&name) {
            Some(entry) => {
                let is_installed = installed.contains(&entry.name);
                let status = if is_installed { "installed" } else { "available" };
                let install_path = if is_installed {
                    installer::skills_dir()
                        .join(&entry.name)
                        .join("SKILL.md")
                        .to_string_lossy()
                        .to_string()
                } else {
                    "not installed".to_string()
                };

                let mut output = format!(
                    "Skill: {}\nVersion: {}\nStatus: {}\nSource: bundled\nPath: {}\nDescription: {}\n",
                    entry.name, entry.version, status, install_path, entry.description
                );

                if let Some(content) = registry.get_content(&name) {
                    output.push_str("\nContent preview:\n");
                    for line in content.lines().take(20) {
                        output.push_str(line);
                        output.push('\n');
                    }
                    output.push_str("...\n");
                }

                output
            }
            None => {
                // Check if it's an installed custom skill
                if installed.contains(&name) {
                    let path = installer::skills_dir().join(&name).join("SKILL.md");
                    format!(
                        "Skill: {}\nStatus: installed (custom)\nSource: local\nPath: {}\n",
                        name,
                        path.display()
                    )
                } else {
                    format!(
                        "Unknown skill '{}'. Use skill_list to see available skills.",
                        name
                    )
                }
            }
        }
    }
}
