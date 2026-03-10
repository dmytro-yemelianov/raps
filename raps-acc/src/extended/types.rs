// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Type definitions for ACC Extended APIs (Assets, Submittals, Checklists, Project Admin)

use serde::{Deserialize, Serialize};

// ============================================================================
// ACC PROJECT ADMIN API (Project Creation)
// ============================================================================

/// Status of an ACC project creation job
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectCreationStatus {
    /// Job is pending
    Pending,
    /// Job is being processed
    Processing,
    /// Project created and active
    Active,
    /// Project creation failed
    Failed,
}

impl ProjectCreationStatus {
    /// Parse status from API response string
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => ProjectCreationStatus::Pending,
            "processing" => ProjectCreationStatus::Processing,
            "active" => ProjectCreationStatus::Active,
            "failed" | "error" => ProjectCreationStatus::Failed,
            _ => ProjectCreationStatus::Processing, // Default to processing for unknown states
        }
    }
}

/// Result of an ACC project creation operation
#[derive(Debug, Clone)]
pub struct ProjectCreationJob {
    /// The job ID returned by the API
    pub job_id: Option<String>,
    /// The created project ID (available after activation)
    pub project_id: Option<String>,
    /// Current status of the project creation
    pub status: ProjectCreationStatus,
    /// Project name
    pub name: Option<String>,
}

/// Request to create an ACC project (Construction Admin v1, camelCase)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// Project name
    pub name: String,
    /// Optional template project ID to clone from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_project_id: Option<String>,
    /// Products to enable. Omit to get all defaults (ACC enables all products).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<String>>,
    /// Project type (required: "ACC")
    #[serde(rename = "type")]
    pub project_type: Option<String>,
}

/// Request to create a BIM 360 project (HQ v1, snake_case)
#[derive(Debug, Serialize)]
pub struct Bim360CreateProjectRequest {
    /// Project name
    pub name: String,
    /// Comma-separated service types (e.g., "doc_manager,pm,field")
    pub service_types: String,
    /// Project type: "project" or "template"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// ACC Project response from API (camelCase)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccProject {
    /// Project ID
    pub id: String,
    /// Project name
    pub name: String,
    /// Project status (pending, active, etc.)
    pub status: Option<String>,
    /// Account ID
    pub account_id: Option<String>,
    /// Job ID (for tracking creation progress)
    pub job_id: Option<String>,
}

/// BIM 360 Project response from HQ v1 API (snake_case)
#[derive(Debug, Clone, Deserialize)]
pub struct Bim360Project {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
}

// ============================================================================
// ASSET TYPES
// ============================================================================

/// ACC Asset information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub category_id: Option<String>,
    pub status_id: Option<String>,
    pub client_asset_id: Option<String>,
    pub description: Option<String>,
    pub barcode: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Assets response
#[derive(Debug, Deserialize)]
pub struct AssetsResponse {
    pub results: Vec<Asset>,
    pub pagination: Option<Pagination>,
}

/// Request body for creating an asset
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAssetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_asset_id: Option<String>,
}

/// Request body for updating an asset
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAssetRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barcode: Option<String>,
}

// ============================================================================
// SUBMITTAL TYPES
// ============================================================================

/// ACC Submittal information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Submittal {
    pub id: String,
    pub title: String,
    pub number: Option<String>,
    pub status: String,
    pub spec_section: Option<String>,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Submittals response
#[derive(Debug, Deserialize)]
pub struct SubmittalsResponse {
    pub results: Vec<Submittal>,
    pub pagination: Option<Pagination>,
}

/// Request body for creating a submittal
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubmittalRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spec_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

/// Request body for updating a submittal
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubmittalRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
}

// ============================================================================
// CHECKLIST TYPES
// ============================================================================

/// ACC Checklist template
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistTemplate {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
}

/// ACC Checklist instance
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Checklist {
    pub id: String,
    pub template_id: Option<String>,
    pub title: String,
    pub status: String,
    pub assignee_id: Option<String>,
    pub location: Option<String>,
    pub due_date: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Checklists response
#[derive(Debug, Deserialize)]
pub struct ChecklistsResponse {
    pub results: Vec<Checklist>,
    pub pagination: Option<Pagination>,
}

/// Request body for creating a checklist
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChecklistRequest {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
}

/// Request body for updating a checklist
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateChecklistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee_id: Option<String>,
}

// ============================================================================
// SHARED PAGINATION
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pagination {
    pub limit: i32,
    pub offset: i32,
    pub total_results: i32,
}
