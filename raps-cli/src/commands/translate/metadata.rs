// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Metadata handlers: model views, object tree, properties, and property queries.

use anyhow::{Context, Result};
use colored::Colorize;

use crate::output::OutputFormat;
use raps_derivative::{DerivativeClient, MdRegion, PropertyQuery};

fn parse_region(region: Option<String>) -> Result<Option<MdRegion>> {
    match region {
        Some(r) => Ok(Some(r.parse::<MdRegion>()?)),
        None => Ok(None),
    }
}

pub(super) async fn show_metadata(
    client: &DerivativeClient,
    urn: &str,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching model metadata...".dimmed());
    let response = client.get_metadata(urn, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Model Views / Viewables".bold());
            println!("{}", "\u{2500}".repeat(70));
            for view in &response.data.metadata {
                let role_icon = match view.role.as_str() {
                    "3d" => "\u{1F9CA}",
                    "2d" => "\u{1F4D0}",
                    _ => "\u{1F4C4}",
                };
                println!(
                    "  {} {} {}",
                    role_icon,
                    view.name.bold(),
                    format!("({})", view.role).dimmed()
                );
                println!("    {} {}", "GUID:".dimmed(), view.guid);
                if let Some(ref mime) = view.mime_type {
                    println!("    {} {}", "MIME:".dimmed(), mime);
                }
                if let Some(ref progress) = view.progress {
                    println!("    {} {}", "Progress:".dimmed(), progress);
                }
            }
            if response.data.metadata.is_empty() {
                println!("  {}", "No views found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

pub(super) async fn show_object_tree(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching object tree...".dimmed());
    let response = client.get_object_tree(urn, guid, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Object Tree".bold());
            println!("{}", "\u{2500}".repeat(70));
            for node in &response.data.objects {
                print_tree_node(node, 0);
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

fn print_tree_node(node: &raps_derivative::ObjectTreeNode, depth: usize) {
    let indent = "  ".repeat(depth + 1);
    let connector = if depth > 0 { "\u{251C}\u{2500} " } else { "" };
    println!(
        "{}{}[{}] {}",
        indent,
        connector,
        node.object_id.to_string().dimmed(),
        node.name
    );
    for child in &node.objects {
        print_tree_node(child, depth + 1);
    }
}

pub(super) async fn show_properties(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    object_id: Option<i64>,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    println!("{}", "Fetching properties...".dimmed());
    let response = client.get_properties(urn, guid, object_id, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Properties".bold());
            println!("{}", "\u{2500}".repeat(70));
            for obj in &response.data.collection {
                println!(
                    "\n  {} [{}]",
                    obj.name.bold(),
                    obj.object_id.to_string().dimmed()
                );
                for (category, props) in &obj.properties {
                    println!("    {}:", category.cyan());
                    if let Some(map) = props.as_object() {
                        for (key, value) in map {
                            println!("      {} {}", format!("{}:", key).dimmed(), value);
                        }
                    }
                }
            }
            if response.data.collection.is_empty() {
                println!("  {}", "No properties found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}

pub(super) async fn query_properties(
    client: &DerivativeClient,
    urn: &str,
    guid: &str,
    filter: &str,
    fields: Option<String>,
    region: Option<String>,
    output: &str,
) -> Result<()> {
    let region = parse_region(region)?;
    let output_format: OutputFormat = output.parse().unwrap_or(OutputFormat::Table);

    // Parse comma-separated object IDs
    let ids: Vec<i64> = filter
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<i64>()
                .context(format!("Invalid object ID: '{}'", s.trim()))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut query = PropertyQuery::by_object_ids(ids);
    if let Some(ref f) = fields {
        query.fields = Some(f.split(',').map(|s| s.trim().to_string()).collect());
    }

    println!("{}", "Querying properties...".dimmed());
    let response = client.query_properties(urn, guid, query, region).await?;

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Query Results".bold());
            println!("{}", "\u{2500}".repeat(70));
            for obj in &response.data.collection {
                println!(
                    "\n  {} [{}]",
                    obj.name.bold(),
                    obj.object_id.to_string().dimmed()
                );
                for (category, props) in &obj.properties {
                    println!("    {}:", category.cyan());
                    if let Some(map) = props.as_object() {
                        for (key, value) in map {
                            println!("      {} {}", format!("{}:", key).dimmed(), value);
                        }
                    }
                }
            }
            if response.data.collection.is_empty() {
                println!("  {}", "No matching properties found.".dimmed());
            }
        }
        _ => {
            output_format.write(&response)?;
        }
    }

    Ok(())
}
