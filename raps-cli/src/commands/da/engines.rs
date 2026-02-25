// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Engine-related handlers for Design Automation.

use anyhow::Result;
use colored::Colorize;
use serde::Serialize;

use crate::output::OutputFormat;
use raps_da::DesignAutomationClient;

pub(super) async fn list_engines(
    client: &DesignAutomationClient,
    output_format: OutputFormat,
) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Fetching engines...".dimmed());
    }

    let engines = client.list_engines().await?;

    #[derive(Serialize)]
    struct EngineOutput {
        id: String,
        description: Option<String>,
        product_version: Option<String>,
    }

    let engine_outputs: Vec<EngineOutput> = engines
        .iter()
        .map(|e| EngineOutput {
            id: e.clone(),
            description: None,
            product_version: None,
        })
        .collect();

    if engine_outputs.is_empty() {
        match output_format {
            OutputFormat::Table => println!("{}", "No engines found.".yellow()),
            _ => {
                output_format.write(&Vec::<EngineOutput>::new())?;
            }
        }
        return Ok(());
    }

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Available Engines:".bold());
            println!("{}", "-".repeat(80));

            // Group by product
            let mut autocad_engines = Vec::new();
            let mut revit_engines = Vec::new();
            let mut inventor_engines = Vec::new();
            let mut max_engines = Vec::new();
            let mut other_engines = Vec::new();

            for engine in &engines {
                if engine.contains("AutoCAD") {
                    autocad_engines.push(engine);
                } else if engine.contains("Revit") {
                    revit_engines.push(engine);
                } else if engine.contains("Inventor") {
                    inventor_engines.push(engine);
                } else if engine.contains("3dsMax") {
                    max_engines.push(engine);
                } else {
                    other_engines.push(engine);
                }
            }

            if !autocad_engines.is_empty() {
                println!("\n{}", "AutoCAD:".cyan().bold());
                for engine in autocad_engines {
                    println!("  {} {}", "-".dimmed(), engine);
                }
            }

            if !revit_engines.is_empty() {
                println!("\n{}", "Revit:".cyan().bold());
                for engine in revit_engines {
                    println!("  {} {}", "-".dimmed(), engine);
                }
            }

            if !inventor_engines.is_empty() {
                println!("\n{}", "Inventor:".cyan().bold());
                for engine in inventor_engines {
                    println!("  {} {}", "-".dimmed(), engine);
                }
            }

            if !max_engines.is_empty() {
                println!("\n{}", "3ds Max:".cyan().bold());
                for engine in max_engines {
                    println!("  {} {}", "-".dimmed(), engine);
                }
            }

            if !other_engines.is_empty() {
                println!("\n{}", "Other:".cyan().bold());
                for engine in other_engines {
                    println!("  {} {}", "-".dimmed(), engine);
                }
            }

            println!("{}", "-".repeat(80));
        }
        _ => {
            output_format.write(&engine_outputs)?;
        }
    }
    Ok(())
}
