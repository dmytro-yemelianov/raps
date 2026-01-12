// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Test utilities for mocking API responses
//!
//! Provides infrastructure for testing API clients with mocked HTTP responses.
//! This module is only available when running tests.

use crate::auth::AuthClient;
use crate::config::Config;
use crate::http::HttpClientConfig;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test configuration builder for creating test clients
pub struct TestConfig {
    /// Base URL (set to mock server URL)
    pub base_url: String,
    /// Client ID for testing
    pub client_id: String,
    /// Client secret for testing
    pub client_secret: String,
    /// Callback URL for OAuth
    pub callback_url: String,
    /// Design Automation nickname
    pub da_nickname: Option<String>,
}

impl TestConfig {
    /// Create a new test configuration with the given mock server URL
    pub fn new(mock_server_url: &str) -> Self {
        Self {
            base_url: mock_server_url.to_string(),
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: Some("test-nickname".to_string()),
        }
    }

    /// Convert to a Config struct
    pub fn to_config(&self) -> Config {
        Config {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            base_url: self.base_url.clone(),
            callback_url: self.callback_url.clone(),
            da_nickname: self.da_nickname.clone(),
            http_config: HttpClientConfig::default(),
        }
    }

    /// Create an AuthClient for testing
    pub fn create_auth_client(&self) -> AuthClient {
        AuthClient::new(self.to_config())
    }
}

/// Mock response builder for common API responses
pub struct MockResponseBuilder;

impl MockResponseBuilder {
    /// Create a successful 2-legged OAuth token response
    pub fn token_response(access_token: &str, expires_in: u64) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": access_token,
            "token_type": "Bearer",
            "expires_in": expires_in
        }))
    }

    /// Create an unauthorized (401) response
    pub fn unauthorized() -> ResponseTemplate {
        ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "developerMessage": "The access token provided is invalid or has expired.",
            "errorCode": "AUTH-001",
            "more info": "https://developer.api.autodesk.com/error/AUTH-001"
        }))
    }

    /// Create a forbidden (403) response
    pub fn forbidden() -> ResponseTemplate {
        ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "developerMessage": "You don't have permission to access this resource.",
            "errorCode": "AUTH-002"
        }))
    }

    /// Create a not found (404) response
    pub fn not_found(resource: &str) -> ResponseTemplate {
        ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "reason": format!("{} not found", resource)
        }))
    }

    /// Create a conflict (409) response
    pub fn conflict(reason: &str) -> ResponseTemplate {
        ResponseTemplate::new(409).set_body_json(serde_json::json!({
            "reason": reason
        }))
    }

    /// Create a rate limit (429) response
    pub fn rate_limited() -> ResponseTemplate {
        ResponseTemplate::new(429)
            .insert_header("Retry-After", "60")
            .set_body_json(serde_json::json!({
                "reason": "Rate limit exceeded"
            }))
    }

    /// Create an internal server error (500) response
    pub fn internal_error() -> ResponseTemplate {
        ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "reason": "Internal server error"
        }))
    }

    /// Create a service unavailable (503) response
    pub fn service_unavailable() -> ResponseTemplate {
        ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "reason": "Service temporarily unavailable"
        }))
    }

    /// Create a successful bucket list response
    pub fn bucket_list(buckets: Vec<(&str, &str)>) -> ResponseTemplate {
        let items: Vec<serde_json::Value> = buckets
            .into_iter()
            .map(|(key, policy)| {
                serde_json::json!({
                    "bucketKey": key,
                    "createdDate": 1609459200000_i64,
                    "policyKey": policy
                })
            })
            .collect();

        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": items
        }))
    }

    /// Create a successful bucket details response
    pub fn bucket_details(bucket_key: &str, policy: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "bucketKey": bucket_key,
            "bucketOwner": "test-owner",
            "createdDate": 1609459200000_i64,
            "permissions": [
                { "authId": "test-auth-id", "access": "full" }
            ],
            "policyKey": policy
        }))
    }

    /// Create a successful object list response
    pub fn object_list(objects: Vec<(&str, u64)>) -> ResponseTemplate {
        let items: Vec<serde_json::Value> = objects
            .into_iter()
            .map(|(key, size)| {
                serde_json::json!({
                    "bucketKey": "test-bucket",
                    "objectKey": key,
                    "objectId": format!("urn:adsk.objects:os.object:test-bucket/{}", key),
                    "sha1": "abc123",
                    "size": size,
                    "contentType": "application/octet-stream",
                    "location": format!("https://developer.api.autodesk.com/oss/v2/buckets/test-bucket/objects/{}", key)
                })
            })
            .collect();

        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": items
        }))
    }

    /// Create a successful upload response
    pub fn upload_success(bucket_key: &str, object_key: &str, size: u64) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "bucketKey": bucket_key,
            "objectKey": object_key,
            "objectId": format!("urn:adsk.objects:os.object:{}/{}", bucket_key, object_key),
            "sha1": "abc123def456",
            "size": size,
            "contentType": "application/octet-stream",
            "location": format!("https://developer.api.autodesk.com/oss/v2/buckets/{}/objects/{}", bucket_key, object_key)
        }))
    }

    /// Create a successful signed URL response
    pub fn signed_url(url: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "signedUrl": url,
            "expiration": 3600
        }))
    }

    /// Create a successful manifest response (complete)
    pub fn manifest_complete(urn: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "manifest",
            "hasThumbnail": "true",
            "status": "success",
            "progress": "complete",
            "region": "US",
            "urn": urn,
            "version": "1.0",
            "derivatives": [
                {
                    "status": "success",
                    "progress": "complete",
                    "outputType": "svf2",
                    "children": []
                }
            ]
        }))
    }

    /// Create a pending manifest response
    pub fn manifest_pending(urn: &str, progress: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "manifest",
            "hasThumbnail": "false",
            "status": "inprogress",
            "progress": progress,
            "region": "US",
            "urn": urn,
            "derivatives": []
        }))
    }

    /// Create a failed manifest response
    pub fn manifest_failed(urn: &str, reason: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "type": "manifest",
            "hasThumbnail": "false",
            "status": "failed",
            "progress": "complete",
            "region": "US",
            "urn": urn,
            "derivatives": [
                {
                    "status": "failed",
                    "progress": "complete",
                    "outputType": "svf2",
                    "messages": [
                        {
                            "type": "error",
                            "message": reason
                        }
                    ]
                }
            ]
        }))
    }

    /// Create a translation job accepted response
    pub fn translation_accepted(urn: &str) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "result": "success",
            "urn": urn,
            "acceptedJobs": {
                "output": {
                    "formats": [
                        { "type": "svf2", "views": ["2d", "3d"] }
                    ]
                }
            }
        }))
    }

    /// Create a hub list response
    pub fn hub_list(hubs: Vec<(&str, &str)>) -> ResponseTemplate {
        let data: Vec<serde_json::Value> = hubs
            .into_iter()
            .map(|(id, name)| {
                serde_json::json!({
                    "type": "hubs",
                    "id": id,
                    "attributes": {
                        "name": name,
                        "region": "US"
                    }
                })
            })
            .collect();

        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonapi": { "version": "1.0" },
            "data": data
        }))
    }

    /// Create a project list response
    pub fn project_list(projects: Vec<(&str, &str)>) -> ResponseTemplate {
        let data: Vec<serde_json::Value> = projects
            .into_iter()
            .map(|(id, name)| {
                serde_json::json!({
                    "type": "projects",
                    "id": id,
                    "attributes": {
                        "name": name
                    }
                })
            })
            .collect();

        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonapi": { "version": "1.0" },
            "data": data
        }))
    }
}

/// Helper to set up common auth mock
pub async fn mock_auth(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/authentication/v2/token"))
        .respond_with(MockResponseBuilder::token_response(
            "test-token-12345",
            3600,
        ))
        .mount(server)
        .await;
}

/// Helper to set up bucket list mock
pub async fn mock_bucket_list(server: &MockServer, buckets: Vec<(&str, &str)>) {
    Mock::given(method("GET"))
        .and(path("/oss/v2/buckets"))
        .and(header("Authorization", "Bearer test-token-12345"))
        .respond_with(MockResponseBuilder::bucket_list(buckets))
        .mount(server)
        .await;
}

/// Helper to set up object list mock
pub async fn mock_object_list(server: &MockServer, bucket: &str, objects: Vec<(&str, u64)>) {
    Mock::given(method("GET"))
        .and(path(format!("/oss/v2/buckets/{}/objects", bucket)))
        .and(header("Authorization", "Bearer test-token-12345"))
        .respond_with(MockResponseBuilder::object_list(objects))
        .mount(server)
        .await;
}

/// Create a temporary directory for test files
pub fn create_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Create a test file with random content
pub fn create_test_file(dir: &tempfile::TempDir, name: &str, size: usize) -> std::path::PathBuf {
    use std::io::Write;

    let path = dir.path().join(name);
    let mut file = std::fs::File::create(&path).expect("Failed to create test file");

    // Write random bytes
    let content: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    file.write_all(&content).expect("Failed to write test file");

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_config_creation() {
        let config = TestConfig::new("http://localhost:8080");
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.client_id, "test-client-id");
    }

    #[test]
    fn test_test_config_to_config() {
        let test_config = TestConfig::new("http://localhost:8080");
        let config = test_config.to_config();
        assert_eq!(config.base_url, "http://localhost:8080");
        assert_eq!(config.client_id, "test-client-id");
    }

    #[test]
    fn test_mock_response_builder_token() {
        // Verify builder doesn't panic and creates a valid ResponseTemplate
        let _response = MockResponseBuilder::token_response("my-token", 7200);
    }

    #[test]
    fn test_mock_response_builder_unauthorized() {
        let _response = MockResponseBuilder::unauthorized();
    }

    #[test]
    fn test_mock_response_builder_not_found() {
        let _response = MockResponseBuilder::not_found("Bucket");
    }

    #[test]
    fn test_mock_response_builder_conflict() {
        let _response = MockResponseBuilder::conflict("Already exists");
    }

    #[test]
    fn test_mock_response_builder_rate_limited() {
        let _response = MockResponseBuilder::rate_limited();
    }

    #[test]
    fn test_mock_response_builder_internal_error() {
        let _response = MockResponseBuilder::internal_error();
    }

    #[test]
    fn test_mock_response_builder_bucket_list() {
        let _response = MockResponseBuilder::bucket_list(vec![
            ("bucket1", "transient"),
            ("bucket2", "persistent"),
        ]);
    }

    #[test]
    fn test_mock_response_builder_object_list() {
        let _response =
            MockResponseBuilder::object_list(vec![("file1.dwg", 1024), ("file2.rvt", 2048)]);
    }

    #[test]
    fn test_mock_response_builder_manifest_complete() {
        let _response = MockResponseBuilder::manifest_complete("test-urn");
    }

    #[test]
    fn test_mock_response_builder_manifest_pending() {
        let _response = MockResponseBuilder::manifest_pending("test-urn", "50%");
    }

    #[test]
    fn test_create_temp_dir() {
        let dir = create_temp_dir();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_create_test_file() {
        let dir = create_temp_dir();
        let file_path = create_test_file(&dir, "test.bin", 1024);
        assert!(file_path.exists());
        assert_eq!(std::fs::metadata(&file_path).unwrap().len(), 1024);
    }

    #[tokio::test]
    async fn test_mock_server_basic() {
        let server = MockServer::start().await;
        let config = TestConfig::new(&server.uri());

        Mock::given(method("GET"))
            .and(path("/test"))
            .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
            .mount(&server)
            .await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}/test", config.base_url))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await.unwrap(), "OK");
    }

    #[tokio::test]
    async fn test_mock_auth_helper() {
        let server = MockServer::start().await;
        mock_auth(&server).await;

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/authentication/v2/token", server.uri()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let json: serde_json::Value = response.json().await.unwrap();
        assert_eq!(json["access_token"], "test-token-12345");
    }
}
