// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use super::*;

use crate::types::ProjectClassification;

#[test]
fn test_normalize_account_id() {
    // BIM 360 format: b.{uuid}
    assert_eq!(normalize_account_id("b.123-456"), "123-456");
    // Raw UUID
    assert_eq!(normalize_account_id("123-456"), "123-456");
    // ACC format: a.{base64} where base64 decodes to "business:{account_id}"
    // "YnVzaW5lc3M6Z21haWw2MDUzMTAz" decodes to "business:gmail6053103"
    assert_eq!(
        normalize_account_id("a.YnVzaW5lc3M6Z21haWw2MDUzMTAz"),
        "gmail6053103"
    );
}

#[test]
fn test_strip_project_prefix_in_admin() {
    assert_eq!(crate::strip_project_prefix("b.proj-123"), "proj-123");
    assert_eq!(crate::strip_project_prefix("proj-123"), "proj-123");
}

#[test]
fn test_create_project_request_serialization() {
    let request = CreateProjectRequest {
        name: "New Project".to_string(),
        r#type: Some("Bridge".to_string()),
        classification: Some(ProjectClassification::Production),
        ..Default::default()
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("New Project"));
    assert!(json.contains("Bridge"));
}

#[test]
fn test_update_project_request_serialization() {
    let request = UpdateProjectRequest {
        name: Some("Updated Name".to_string()),
        status: Some("active".to_string()),
        ..Default::default()
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("Updated Name"));
    assert!(json.contains("active"));
}

#[test]
fn test_update_project_request_skips_none() {
    let request = UpdateProjectRequest::default();
    let json = serde_json::to_string(&request).unwrap();
    // All fields are None with skip_serializing_if, so should be minimal
    assert!(!json.contains("name"));
}

#[test]
fn test_update_account_user_request_serialization() {
    let request = UpdateAccountUserRequest {
        company_id: Some("comp-123".to_string()),
        company_name: Some("Acme".to_string()),
        status: None,
    };
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("comp-123"));
    assert!(json.contains("Acme"));
}

/// Integration tests using raps-mock
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::http::HttpClientConfig;

    fn create_mock_admin_client(mock_url: &str) -> AccountAdminClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        AccountAdminClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_admin_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_admin_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_list_companies() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_admin_client(&server.url);
        let result = client.list_companies("account-123").await;
        let _ = result;
    }

    #[tokio::test]
    async fn test_create_project() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_admin_client(&server.url);
        let request = CreateProjectRequest {
            name: "Test Project".to_string(),
            ..Default::default()
        };
        let result = client.create_project("account-123", request).await;
        let _ = result;
    }
}
