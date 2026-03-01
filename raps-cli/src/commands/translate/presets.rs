// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Translation preset management: list, show, create, delete, and use presets.

use std::str::FromStr;

use anyhow::{Context, Result};
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_derivative::{DerivativeClient, OutputFormat as DerivativeOutputFormat};

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct TranslationPreset {
    name: String,
    format: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, serde::Deserialize, Default)]
struct PresetStore {
    presets: Vec<TranslationPreset>,
}

impl PresetStore {
    fn file_path() -> Result<std::path::PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
            .context("Failed to get project directories")?;
        let config_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(config_dir)?;
        Ok(config_dir.join("presets.json"))
    }

    fn load() -> Result<Self> {
        let path = Self::file_path()?;
        if !path.exists() {
            return Ok(Self::default_presets());
        }
        let content = std::fs::read_to_string(&path)?;
        let store: Self = serde_json::from_str(&content)?;
        Ok(store)
    }

    fn save(&self) -> Result<()> {
        let path = Self::file_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    fn default_presets() -> Self {
        Self {
            presets: vec![
                TranslationPreset {
                    name: "viewer".to_string(),
                    format: "svf2".to_string(),
                    description: Some("Optimized for web viewer (SVF2)".to_string()),
                },
                TranslationPreset {
                    name: "export-3d".to_string(),
                    format: "obj".to_string(),
                    description: Some("Export as OBJ mesh for external tools".to_string()),
                },
                TranslationPreset {
                    name: "3d-print".to_string(),
                    format: "stl".to_string(),
                    description: Some("Export for 3D printing (STL)".to_string()),
                },
                TranslationPreset {
                    name: "cad-exchange".to_string(),
                    format: "step".to_string(),
                    description: Some("CAD interchange format (STEP)".to_string()),
                },
                TranslationPreset {
                    name: "bim".to_string(),
                    format: "ifc".to_string(),
                    description: Some("BIM format (IFC)".to_string()),
                },
            ],
        }
    }
}

pub(super) fn list_presets(output_format: OutputFormat) -> Result<()> {
    let store = PresetStore::load()?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Translation Presets:".bold());
            println!("{}", "-".repeat(70));
            println!(
                "{:<20} {:<12} {}",
                "Name".bold(),
                "Format".bold(),
                "Description".bold()
            );
            println!("{}", "-".repeat(70));

            for preset in &store.presets {
                let desc = preset.description.as_deref().unwrap_or("-");
                println!(
                    "{:<20} {:<12} {}",
                    preset.name.cyan(),
                    preset.format,
                    desc.dimmed()
                );
            }

            println!("{}", "-".repeat(70));
            println!(
                "\n{}",
                "Use 'raps translate preset use <urn> <preset>' to translate".dimmed()
            );
        }
        _ => {
            output_format.write(&store.presets)?;
        }
    }

    Ok(())
}

pub(super) fn show_preset(name: &str, output_format: OutputFormat) -> Result<()> {
    let store = PresetStore::load()?;

    let preset = store
        .presets
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", name))?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Preset Details:".bold());
            println!("{}", "-".repeat(50));
            println!("  {} {}", "Name:".bold(), preset.name.cyan());
            println!("  {} {}", "Format:".bold(), preset.format);
            if let Some(ref desc) = preset.description {
                println!("  {} {}", "Description:".bold(), desc);
            }
            println!("{}", "-".repeat(50));
        }
        _ => {
            output_format.write(preset)?;
        }
    }

    Ok(())
}

pub(super) fn create_preset(
    name: &str,
    format: &str,
    description: Option<String>,
    output_format: OutputFormat,
) -> Result<()> {
    // Validate format
    if DerivativeOutputFormat::from_str(format).is_err() {
        anyhow::bail!(
            "Invalid format '{}'. Valid formats: svf2, svf, obj, stl, step, iges, ifc, thumbnail",
            format
        );
    }

    let mut store = PresetStore::load()?;

    // Check for duplicate
    if store
        .presets
        .iter()
        .any(|p| p.name.eq_ignore_ascii_case(name))
    {
        anyhow::bail!("Preset '{name}' already exists");
    }

    let preset = TranslationPreset {
        name: name.to_string(),
        format: format.to_string(),
        description,
    };

    store.presets.push(preset.clone());
    store.save()?;

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Preset '{}' created!",
                "\u{2713}".green().bold(),
                name.cyan()
            );
        }
        _ => {
            output_format.write(&preset)?;
        }
    }

    Ok(())
}

pub(super) fn delete_preset(name: &str, output_format: OutputFormat) -> Result<()> {
    let mut store = PresetStore::load()?;

    let initial_len = store.presets.len();
    store.presets.retain(|p| !p.name.eq_ignore_ascii_case(name));

    if store.presets.len() == initial_len {
        anyhow::bail!("Preset '{name}' not found");
    }

    store.save()?;

    #[derive(Serialize)]
    struct DeleteOutput {
        success: bool,
        name: String,
    }

    let output = DeleteOutput {
        success: true,
        name: name.to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} Preset '{}' deleted!", "\u{2713}".green().bold(), name);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

pub(super) async fn use_preset(
    client: &DerivativeClient,
    urn: &str,
    preset_name: &str,
    output_format: OutputFormat,
) -> Result<()> {
    let store = PresetStore::load()?;

    let preset = store
        .presets
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(preset_name))
        .ok_or_else(|| anyhow::anyhow!("Preset '{}' not found", preset_name))?;

    let format = DerivativeOutputFormat::from_str(&preset.format)
        .map_err(|_| anyhow::anyhow!("Invalid format in preset: {}", preset.format))?;

    if output_format.supports_colors() {
        println!(
            "{} Using preset: {} ({})",
            "->".cyan(),
            preset.name.bold(),
            preset.format
        );
    }

    // Start translation using the preset format
    let response = client
        .translate(
            urn,
            format,
            None,
            raps_derivative::MdRegion::default(),
            false,
        )
        .await?;

    #[derive(Serialize)]
    struct UsePresetOutput {
        success: bool,
        preset: String,
        format: String,
        urn: String,
        result: String,
    }

    let output = UsePresetOutput {
        success: response.result == "created" || response.result == "success",
        preset: preset.name.clone(),
        format: preset.format.clone(),
        urn: response.urn.clone(),
        result: response.result.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!(
                "{} Translation started with preset '{}'!",
                "\u{2713}".green().bold(),
                preset.name
            );
            println!("  {} {}", "Format:".bold(), output.format.cyan());
            println!("  {} {}", "URN:".bold(), output.urn.dimmed());
            println!(
                "\n{}",
                "Use 'raps translate status <urn> --wait' to monitor progress".dimmed()
            );
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}
