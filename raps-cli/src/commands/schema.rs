// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! JSON Schema generation for CLI output types
//!
//! Generates JSON Schema definitions for all structured output types,
//! enabling automation consumers to validate CLI output programmatically.

use anyhow::Result;
use clap::Subcommand;
use schemars::schema_for;

#[derive(Debug, Subcommand)]
pub enum SchemaCommands {
    /// List all available schema types
    List,

    /// Generate JSON Schema for a specific output type
    Generate {
        /// Output type name (use 'schema list' to see available types)
        name: String,
    },

    /// Generate all schemas as a single JSON object
    All,
}

impl SchemaCommands {
    pub fn execute(self) -> Result<()> {
        match self {
            SchemaCommands::List => list_schemas(),
            SchemaCommands::Generate { name } => generate_schema(&name),
            SchemaCommands::All => generate_all(),
        }
    }
}

/// Registry of all output types with their schema generators
struct SchemaEntry {
    name: &'static str,
    category: &'static str,
    description: &'static str,
    generate: fn() -> schemars::Schema,
}

macro_rules! schema_entry {
    ($name:expr, $cat:expr, $desc:expr, $ty:ty) => {
        SchemaEntry {
            name: $name,
            category: $cat,
            description: $desc,
            generate: || schema_for!($ty),
        }
    };
}

fn schema_registry() -> Vec<SchemaEntry> {
    use super::auth::{InspectOutput, TestAuthOutput, WhoamiOutput};
    use super::bucket::{BucketInfoOutput, BucketOutput};
    use super::folder::FolderItemOutput;
    use super::hub::HubListOutput;
    use super::item::ItemInfoOutput;
    use super::object::download::{
        DeleteObjectOutput, DownloadOutput, ObjectInfoOutput, ObjectListOutput, SignedUrlOutput,
    };
    use super::object::upload::{BatchUploadResult, UploadOutput};
    use super::project::ProjectListOutput;

    vec![
        // Auth
        schema_entry!("auth.test", "auth", "Auth test result", TestAuthOutput),
        schema_entry!("auth.whoami", "auth", "Current user info", WhoamiOutput),
        schema_entry!(
            "auth.inspect",
            "auth",
            "Token inspection result",
            InspectOutput
        ),
        // Buckets
        schema_entry!(
            "bucket.list",
            "bucket",
            "Bucket list item",
            Vec<BucketOutput>
        ),
        schema_entry!("bucket.info", "bucket", "Bucket details", BucketInfoOutput),
        // Objects
        schema_entry!("object.upload", "object", "Upload result", UploadOutput),
        schema_entry!(
            "object.upload-batch",
            "object",
            "Batch upload result",
            BatchUploadResult
        ),
        schema_entry!(
            "object.download",
            "object",
            "Download result",
            DownloadOutput
        ),
        schema_entry!(
            "object.list",
            "object",
            "Object list",
            Vec<ObjectListOutput>
        ),
        schema_entry!(
            "object.delete",
            "object",
            "Delete result",
            DeleteObjectOutput
        ),
        schema_entry!(
            "object.signed-url",
            "object",
            "Signed URL result",
            SignedUrlOutput
        ),
        schema_entry!("object.info", "object", "Object details", ObjectInfoOutput),
        // Data Management
        schema_entry!(
            "hub.list",
            "dm",
            "Hub list item",
            Vec<HubListOutput>
        ),
        schema_entry!(
            "project.list",
            "dm",
            "Project list item",
            Vec<ProjectListOutput>
        ),
        schema_entry!(
            "folder.list",
            "dm",
            "Folder list item",
            Vec<FolderItemOutput>
        ),
        schema_entry!("item.info", "dm", "Item details", ItemInfoOutput),
    ]
}

fn list_schemas() -> Result<()> {
    let registry = schema_registry();
    let mut current_category = "";

    println!("Available output schemas:\n");

    for entry in &registry {
        if entry.category != current_category {
            if !current_category.is_empty() {
                println!();
            }
            current_category = entry.category;
        }
        println!("  {:<30} {}", entry.name, entry.description);
    }

    println!("\nUsage: raps schema generate <name>");
    println!("       raps schema all");
    Ok(())
}

fn generate_schema(name: &str) -> Result<()> {
    let registry = schema_registry();

    let entry = registry.iter().find(|e| e.name == name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown schema '{}'. Use 'raps schema list' to see available types.",
            name
        )
    })?;

    let schema = (entry.generate)();
    let json = serde_json::to_string_pretty(&schema)?;
    println!("{}", json);
    Ok(())
}

fn generate_all() -> Result<()> {
    let registry = schema_registry();
    let mut all = serde_json::Map::new();

    for entry in &registry {
        let schema = (entry.generate)();
        let value = serde_json::to_value(&schema)?;
        all.insert(entry.name.to_string(), value);
    }

    let json = serde_json::to_string_pretty(&serde_json::Value::Object(all))?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_registry_not_empty() {
        let registry = schema_registry();
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_schema_registry_no_duplicate_names() {
        let registry = schema_registry();
        let mut seen = std::collections::HashSet::new();
        for entry in &registry {
            assert!(
                seen.insert(entry.name),
                "duplicate schema name: {}",
                entry.name
            );
        }
    }

    #[test]
    fn test_schema_registry_all_have_category() {
        let registry = schema_registry();
        for entry in &registry {
            assert!(
                !entry.category.is_empty(),
                "schema '{}' has empty category",
                entry.name
            );
        }
    }

    #[test]
    fn test_schema_registry_generators_work() {
        let registry = schema_registry();
        for entry in &registry {
            let schema = (entry.generate)();
            let json = serde_json::to_value(&schema);
            assert!(
                json.is_ok(),
                "schema '{}' failed to serialize: {:?}",
                entry.name,
                json.err()
            );
        }
    }

    #[test]
    fn test_schema_registry_covers_dm_types() {
        let registry = schema_registry();
        let names: Vec<_> = registry.iter().map(|e| e.name).collect();
        assert!(names.contains(&"hub.list"), "missing hub.list");
        assert!(names.contains(&"project.list"), "missing project.list");
        assert!(names.contains(&"folder.list"), "missing folder.list");
        assert!(names.contains(&"item.info"), "missing item.info");
    }
}
