// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tests for portfolio report types and helpers

#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_rfi_project_summary_serialization() {
        let summary = RfiProjectSummary {
            project_id: "proj-001".to_string(),
            project_name: "Hospital Wing A".to_string(),
            total: 25,
            open: 10,
            answered: 8,
            closed: 5,
            void: 2,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-001\""));
        assert!(json.contains("\"project_name\":\"Hospital Wing A\""));
        assert!(json.contains("\"total\":25"));
        assert!(json.contains("\"open\":10"));
        assert!(json.contains("\"answered\":8"));
        assert!(json.contains("\"closed\":5"));
        assert!(json.contains("\"void\":2"));
    }

    #[test]
    fn test_issue_project_summary_serialization() {
        let summary = IssueProjectSummary {
            project_id: "proj-002".to_string(),
            project_name: "Office Tower".to_string(),
            total: 40,
            open: 15,
            closed: 20,
            other: 5,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-002\""));
        assert!(json.contains("\"project_name\":\"Office Tower\""));
        assert!(json.contains("\"total\":40"));
        assert!(json.contains("\"open\":15"));
        assert!(json.contains("\"closed\":20"));
        assert!(json.contains("\"other\":5"));
    }

    #[test]
    fn test_submittal_project_summary_serialization() {
        let summary = SubmittalProjectSummary {
            project_id: "proj-003".to_string(),
            project_name: "Parking Garage".to_string(),
            total: 12,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_id\":\"proj-003\""));
        assert!(json.contains("\"total\":12"));
    }

    #[test]
    fn test_checklist_project_summary_serialization() {
        let summary = ChecklistProjectSummary {
            project_id: "proj-004".to_string(),
            project_name: "Bridge Inspection".to_string(),
            total: 7,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_name\":\"Bridge Inspection\""));
        assert!(json.contains("\"total\":7"));
    }

    #[test]
    fn test_asset_project_summary_serialization() {
        let summary = AssetProjectSummary {
            project_id: "proj-005".to_string(),
            project_name: "Data Center".to_string(),
            total: 150,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"project_name\":\"Data Center\""));
        assert!(json.contains("\"total\":150"));
    }

    #[test]
    fn test_report_summary_output_with_rfi_data() {
        let output = ReportSummaryOutput {
            total_projects: 2,
            projects: vec![
                RfiProjectSummary {
                    project_id: "p1".to_string(),
                    project_name: "Project A".to_string(),
                    total: 10,
                    open: 5,
                    answered: 3,
                    closed: 1,
                    void: 1,
                },
                RfiProjectSummary {
                    project_id: "p2".to_string(),
                    project_name: "Project B".to_string(),
                    total: 0,
                    open: 0,
                    answered: 0,
                    closed: 0,
                    void: 0,
                },
            ],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total_projects\":2"));
        assert!(json.contains("\"projects\":["));
        assert!(json.contains("Project A"));
        assert!(json.contains("Project B"));
    }

    #[test]
    fn test_report_summary_output_empty_projects() {
        let output: ReportSummaryOutput<IssueProjectSummary> = ReportSummaryOutput {
            total_projects: 0,
            projects: vec![],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"total_projects\":0"));
        assert!(json.contains("\"projects\":[]"));
    }

    #[test]
    fn test_rfi_project_summary_zero_counts() {
        let summary = RfiProjectSummary {
            project_id: "empty".to_string(),
            project_name: "Empty Project".to_string(),
            total: 0,
            open: 0,
            answered: 0,
            closed: 0,
            void: 0,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["total"], 0);
        assert_eq!(parsed["open"], 0);
        assert_eq!(parsed["answered"], 0);
        assert_eq!(parsed["closed"], 0);
        assert_eq!(parsed["void"], 0);
    }

    #[test]
    fn test_truncate_name_short() {
        assert_eq!(truncate_name("Short Name"), "Short Name");
    }

    #[test]
    fn test_truncate_name_long() {
        let long = "A Very Long Project Name That Exceeds Limit";
        let result = truncate_name(long);
        assert!(result.ends_with("..."));
        assert!(result.len() <= 28);
    }

    #[test]
    fn test_get_account_id_some() {
        let id = get_account_id(Some("abc-123".to_string())).unwrap();
        assert_eq!(id, "abc-123");
    }

    #[test]
    fn test_get_account_id_empty() {
        let result = get_account_id(Some(String::new()));
        assert!(result.is_err());
    }

    #[test]
    fn test_get_account_id_none() {
        // Without APS_ACCOUNT_ID set, should fail
        unsafe { std::env::remove_var("APS_ACCOUNT_ID") };
        let result = get_account_id(None);
        assert!(result.is_err());
    }

    // --- count_status ---

    struct MockStatusItem {
        status: String,
    }

    impl HasStatus for MockStatusItem {
        fn status(&self) -> &str {
            &self.status
        }
    }

    #[test]
    fn test_count_status_matches() {
        let items = vec![
            MockStatusItem {
                status: "open".to_string(),
            },
            MockStatusItem {
                status: "closed".to_string(),
            },
            MockStatusItem {
                status: "open".to_string(),
            },
        ];
        assert_eq!(count_status(&items, "open"), 2);
    }

    #[test]
    fn test_count_status_case_insensitive() {
        let items = vec![
            MockStatusItem {
                status: "Open".to_string(),
            },
            MockStatusItem {
                status: "OPEN".to_string(),
            },
        ];
        assert_eq!(count_status(&items, "open"), 2);
    }

    #[test]
    fn test_count_status_no_matches() {
        let items = vec![MockStatusItem {
            status: "closed".to_string(),
        }];
        assert_eq!(count_status(&items, "open"), 0);
    }

    #[test]
    fn test_count_status_empty() {
        let items: Vec<MockStatusItem> = vec![];
        assert_eq!(count_status(&items, "open"), 0);
    }

    // --- parse_project_filter ---

    #[test]
    fn test_parse_project_filter_none() {
        let filter = parse_project_filter(&None).unwrap();
        // Empty filter should not error
        assert!(format!("{:?}", filter).contains("ProjectFilter"));
    }

    // --- truncate_name boundary ---

    #[test]
    fn test_truncate_name_exactly_28() {
        let name = "a".repeat(28);
        assert_eq!(truncate_name(&name), name);
    }

    #[test]
    fn test_truncate_name_29_chars() {
        let name = "a".repeat(29);
        let result = truncate_name(&name);
        assert!(result.ends_with("..."));
        assert_eq!(result.len(), 28);
    }
}
