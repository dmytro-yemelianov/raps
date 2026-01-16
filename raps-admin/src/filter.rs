// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project filter for selecting target projects

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AdminError;

/// Platform type for projects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// Autodesk Construction Cloud
    Acc,
    /// BIM 360 (legacy)
    Bim360,
}

/// Project status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Inactive,
    Archived,
}

/// Region for projects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    Us,
    Emea,
}

/// Filter criteria for selecting target projects
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFilter {
    /// Glob pattern for project name matching
    pub name_pattern: Option<String>,
    /// Filter by project status (default: Active)
    pub status: Option<ProjectStatus>,
    /// Filter by platform (ACC, BIM360, or both if None)
    pub platform: Option<Platform>,
    /// Include projects created after this date
    pub created_after: Option<DateTime<Utc>>,
    /// Include projects created before this date
    pub created_before: Option<DateTime<Utc>>,
    /// Filter by region
    pub region: Option<Region>,
    /// Explicit list of project IDs to include
    pub include_ids: Option<Vec<String>>,
    /// Explicit list of project IDs to exclude
    pub exclude_ids: Option<Vec<String>>,
}

impl ProjectFilter {
    /// Create a new empty filter (matches all projects)
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse filter from string expression
    ///
    /// Syntax: `key:value[,key:value...]`
    ///
    /// Keys:
    /// - `name` - Project name (supports * wildcard)
    /// - `status` - Project status (active, inactive, archived)
    /// - `platform` - Platform type (acc, bim360)
    /// - `created` - Date filter (>YYYY-MM-DD, <YYYY-MM-DD)
    /// - `region` - Region (us, emea)
    ///
    /// Example: `name:*Hospital*,status:active,platform:acc`
    pub fn from_expression(expr: &str) -> Result<Self, AdminError> {
        let mut filter = Self::new();

        for part in expr.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            let (key, value) = part
                .split_once(':')
                .ok_or_else(|| AdminError::InvalidFilter {
                    message: format!("Invalid filter syntax: '{}'. Expected 'key:value'", part),
                })?;

            match key.trim().to_lowercase().as_str() {
                "name" => filter.name_pattern = Some(value.trim().to_string()),
                "status" => {
                    filter.status = Some(match value.trim().to_lowercase().as_str() {
                        "active" => ProjectStatus::Active,
                        "inactive" => ProjectStatus::Inactive,
                        "archived" => ProjectStatus::Archived,
                        _ => {
                            return Err(AdminError::InvalidFilter {
                                message: format!(
                                    "Invalid status: '{}'. Expected: active, inactive, archived",
                                    value
                                ),
                            });
                        }
                    });
                }
                "platform" => {
                    filter.platform = Some(match value.trim().to_lowercase().as_str() {
                        "acc" => Platform::Acc,
                        "bim360" => Platform::Bim360,
                        _ => {
                            return Err(AdminError::InvalidFilter {
                                message: format!(
                                    "Invalid platform: '{}'. Expected: acc, bim360",
                                    value
                                ),
                            });
                        }
                    });
                }
                "region" => {
                    filter.region = Some(match value.trim().to_lowercase().as_str() {
                        "us" => Region::Us,
                        "emea" => Region::Emea,
                        _ => {
                            return Err(AdminError::InvalidFilter {
                                message: format!("Invalid region: '{}'. Expected: us, emea", value),
                            });
                        }
                    });
                }
                "created" => {
                    let value = value.trim();
                    if let Some(date_str) = value.strip_prefix('>') {
                        let date = parse_date(date_str.trim())?;
                        filter.created_after = Some(date);
                    } else if let Some(date_str) = value.strip_prefix('<') {
                        let date = parse_date(date_str.trim())?;
                        filter.created_before = Some(date);
                    } else {
                        return Err(AdminError::InvalidFilter {
                            message: format!(
                                "Invalid created filter: '{}'. Use >YYYY-MM-DD or <YYYY-MM-DD",
                                value
                            ),
                        });
                    }
                }
                _ => {
                    return Err(AdminError::InvalidFilter {
                        message: format!(
                            "Unknown filter key: '{}'. Valid keys: name, status, platform, created, region",
                            key
                        ),
                    });
                }
            }
        }

        Ok(filter)
    }

    /// Check if a project name matches the filter's name pattern
    pub fn matches_name(&self, project_name: &str) -> bool {
        match &self.name_pattern {
            None => true,
            Some(pattern) => {
                let glob_pattern = glob::Pattern::new(pattern).ok();
                glob_pattern
                    .map(|p| p.matches(project_name))
                    .unwrap_or(false)
            }
        }
    }

    /// Check if a project matches all filter criteria
    pub fn matches(&self, project: &raps_acc::types::AccountProject) -> bool {
        // Check name pattern
        if !self.matches_name(&project.name) {
            return false;
        }

        // Check status
        if let Some(ref filter_status) = self.status {
            let project_status = project
                .status
                .as_ref()
                .map(|s| s.to_lowercase())
                .unwrap_or_else(|| "active".to_string());

            let status_matches = match filter_status {
                ProjectStatus::Active => project_status == "active",
                ProjectStatus::Inactive => project_status == "inactive",
                ProjectStatus::Archived => project_status == "archived",
            };

            if !status_matches {
                return false;
            }
        }

        // Check platform
        if let Some(ref filter_platform) = self.platform {
            let platform_matches = match filter_platform {
                Platform::Acc => project.is_acc(),
                Platform::Bim360 => project.is_bim360(),
            };

            if !platform_matches {
                return false;
            }
        }

        // Check created_after
        if let Some(ref after_date) = self.created_after {
            if let Some(ref created) = project.created_at {
                if created < after_date {
                    return false;
                }
            }
        }

        // Check created_before
        if let Some(ref before_date) = self.created_before {
            if let Some(ref created) = project.created_at {
                if created > before_date {
                    return false;
                }
            }
        }

        // Check include_ids (if specified, project must be in the list)
        if let Some(ref include_ids) = self.include_ids {
            if !include_ids.contains(&project.id) {
                return false;
            }
        }

        // Check exclude_ids
        if let Some(ref exclude_ids) = self.exclude_ids {
            if exclude_ids.contains(&project.id) {
                return false;
            }
        }

        true
    }

    /// Apply filter to a list of projects
    pub fn apply(
        &self,
        projects: Vec<raps_acc::types::AccountProject>,
    ) -> Vec<raps_acc::types::AccountProject> {
        projects.into_iter().filter(|p| self.matches(p)).collect()
    }
}

/// Parse a date string in YYYY-MM-DD format
fn parse_date(s: &str) -> Result<DateTime<Utc>, AdminError> {
    let naive = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
        AdminError::InvalidFilter {
            message: format!("Invalid date format: '{}'. Expected YYYY-MM-DD", s),
        }
    })?;

    Ok(naive.and_hms_opt(0, 0, 0).expect("Valid time").and_utc())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_filter() {
        let filter = ProjectFilter::from_expression("").unwrap();
        assert!(filter.name_pattern.is_none());
        assert!(filter.status.is_none());
    }

    #[test]
    fn test_parse_name_filter() {
        let filter = ProjectFilter::from_expression("name:*Hospital*").unwrap();
        assert_eq!(filter.name_pattern, Some("*Hospital*".to_string()));
    }

    #[test]
    fn test_parse_multiple_filters() {
        let filter =
            ProjectFilter::from_expression("name:*Building*,status:active,platform:acc").unwrap();
        assert_eq!(filter.name_pattern, Some("*Building*".to_string()));
        assert_eq!(filter.status, Some(ProjectStatus::Active));
        assert_eq!(filter.platform, Some(Platform::Acc));
    }

    #[test]
    fn test_parse_date_filter() {
        let filter = ProjectFilter::from_expression("created:>2024-01-01").unwrap();
        assert!(filter.created_after.is_some());
    }

    #[test]
    fn test_invalid_filter_syntax() {
        let result = ProjectFilter::from_expression("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_matches_name() {
        let filter = ProjectFilter {
            name_pattern: Some("*Hospital*".to_string()),
            ..Default::default()
        };
        assert!(filter.matches_name("City Hospital Phase 2"));
        assert!(filter.matches_name("Hospital"));
        assert!(!filter.matches_name("Office Building"));
    }
}
