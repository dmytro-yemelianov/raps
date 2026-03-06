// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Request/response types for Account Admin API

use serde::{Deserialize, Serialize};

use crate::types::ProjectClassification;

/// Request to create a new project
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    /// Project name (required)
    pub name: String,
    /// Project type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Project classification (production, template, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification: Option<ProjectClassification>,
    /// Template configuration (for creating from template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateConfig>,
    /// Products to enable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub products: Option<Vec<String>>,
    /// Project start date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Project end date (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Project value/budget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Currency code (e.g., "USD")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Project address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line1: Option<String>,
    /// Project address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line2: Option<String>,
    /// City
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// State/Province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_or_province: Option<String>,
    /// Postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// Country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Time zone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Construction type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub construction_type: Option<String>,
    /// Contract type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_type: Option<String>,
}

/// Template configuration for creating project from template
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateConfig {
    /// ID of the template project to clone from
    pub project_id: String,
    /// Options for what to include when cloning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<TemplateOptions>,
}

/// Options for template cloning
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateOptions {
    /// Field-level options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<TemplateFieldOptions>,
}

/// Field-level options for template cloning
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TemplateFieldOptions {
    /// Whether to copy company data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_companies: Option<bool>,
    /// Whether to copy location data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_locations: Option<bool>,
}

/// Request to update an existing project
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    /// Project name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Project status (active, archived, suspended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Project start date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Project end date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Project type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Project value/budget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    /// Currency code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    /// Address line 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line1: Option<String>,
    /// Address line 2
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address_line2: Option<String>,
    /// City
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// State/Province
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_or_province: Option<String>,
    /// Postal code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// Country
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Time zone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
}

/// Request to update an account-level user's properties
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAccountUserRequest {
    /// Company ID to assign the user to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    /// Company name (for display purposes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_name: Option<String>,
}

/// A role available in an account (ACC or BIM 360)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountRole {
    /// Role ID (UUID)
    pub id: String,
    /// Human-readable role name (e.g., "Project Admin", "Project Member")
    pub name: String,
}

/// BIM 360 HQ v2 role response (snake_case)
#[derive(Debug, Deserialize)]
pub struct Bim360Role {
    pub id: String,
    pub name: String,
}
