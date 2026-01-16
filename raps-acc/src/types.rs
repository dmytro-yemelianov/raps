// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Shared types for ACC/BIM 360 API responses

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// PAGINATION
// ============================================================================

/// Paginated API response wrapper
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginatedResponse<T> {
    /// Results for the current page
    pub results: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationInfo,
}

/// Pagination metadata from API responses
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfo {
    /// Maximum items per page
    pub limit: usize,
    /// Current offset (starting index)
    pub offset: usize,
    /// Total number of results available
    pub total_results: usize,
}

impl<T> PaginatedResponse<T> {
    /// Check if there are more pages available
    pub fn has_more(&self) -> bool {
        self.pagination.offset + self.results.len() < self.pagination.total_results
    }

    /// Get the offset for the next page
    pub fn next_offset(&self) -> usize {
        self.pagination.offset + self.pagination.limit
    }

    /// Check if this is the first page
    pub fn is_first_page(&self) -> bool {
        self.pagination.offset == 0
    }
}

// ============================================================================
// ACCOUNT TYPES
// ============================================================================

/// Account/Hub information
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    /// Account ID (e.g., "b.account-uuid")
    pub id: String,
    /// Account name
    pub name: String,
    /// Account region
    #[serde(default)]
    pub region: Option<String>,
}

/// User within an Autodesk account
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUser {
    /// User ID (Autodesk user identifier)
    pub id: String,
    /// User's email address
    pub email: String,
    /// User's display name
    #[serde(default)]
    pub name: Option<String>,
    /// User's first name
    #[serde(default)]
    pub first_name: Option<String>,
    /// User's last name
    #[serde(default)]
    pub last_name: Option<String>,
    /// Company ID if associated
    #[serde(default)]
    pub company_id: Option<String>,
    /// User status in the account
    #[serde(default)]
    pub status: Option<String>,
    /// When the user was added to the account
    #[serde(default)]
    pub added_on: Option<DateTime<Utc>>,
}

impl AccountUser {
    /// Get the user's display name, falling back to email if not available
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.email)
    }
}

// ============================================================================
// PROJECT TYPES
// ============================================================================

/// Project within an account
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountProject {
    /// Project ID (e.g., "b.project-uuid")
    pub id: String,
    /// Project name
    pub name: String,
    /// Project status (active, inactive, archived)
    #[serde(default)]
    pub status: Option<String>,
    /// Platform type (ACC or BIM360)
    #[serde(default)]
    pub platform: Option<String>,
    /// Account ID this project belongs to
    #[serde(default)]
    pub account_id: Option<String>,
    /// Project creation date
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
    /// Last update date
    #[serde(default)]
    pub updated_at: Option<DateTime<Utc>>,
    /// Project type (e.g., "ACC", "BIM 360")
    #[serde(default, alias = "projectType")]
    pub project_type: Option<String>,
}

impl AccountProject {
    /// Check if this is an ACC project
    pub fn is_acc(&self) -> bool {
        self.platform
            .as_ref()
            .map(|p| p.to_lowercase() == "acc")
            .unwrap_or(false)
            || self
                .project_type
                .as_ref()
                .map(|t| t.to_lowercase().contains("acc"))
                .unwrap_or(false)
    }

    /// Check if this is a BIM 360 project
    pub fn is_bim360(&self) -> bool {
        self.platform
            .as_ref()
            .map(|p| p.to_lowercase().contains("bim360") || p.to_lowercase().contains("bim 360"))
            .unwrap_or(false)
            || self
                .project_type
                .as_ref()
                .map(|t| t.to_lowercase().contains("bim"))
                .unwrap_or(false)
    }

    /// Check if the project is active
    pub fn is_active(&self) -> bool {
        self.status
            .as_ref()
            .map(|s| s.to_lowercase() == "active")
            .unwrap_or(true)
    }
}

// ============================================================================
// PROJECT USER TYPES
// ============================================================================

/// User's membership in a specific project
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectUser {
    /// User ID
    pub id: String,
    /// User's email
    #[serde(default)]
    pub email: Option<String>,
    /// User's display name
    #[serde(default)]
    pub name: Option<String>,
    /// Role ID assigned in this project
    #[serde(default)]
    pub role_id: Option<String>,
    /// Role name
    #[serde(default)]
    pub role_name: Option<String>,
    /// Access levels for various products
    #[serde(default)]
    pub products: Option<Vec<ProductAccess>>,
    /// When user was added to the project
    #[serde(default)]
    pub added_on: Option<DateTime<Utc>>,
}

/// Product access configuration for a project user
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductAccess {
    /// Product key (e.g., "projectAdministration", "docs", "build")
    pub key: String,
    /// Access level (e.g., "administrator", "member", "none")
    pub access: String,
}

// ============================================================================
// FOLDER PERMISSION TYPES
// ============================================================================

/// Permission on a folder
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPermission {
    /// Permission ID
    pub id: String,
    /// Subject ID (user, role, or company ID)
    pub subject_id: String,
    /// Subject type
    pub subject_type: SubjectType,
    /// Permitted actions
    pub actions: Vec<String>,
}

/// Type of subject for permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubjectType {
    /// Individual user
    User,
    /// Role-based permission
    Role,
    /// Company-wide permission
    Company,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_response_has_more() {
        let response: PaginatedResponse<String> = PaginatedResponse {
            results: vec!["a".to_string(), "b".to_string()],
            pagination: PaginationInfo {
                limit: 2,
                offset: 0,
                total_results: 10,
            },
        };
        assert!(response.has_more());
        assert_eq!(response.next_offset(), 2);
    }

    #[test]
    fn test_paginated_response_last_page() {
        let response: PaginatedResponse<String> = PaginatedResponse {
            results: vec!["a".to_string()],
            pagination: PaginationInfo {
                limit: 2,
                offset: 8,
                total_results: 9,
            },
        };
        assert!(!response.has_more());
    }

    #[test]
    fn test_account_project_is_acc() {
        let project = AccountProject {
            id: "b.123".to_string(),
            name: "Test".to_string(),
            platform: Some("ACC".to_string()),
            status: None,
            account_id: None,
            created_at: None,
            updated_at: None,
            project_type: None,
        };
        assert!(project.is_acc());
        assert!(!project.is_bim360());
    }

    #[test]
    fn test_account_project_is_bim360() {
        let project = AccountProject {
            id: "b.123".to_string(),
            name: "Test".to_string(),
            platform: Some("BIM 360".to_string()),
            status: None,
            account_id: None,
            created_at: None,
            updated_at: None,
            project_type: None,
        };
        assert!(!project.is_acc());
        assert!(project.is_bim360());
    }
}
