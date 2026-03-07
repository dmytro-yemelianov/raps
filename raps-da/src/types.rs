// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Type definitions for the Design Automation API module.

use serde::{Deserialize, Serialize};

/// Engine information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Engine {
    pub id: String,
    pub description: Option<String>,
    pub product_version: Option<String>,
}

/// AppBundle information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundle {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
}

/// AppBundle details (full)
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBundleDetails {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: i32,
    pub package: Option<String>,
    pub upload_parameters: Option<UploadParameters>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadParameters {
    pub endpoint_url: Option<String>,
    pub form_data: Option<std::collections::HashMap<String, String>>,
}

/// Activity information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
    pub version: Option<i32>,
    pub command_line: Option<Vec<String>>,
    pub app_bundles: Option<Vec<String>>,
}

/// WorkItem information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItem {
    pub id: String,
    pub status: String,
    pub progress: Option<String>,
    pub report_url: Option<String>,
    pub stats: Option<WorkItemStats>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemStats {
    pub time_queued: Option<String>,
    pub time_download_started: Option<String>,
    pub time_instruction_started: Option<String>,
    pub time_instruction_ended: Option<String>,
    pub time_upload_ended: Option<String>,
    pub time_finished: Option<String>,
    pub bytes_downloaded: Option<i64>,
    pub bytes_uploaded: Option<i64>,
}

/// Request to create an AppBundle
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppBundleRequest {
    pub id: String,
    pub engine: String,
    pub description: Option<String>,
}

/// Request to create an Activity
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateActivityRequest {
    pub id: String,
    pub engine: String,
    pub command_line: Vec<String>,
    pub app_bundles: Vec<String>,
    pub parameters: std::collections::HashMap<String, ActivityParameter>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityParameter {
    pub verb: String,
    pub local_name: Option<String>,
    pub description: Option<String>,
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip: Option<bool>,
}

/// Request to create a WorkItem
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkItemRequest {
    pub activity_id: String,
    pub arguments: std::collections::HashMap<String, WorkItemArgument>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemArgument {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Paginated response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_deserialization() {
        let json = r#"{
            "id": "Autodesk.Revit+2024",
            "description": "Revit 2024 Engine",
            "productVersion": "2024"
        }"#;

        let engine: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(engine.id, "Autodesk.Revit+2024");
        assert_eq!(engine.description, Some("Revit 2024 Engine".to_string()));
    }

    #[test]
    fn test_appbundle_deserialization() {
        let json = r#"{
            "id": "myapp.MyBundle+dev",
            "engine": "Autodesk.Revit+2024",
            "description": "My custom bundle",
            "version": 1
        }"#;

        let bundle: AppBundle = serde_json::from_str(json).unwrap();
        assert_eq!(bundle.id, "myapp.MyBundle+dev");
        assert_eq!(bundle.engine, "Autodesk.Revit+2024");
    }

    #[test]
    fn test_activity_deserialization() {
        let json = r#"{
            "id": "myapp.MyActivity+dev",
            "engine": "Autodesk.Revit+2024",
            "description": "My activity",
            "version": 1
        }"#;

        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.id, "myapp.MyActivity+dev");
    }

    #[test]
    fn test_workitem_deserialization() {
        let json = r#"{
            "id": "workitem-id-123",
            "status": "pending",
            "progress": "0%"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.id, "workitem-id-123");
        assert_eq!(workitem.status, "pending");
    }

    #[test]
    fn test_workitem_stats_deserialization() {
        let json = r#"{
            "id": "workitem-id-123",
            "status": "success",
            "stats": {
                "bytesDownloaded": 1024,
                "bytesUploaded": 2048
            }
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert!(workitem.stats.is_some());
        let stats = workitem.stats.unwrap();
        assert_eq!(stats.bytes_downloaded, Some(1024));
    }

    #[test]
    fn test_create_appbundle_request_serialization() {
        let request = CreateAppBundleRequest {
            id: "MyBundle".to_string(),
            engine: "Autodesk.Revit+2024".to_string(),
            description: Some("Test bundle".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["id"], "MyBundle");
        assert_eq!(json["engine"], "Autodesk.Revit+2024");
    }

    #[test]
    fn test_create_activity_request_serialization() {
        let mut parameters = std::collections::HashMap::new();
        parameters.insert(
            "input".to_string(),
            ActivityParameter {
                verb: "get".to_string(),
                local_name: Some("input.rvt".to_string()),
                description: None,
                required: Some(true),
                zip: None,
            },
        );

        let request = CreateActivityRequest {
            id: "MyActivity".to_string(),
            engine: "Autodesk.Revit+2024".to_string(),
            command_line: vec!["$(engine.path)\\revitcoreconsole.exe".to_string()],
            app_bundles: vec!["myapp.MyBundle+dev".to_string()],
            description: Some("Test activity".to_string()),
            parameters,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["id"], "MyActivity");
        assert!(json["commandLine"].is_array());
    }

    #[test]
    fn test_create_workitem_request_serialization() {
        let mut arguments = std::collections::HashMap::new();
        arguments.insert(
            "input".to_string(),
            WorkItemArgument {
                url: "https://example.com/input.rvt".to_string(),
                verb: Some("get".to_string()),
                headers: None,
            },
        );

        let request = CreateWorkItemRequest {
            activity_id: "myapp.MyActivity+dev".to_string(),
            arguments,
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["activityId"], "myapp.MyActivity+dev");
    }

    #[test]
    fn test_paginated_response_deserialization() {
        let json = r#"{
            "paginationToken": "next-page-token",
            "data": [
                {"id": "item1", "engine": "engine1"},
                {"id": "item2", "engine": "engine2"}
            ]
        }"#;

        let response: PaginatedResponse<AppBundle> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.pagination_token,
            Some("next-page-token".to_string())
        );
        assert_eq!(response.data.len(), 2);
    }

    #[test]
    fn test_workitem_with_progress() {
        let json = r#"{
            "id": "workitem-id",
            "status": "inprogress",
            "progress": "50%"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.status, "inprogress");
        assert_eq!(workitem.progress, Some("50%".to_string()));
    }

    #[test]
    fn test_workitem_with_report_url() {
        let json = r#"{
            "id": "workitem-id",
            "status": "success",
            "reportUrl": "https://example.com/report.txt"
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert!(workitem.report_url.is_some());
    }

    #[test]
    fn test_activity_parameter_serialization() {
        let param = ActivityParameter {
            verb: "get".to_string(),
            local_name: Some("input.rvt".to_string()),
            description: Some("Input file".to_string()),
            required: Some(true),
            zip: Some(false),
        };

        let json = serde_json::to_value(&param).unwrap();
        assert_eq!(json["verb"], "get");
        assert_eq!(json["localName"], "input.rvt");
        assert_eq!(json["required"], true);
    }

    #[test]
    fn test_workitem_argument_with_headers() {
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer token".to_string());

        let arg = WorkItemArgument {
            url: "https://example.com/file.rvt".to_string(),
            verb: Some("get".to_string()),
            headers: Some(headers),
        };

        let json = serde_json::to_value(&arg).unwrap();
        assert_eq!(json["url"], "https://example.com/file.rvt");
        assert_eq!(json["headers"]["Authorization"], "Bearer token");
    }

    #[test]
    fn test_engine_with_product_version() {
        let json = r#"{
            "id": "Autodesk.Revit+2024",
            "productVersion": "2024"
        }"#;

        let engine: Engine = serde_json::from_str(json).unwrap();
        assert_eq!(engine.id, "Autodesk.Revit+2024");
        assert_eq!(engine.product_version, Some("2024".to_string()));
    }

    #[test]
    fn test_paginated_workitem_response_deserialization() {
        let json = r#"{
            "paginationToken": "next-token-abc",
            "data": [
                {
                    "id": "wi-001",
                    "status": "success",
                    "progress": "100%",
                    "reportUrl": "https://example.com/report1.txt"
                },
                {
                    "id": "wi-002",
                    "status": "pending"
                }
            ]
        }"#;

        let response: PaginatedResponse<WorkItem> = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.pagination_token,
            Some("next-token-abc".to_string())
        );
        assert_eq!(response.data.len(), 2);
        assert_eq!(response.data[0].id, "wi-001");
        assert_eq!(response.data[0].status, "success");
        assert!(response.data[0].report_url.is_some());
        assert_eq!(response.data[1].id, "wi-002");
        assert_eq!(response.data[1].status, "pending");
        assert!(response.data[1].report_url.is_none());
    }

    #[test]
    fn test_paginated_workitem_response_no_token() {
        let json = r#"{
            "data": [
                {
                    "id": "wi-003",
                    "status": "inprogress",
                    "progress": "25%"
                }
            ]
        }"#;

        let response: PaginatedResponse<WorkItem> = serde_json::from_str(json).unwrap();
        assert!(response.pagination_token.is_none());
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].progress, Some("25%".to_string()));
    }

    #[test]
    fn test_workitem_full_stats_deserialization() {
        let json = r#"{
            "id": "wi-full",
            "status": "success",
            "reportUrl": "https://example.com/report.txt",
            "stats": {
                "timeQueued": "2024-01-01T00:00:00Z",
                "timeDownloadStarted": "2024-01-01T00:00:01Z",
                "timeInstructionStarted": "2024-01-01T00:00:02Z",
                "timeInstructionEnded": "2024-01-01T00:01:00Z",
                "timeUploadEnded": "2024-01-01T00:01:05Z",
                "timeFinished": "2024-01-01T00:01:06Z",
                "bytesDownloaded": 5242880,
                "bytesUploaded": 1048576
            }
        }"#;

        let workitem: WorkItem = serde_json::from_str(json).unwrap();
        assert_eq!(workitem.id, "wi-full");
        assert_eq!(workitem.status, "success");
        let stats = workitem.stats.unwrap();
        assert_eq!(stats.time_queued, Some("2024-01-01T00:00:00Z".to_string()));
        assert_eq!(stats.bytes_downloaded, Some(5242880));
        assert_eq!(stats.bytes_uploaded, Some(1048576));
        assert_eq!(
            stats.time_finished,
            Some("2024-01-01T00:01:06Z".to_string())
        );
    }

    #[test]
    fn test_upload_parameters_deserialization() {
        let json = r#"{
            "endpointUrl": "https://s3.amazonaws.com/da-uploads",
            "formData": {
                "key": "apps/myapp/bundle.zip",
                "policy": "base64-encoded-policy",
                "x-amz-signature": "sig123",
                "x-amz-credential": "cred456",
                "x-amz-date": "20240101T000000Z"
            }
        }"#;

        let params: UploadParameters = serde_json::from_str(json).unwrap();
        assert_eq!(
            params.endpoint_url,
            Some("https://s3.amazonaws.com/da-uploads".to_string())
        );
        let form_data = params.form_data.unwrap();
        assert_eq!(form_data.len(), 5);
        assert_eq!(
            form_data.get("key"),
            Some(&"apps/myapp/bundle.zip".to_string())
        );
    }

    #[test]
    fn test_upload_parameters_missing_endpoint() {
        let json = r#"{}"#;
        let params: UploadParameters = serde_json::from_str(json).unwrap();
        assert!(params.endpoint_url.is_none());
        assert!(params.form_data.is_none());
    }

    #[test]
    fn test_appbundle_details_with_upload_params() {
        let json = r#"{
            "id": "myapp.MyBundle+dev",
            "engine": "Autodesk.Revit+2024",
            "version": 2,
            "uploadParameters": {
                "endpointUrl": "https://s3.amazonaws.com/upload",
                "formData": {
                    "key": "upload-key"
                }
            }
        }"#;

        let details: AppBundleDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.version, 2);
        assert!(details.upload_parameters.is_some());
        let params = details.upload_parameters.unwrap();
        assert!(params.endpoint_url.is_some());
    }

    // ==================== Contract Tests ====================

    #[test]
    fn test_contract_engine_response() {
        let json = include_str!("../../tests/fixtures/engine_response.json");
        let response: PaginatedResponse<Engine> = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_contract_appbundle_details() {
        let json = include_str!("../../tests/fixtures/appbundle_details.json");
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        // Verify it deserializes to our type (HashMap ordering makes debug snapshots flaky)
        let _response: AppBundleDetails = serde_json::from_value(value.clone()).unwrap();
        insta::assert_json_snapshot!(value);
    }

    #[test]
    fn test_contract_activity_response() {
        let json = include_str!("../../tests/fixtures/activity_response.json");
        let response: PaginatedResponse<Activity> = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_contract_workitem_response() {
        let json = include_str!("../../tests/fixtures/workitem_success.json");
        let response: WorkItem = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }
}
