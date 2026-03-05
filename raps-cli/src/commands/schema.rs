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
    use super::acc::{AssetOutput, ChecklistOutput, SubmittalOutput};
    use super::admin::{
        AdminProjectListOutput, BulkResultOutput, CompanyListOutput, CsvUpdateResultOutput,
        OperationListOutput, OperationStatusOutput, UserListOutput,
    };
    use super::auth::{InspectOutput, TestAuthOutput, WhoamiOutput};
    use super::bucket::{BucketInfoOutput, BucketOutput};
    use super::da::{AppbundleUploadOutput, CreateActivityOutput, EngineOutput, WorkitemOutput};
    use super::folder::FolderItemOutput;
    use super::hub::HubListOutput;
    use super::issue::IssueOutput;
    use super::item::ItemInfoOutput;
    use super::object::download::{
        DeleteObjectOutput, DownloadOutput, ObjectInfoOutput, ObjectListOutput, SignedUrlOutput,
    };
    use super::object::upload::{BatchUploadResult, UploadOutput};
    use super::project::ProjectListOutput;
    use super::reality::{CreatePhotosceneOutput, PhotosceneOutput};
    use super::rfi::RfiOutput;
    use super::webhook::{CreateWebhookOutput, GetWebhookOutput, WebhookListOutput};

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
        // ACC — Issues
        schema_entry!(
            "issue.list",
            "acc",
            "Issue list item",
            Vec<IssueOutput>
        ),
        schema_entry!("issue.get", "acc", "Issue details", IssueOutput),
        // ACC — RFIs
        schema_entry!(
            "rfi.list",
            "acc",
            "RFI list item",
            Vec<RfiOutput>
        ),
        schema_entry!("rfi.get", "acc", "RFI details", RfiOutput),
        // ACC — Assets
        schema_entry!(
            "asset.list",
            "acc",
            "Asset list item",
            Vec<AssetOutput>
        ),
        // ACC — Submittals
        schema_entry!(
            "submittal.list",
            "acc",
            "Submittal list item",
            Vec<SubmittalOutput>
        ),
        // ACC — Checklists
        schema_entry!(
            "checklist.list",
            "acc",
            "Checklist list item",
            Vec<ChecklistOutput>
        ),
        // Admin — Users
        schema_entry!(
            "admin.user-list",
            "admin",
            "User list item",
            Vec<UserListOutput>
        ),
        // Admin — Projects
        schema_entry!(
            "admin.project-list",
            "admin",
            "Admin project list item",
            Vec<AdminProjectListOutput>
        ),
        schema_entry!(
            "admin.company-list",
            "admin",
            "Company list item",
            Vec<CompanyListOutput>
        ),
        // Admin — Operations
        schema_entry!(
            "admin.operation-status",
            "admin",
            "Bulk operation status",
            OperationStatusOutput
        ),
        schema_entry!(
            "admin.operation-list",
            "admin",
            "Bulk operation list item",
            Vec<OperationListOutput>
        ),
        schema_entry!(
            "admin.bulk-result",
            "admin",
            "Bulk operation result",
            BulkResultOutput
        ),
        // Admin — CSV ops
        schema_entry!(
            "admin.csv-update-result",
            "admin",
            "CSV bulk update result",
            CsvUpdateResultOutput
        ),
        // Design Automation — Engines
        schema_entry!(
            "da.engine-list",
            "da",
            "Engine list item",
            Vec<EngineOutput>
        ),
        // Design Automation — App Bundles
        schema_entry!(
            "da.appbundle-upload",
            "da",
            "App bundle upload result",
            AppbundleUploadOutput
        ),
        // Design Automation — Activities
        schema_entry!(
            "da.activity-create",
            "da",
            "Activity creation result",
            CreateActivityOutput
        ),
        // Design Automation — Work Items
        schema_entry!(
            "da.workitem-list",
            "da",
            "Work item list item",
            Vec<WorkitemOutput>
        ),
        // Webhooks
        schema_entry!(
            "webhook.list",
            "webhook",
            "Webhook list item",
            Vec<WebhookListOutput>
        ),
        schema_entry!(
            "webhook.create",
            "webhook",
            "Webhook creation result",
            CreateWebhookOutput
        ),
        schema_entry!(
            "webhook.get",
            "webhook",
            "Webhook details",
            GetWebhookOutput
        ),
        // Reality Capture
        schema_entry!(
            "reality.photoscene-list",
            "reality",
            "Photoscene list item",
            Vec<PhotosceneOutput>
        ),
        schema_entry!(
            "reality.photoscene-create",
            "reality",
            "Photoscene creation result",
            CreatePhotosceneOutput
        ),
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

    #[test]
    fn test_schema_registry_covers_acc_types() {
        let registry = schema_registry();
        let names: Vec<_> = registry.iter().map(|e| e.name).collect();
        assert!(names.contains(&"issue.list"), "missing issue.list");
        assert!(names.contains(&"rfi.list"), "missing rfi.list");
        assert!(names.contains(&"asset.list"), "missing asset.list");
    }

    #[test]
    fn test_schema_registry_covers_remaining_types() {
        let registry = schema_registry();
        let names: Vec<_> = registry.iter().map(|e| e.name).collect();
        // At least one entry per remaining category
        let has_admin = names.iter().any(|n| n.starts_with("admin."));
        let has_da = names.iter().any(|n| n.starts_with("da."));
        let has_webhook = names.iter().any(|n| n.starts_with("webhook."));
        let has_reality = names.iter().any(|n| n.starts_with("reality."));
        assert!(has_admin, "no admin entries in schema registry");
        assert!(has_da, "no DA entries in schema registry");
        assert!(has_webhook, "no webhook entries in schema registry");
        assert!(has_reality, "no reality entries in schema registry");
    }
}
