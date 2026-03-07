// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management API module
//!
//! Handles access to Hubs, Projects, Folders, and Items in BIM 360/ACC.

mod folders;
mod hubs;
mod items;
pub mod types;

// Re-export all public types for backward compatibility
pub use types::*;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Maximum pages to follow during pagination (safety cap)
const MAX_PAGINATION_PAGES: usize = 100;

/// Data Management API client
#[derive(Clone)]
pub struct DataManagementClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl DataManagementClient {
    /// Create a new Data Management client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Data Management client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> Self {
        // Create HTTP client with configured timeouts
        let http_client = http_config.create_client().unwrap_or_else(|e| {
            tracing::warn!("HTTP client configuration failed, using defaults: {e}");
            reqwest::Client::new()
        });

        Self {
            config,
            auth,
            http_client,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_deserialization() {
        let json = r#"{
            "type": "hubs",
            "id": "b.hub-id",
            "attributes": {
                "name": "Test Hub",
                "region": "US"
            }
        }"#;

        let hub: Hub = serde_json::from_str(json).unwrap();
        assert_eq!(hub.hub_type, "hubs");
        assert_eq!(hub.id, "b.hub-id");
        assert_eq!(hub.attributes.name, "Test Hub");
    }

    #[test]
    fn test_project_deserialization() {
        let json = r#"{
            "type": "projects",
            "id": "b.project-id",
            "attributes": {
                "name": "Test Project"
            }
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.project_type, "projects");
        assert_eq!(project.attributes.name, "Test Project");
    }

    #[test]
    fn test_folder_deserialization() {
        let json = r#"{
            "type": "folders",
            "id": "urn:adsk.wipprod:folder.id",
            "attributes": {
                "name": "Project Files"
            }
        }"#;

        let folder: Folder = serde_json::from_str(json).unwrap();
        assert_eq!(folder.folder_type, "folders");
        assert_eq!(folder.attributes.name, "Project Files");
    }

    #[test]
    fn test_item_deserialization() {
        let json = r#"{
            "type": "items",
            "id": "urn:adsk.wipprod:dm.lineage:item-id",
            "attributes": {
                "displayName": "model.rvt"
            }
        }"#;

        let item: Item = serde_json::from_str(json).unwrap();
        assert_eq!(item.item_type, "items");
        assert_eq!(item.attributes.display_name, "model.rvt");
    }

    #[test]
    fn test_version_deserialization() {
        let json = r#"{
            "type": "versions",
            "id": "urn:adsk.wipprod:fs.file:version-id",
            "attributes": {
                "name": "model.rvt",
                "displayName": "model.rvt",
                "versionNumber": 1
            }
        }"#;

        let version: Version = serde_json::from_str(json).unwrap();
        assert_eq!(version.version_type, "versions");
        assert_eq!(version.attributes.version_number, Some(1));
    }

    #[test]
    fn test_create_folder_request_serialization() {
        let request = CreateFolderRequest {
            jsonapi: JsonApiVersion {
                version: "1.0".to_string(),
            },
            data: CreateFolderData {
                data_type: "folders".to_string(),
                attributes: CreateFolderAttributes {
                    name: "New Folder".to_string(),
                    extension: CreateFolderExtension {
                        ext_type: "folders:autodesk.bim360:Folder".to_string(),
                        version: "1.0".to_string(),
                    },
                },
                relationships: CreateFolderRelationships {
                    parent: CreateFolderParent {
                        data: CreateFolderParentData {
                            data_type: "folders".to_string(),
                            id: "parent-folder-id".to_string(),
                        },
                    },
                },
            },
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonapi"]["version"], "1.0");
        assert_eq!(json["data"]["type"], "folders");
        assert_eq!(json["data"]["attributes"]["name"], "New Folder");
    }

    #[test]
    fn test_hub_with_region() {
        let json = r#"{
            "type": "hubs",
            "id": "b.hub-id",
            "attributes": {
                "name": "Test Hub",
                "region": "US"
            }
        }"#;

        let hub: Hub = serde_json::from_str(json).unwrap();
        assert_eq!(hub.attributes.region, Some("US".to_string()));
    }

    #[test]
    fn test_project_with_scopes() {
        let json = r#"{
            "type": "projects",
            "id": "b.project-id",
            "attributes": {
                "name": "Test Project",
                "scopes": ["docs:read", "docs:write"]
            }
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert!(project.attributes.scopes.is_some());
        let scopes = project.attributes.scopes.unwrap();
        assert_eq!(scopes.len(), 2);
    }

    #[test]
    fn test_folder_with_display_name() {
        let json = r#"{
            "type": "folders",
            "id": "folder-id",
            "attributes": {
                "name": "folder",
                "displayName": "Project Files"
            }
        }"#;

        let folder: Folder = serde_json::from_str(json).unwrap();
        assert_eq!(
            folder.attributes.display_name,
            Some("Project Files".to_string())
        );
    }

    #[test]
    fn test_item_with_extension() {
        let json = r#"{
            "type": "items",
            "id": "item-id",
            "attributes": {
                "displayName": "model.rvt",
                "extension": {
                    "type": "items:autodesk.bim360:File",
                    "version": "1.0"
                }
            }
        }"#;

        let item: Item = serde_json::from_str(json).unwrap();
        assert!(item.attributes.extension.is_some());
        let ext = item.attributes.extension.unwrap();
        assert_eq!(
            ext.extension_type,
            Some("items:autodesk.bim360:File".to_string())
        );
    }

    #[test]
    fn test_version_with_storage_size() {
        let json = r#"{
            "type": "versions",
            "id": "version-id",
            "attributes": {
                "name": "model.rvt",
                "displayName": "model.rvt",
                "versionNumber": 2,
                "storageSize": 1048576
            }
        }"#;

        let version: Version = serde_json::from_str(json).unwrap();
        assert_eq!(version.attributes.storage_size, Some(1048576));
        assert_eq!(version.attributes.version_number, Some(2));
    }

    #[test]
    fn test_json_api_response_hubs_deserialization() {
        let json = r#"{
            "data": [{
                "type": "hubs",
                "id": "b.hub-123",
                "attributes": {
                    "name": "Test Hub",
                    "region": "US"
                }
            }],
            "included": [],
            "links": {
                "self": {"href": "https://api.example.com/hubs"}
            }
        }"#;

        let response: JsonApiResponse<Vec<Hub>> = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "b.hub-123");
    }

    #[test]
    fn test_json_api_response_single_hub() {
        let json = r#"{
            "data": {
                "type": "hubs",
                "id": "b.hub-456",
                "attributes": {
                    "name": "Single Hub"
                }
            },
            "included": []
        }"#;

        let response: JsonApiResponse<Hub> = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.id, "b.hub-456");
    }

    #[test]
    fn test_json_api_links_simple() {
        let json = r#"{
            "data": [],
            "links": {
                "self": "https://api.example.com/simple"
            }
        }"#;

        let response: JsonApiResponse<Vec<Hub>> = serde_json::from_str(json).unwrap();
        assert!(response.links.is_some());
    }

    #[test]
    fn test_json_api_links_complex() {
        let json = r#"{
            "data": [],
            "links": {
                "self": {"href": "https://api.example.com/complex"},
                "next": {"href": "https://api.example.com/complex?page=2"}
            }
        }"#;

        let response: JsonApiResponse<Vec<Hub>> = serde_json::from_str(json).unwrap();
        let links = response.links.unwrap();
        assert!(links.next.is_some());
    }

    #[test]
    fn test_hub_extension_deserialization() {
        let json = r#"{
            "type": "hubs",
            "id": "b.hub-789",
            "attributes": {
                "name": "Hub with Extension",
                "extension": {
                    "type": "hubs:autodesk.bim360:Account"
                }
            }
        }"#;

        let hub: Hub = serde_json::from_str(json).unwrap();
        assert!(hub.attributes.extension.is_some());
        let ext = hub.attributes.extension.unwrap();
        assert_eq!(
            ext.extension_type,
            Some("hubs:autodesk.bim360:Account".to_string())
        );
    }

    #[test]
    fn test_folder_content_folder_variant() {
        let json = r#"{
            "type": "folders",
            "id": "folder-id",
            "attributes": {
                "name": "Test Folder"
            }
        }"#;

        let content: FolderContent = serde_json::from_str(json).unwrap();
        match content {
            FolderContent::Folder(f) => assert_eq!(f.attributes.name, "Test Folder"),
            FolderContent::Item(_) => panic!("Expected folder"),
        }
    }

    #[test]
    fn test_folder_content_item_variant() {
        let json = r#"{
            "type": "items",
            "id": "item-id",
            "attributes": {
                "displayName": "model.rvt"
            }
        }"#;

        let content: FolderContent = serde_json::from_str(json).unwrap();
        match content {
            FolderContent::Item(i) => assert_eq!(i.attributes.display_name, "model.rvt"),
            FolderContent::Folder(_) => panic!("Expected item"),
        }
    }

    #[test]
    fn test_folder_with_timestamps() {
        let json = r#"{
            "type": "folders",
            "id": "folder-id",
            "attributes": {
                "name": "Timestamped Folder",
                "createTime": "2024-01-15T10:00:00Z",
                "lastModifiedTime": "2024-01-16T15:30:00Z"
            }
        }"#;

        let folder: Folder = serde_json::from_str(json).unwrap();
        assert_eq!(
            folder.attributes.create_time,
            Some("2024-01-15T10:00:00Z".to_string())
        );
        assert_eq!(
            folder.attributes.last_modified_time,
            Some("2024-01-16T15:30:00Z".to_string())
        );
    }

    #[test]
    fn test_item_with_timestamps() {
        let json = r#"{
            "type": "items",
            "id": "item-id",
            "attributes": {
                "displayName": "model.rvt",
                "createTime": "2024-01-10T08:00:00Z",
                "lastModifiedTime": "2024-01-12T12:00:00Z"
            }
        }"#;

        let item: Item = serde_json::from_str(json).unwrap();
        assert!(item.attributes.create_time.is_some());
        assert!(item.attributes.last_modified_time.is_some());
    }

    #[test]
    fn test_version_with_create_time() {
        let json = r#"{
            "type": "versions",
            "id": "version-id",
            "attributes": {
                "name": "v1",
                "createTime": "2024-01-15T10:00:00Z"
            }
        }"#;

        let version: Version = serde_json::from_str(json).unwrap();
        assert_eq!(
            version.attributes.create_time,
            Some("2024-01-15T10:00:00Z".to_string())
        );
    }

    #[test]
    fn test_pagination_follows_next_links() {
        // Simulate 3 pages of JSON:API responses with links.next
        let page1_json = r#"{
            "data": [{"type": "projects", "id": "p1", "attributes": {"name": "P1"}}],
            "links": {"next": {"href": "https://api.example.com/projects?page=2"}}
        }"#;
        let page2_json = r#"{
            "data": [{"type": "projects", "id": "p2", "attributes": {"name": "P2"}}],
            "links": {"next": "https://api.example.com/projects?page=3"}
        }"#;
        let page3_json = r#"{
            "data": [{"type": "projects", "id": "p3", "attributes": {"name": "P3"}}],
            "links": {"self": "https://api.example.com/projects?page=3"}
        }"#;

        let r1: JsonApiResponse<Vec<Project>> = serde_json::from_str(page1_json).unwrap();
        let r2: JsonApiResponse<Vec<Project>> = serde_json::from_str(page2_json).unwrap();
        let r3: JsonApiResponse<Vec<Project>> = serde_json::from_str(page3_json).unwrap();

        // Accumulate items following next links (simulating the pagination loop)
        let mut all_items = Vec::new();
        let pages = [r1, r2, r3];
        let mut next_url: Option<String> = Some("start".to_string());

        for page in &pages {
            if next_url.is_none() {
                break;
            }
            all_items.extend(page.data.iter().map(|p| p.id.clone()));
            next_url = page
                .links
                .as_ref()
                .and_then(|l| l.next.as_ref())
                .map(|link| link.href().to_string());
        }

        assert_eq!(all_items.len(), 3);
        assert_eq!(all_items, vec!["p1", "p2", "p3"]);
    }

    #[test]
    fn test_pagination_stops_when_no_next() {
        let json = r#"{
            "data": [{"type": "projects", "id": "p1", "attributes": {"name": "P1"}}],
            "links": {"self": "https://api.example.com/projects"}
        }"#;

        let response: JsonApiResponse<Vec<Project>> = serde_json::from_str(json).unwrap();
        let next = response
            .links
            .and_then(|l| l.next)
            .map(|link| link.href().to_string());
        assert!(next.is_none(), "Should stop when no next link present");
    }

    #[test]
    fn test_pagination_continues_on_empty_page() {
        // Page with zero items but links.next still present — pagination must continue
        let json = r#"{
            "data": [],
            "links": {"next": {"href": "https://api.example.com/projects?page=3"}}
        }"#;

        let response: JsonApiResponse<Vec<Project>> = serde_json::from_str(json).unwrap();
        assert!(response.data.is_empty(), "Page should have zero items");
        let next = response
            .links
            .and_then(|l| l.next)
            .map(|link| link.href().to_string());
        assert!(
            next.is_some(),
            "Should continue when next link is present even with empty data"
        );
        assert_eq!(next.unwrap(), "https://api.example.com/projects?page=3");
    }

    #[test]
    fn test_pagination_max_page_cap() {
        assert_eq!(MAX_PAGINATION_PAGES, 100, "Safety cap should be 100 pages");
    }

    #[test]
    fn test_json_api_link_href_simple() {
        let link = JsonApiLink::Simple("https://example.com/page".to_string());
        assert_eq!(link.href(), "https://example.com/page");
    }

    #[test]
    fn test_json_api_link_href_complex() {
        let link = JsonApiLink::Complex {
            href: "https://example.com/page2".to_string(),
        };
        assert_eq!(link.href(), "https://example.com/page2");
    }
}

/// Integration tests using raps-mock
///
/// DM uses 3-legged OAuth (`get_3leg_token()`). Tests set up a `StoredToken`
/// via `set_3leg_token_for_testing` to provide a valid mock token.
/// Some endpoints use client methods directly (where mock responses match type contracts),
/// others use raw HTTP where the mock returns simplified responses.
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::http::HttpClientConfig;
    use raps_kernel::types::StoredToken;

    fn create_mock_dm_client(mock_url: &str) -> DataManagementClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DataManagementClient::new(config, auth)
    }

    /// Acquire a mock token from the mock server and set it as a 3-legged token
    /// on the DM client.
    async fn acquire_mock_3leg_token(client: &DataManagementClient, mock_url: &str) -> String {
        let http = reqwest::Client::new();
        let resp = http
            .post(format!("{}/authentication/v2/token", mock_url))
            .json(&serde_json::json!({
                "grant_type": "client_credentials",
                "client_id": "test-client-id",
                "client_secret": "test-client-secret"
            }))
            .send()
            .await
            .expect("token request failed");
        assert!(
            resp.status().is_success(),
            "token endpoint returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.expect("invalid token JSON");
        let access_token = body["access_token"]
            .as_str()
            .expect("no access_token")
            .to_string();

        // Set as 3-legged token (DM uses get_3leg_token())
        let token = StoredToken {
            access_token: access_token.clone(),
            refresh_token: None,
            expires_at: chrono::Utc::now().timestamp() + 3600,
            scopes: vec!["data:read".to_string(), "data:write".to_string()],
        };
        client.auth.set_3leg_token_for_testing(token).await;

        access_token
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_list_hubs_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        acquire_mock_3leg_token(&client, &server.url).await;

        let result = client.list_hubs().await;
        assert!(result.is_ok(), "list_hubs failed: {:?}", result.err());
        let hubs = result.unwrap();
        assert!(!hubs.is_empty(), "hubs list should not be empty");
        let first = &hubs[0];
        assert!(!first.id.is_empty(), "hub id should not be empty");
        assert!(!first.attributes.name.is_empty(), "hub name should not be empty");
    }

    #[tokio::test]
    async fn test_get_hub_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty(), "need at least one hub");
        let hub_id = &hubs[0].id;

        let result = client.get_hub(hub_id).await;
        assert!(result.is_ok(), "get_hub failed: {:?}", result.err());
        let hub = result.unwrap();
        assert_eq!(hub.id, *hub_id);
        assert!(!hub.attributes.name.is_empty(), "hub name should not be empty");
    }

    #[tokio::test]
    async fn test_list_projects_with_mock() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty());
        let hub_id = &hubs[0].id;

        let result = client.list_projects(hub_id).await;
        assert!(result.is_ok(), "list_projects failed: {:?}", result.err());
        let projects = result.unwrap();
        assert!(!projects.is_empty(), "projects list should not be empty");
        let first = &projects[0];
        assert!(!first.id.is_empty(), "project id should not be empty");
        assert!(!first.attributes.name.is_empty(), "project name should not be empty");
    }

    #[tokio::test]
    async fn test_get_project_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty());
        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        assert!(!projects.is_empty());
        let project_id = &projects[0].id;

        // Use raw HTTP to verify the get_project endpoint
        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/project/v1/hubs/{}/projects/{}",
                server.url, hub_id, project_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get project returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(body["data"]["type"], "projects");
        assert_eq!(body["data"]["id"], project_id.as_str());
    }

    #[tokio::test]
    async fn test_get_top_folders_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty());
        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        assert!(!projects.is_empty());
        let project_id = &projects[0].id;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/project/v1/hubs/{}/projects/{}/topFolders",
                server.url, hub_id, project_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "get top folders returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["data"].is_array(), "data should be an array");
    }

    #[tokio::test]
    async fn test_list_folder_contents_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty());
        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        assert!(!projects.is_empty());
        let project_id = &projects[0].id;

        // Get top folders via raw HTTP
        let http = reqwest::Client::new();
        let folders_resp = http
            .get(format!(
                "{}/project/v1/hubs/{}/projects/{}/topFolders",
                server.url, hub_id, project_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        let folders_body: serde_json::Value = folders_resp.json().await.unwrap();
        let folders_data = folders_body["data"].as_array().unwrap();
        if folders_data.is_empty() {
            // No folders seeded, just verify endpoint accepts request
            return;
        }
        let folder_id = folders_data[0]["id"].as_str().unwrap();

        let resp = http
            .get(format!(
                "{}/data/v1/projects/{}/folders/{}/contents",
                server.url, project_id, folder_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list folder contents returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_create_folder_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        let project_id = &projects[0].id;

        let http = reqwest::Client::new();
        let resp = http
            .post(format!(
                "{}/data/v1/projects/{}/folders",
                server.url, project_id
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "jsonapi": {"version": "1.0"},
                "data": {
                    "type": "folders",
                    "attributes": {
                        "name": "Test Subfolder",
                        "extension": {
                            "type": "folders:autodesk.bim360:Folder",
                            "version": "1.0"
                        }
                    },
                    "relationships": {
                        "parent": {
                            "data": {
                                "type": "folders",
                                "id": "parent-folder-id"
                            }
                        }
                    }
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 201,
            "create folder returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["data"]["id"].is_string(), "created folder should have an id");
    }

    #[tokio::test]
    async fn test_rename_folder_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        // Navigate to get a real folder from mock seeded data
        let hubs = client.list_hubs().await.expect("list_hubs failed");
        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        let project_id = &projects[0].id;

        // First create a folder so we have one to rename
        let http = reqwest::Client::new();
        let create_resp = http
            .post(format!(
                "{}/data/v1/projects/{}/folders",
                server.url, project_id
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "jsonapi": {"version": "1.0"},
                "data": {
                    "type": "folders",
                    "attributes": {
                        "name": "Rename Me",
                        "extension": {
                            "type": "folders:autodesk.bim360:Folder",
                            "version": "1.0"
                        }
                    },
                    "relationships": {
                        "parent": {
                            "data": {
                                "type": "folders",
                                "id": "parent-folder-id"
                            }
                        }
                    }
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(create_resp.status().is_success() || create_resp.status().as_u16() == 201);
        let create_body: serde_json::Value = create_resp.json().await.unwrap();
        let folder_id = create_body["data"]["id"].as_str().expect("need folder id");

        let resp = http
            .patch(format!(
                "{}/data/v1/projects/{}/folders/{}",
                server.url, project_id, folder_id
            ))
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "jsonapi": {"version": "1.0"},
                "data": {
                    "type": "folders",
                    "id": folder_id,
                    "attributes": {
                        "name": "New Name"
                    }
                }
            }))
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "rename folder returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_delete_folder_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .delete(format!(
                "{}/data/v1/projects/b.project-123/folders/folder-456",
                server.url
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success() || resp.status().as_u16() == 204,
            "delete folder returned {}",
            resp.status()
        );
    }

    #[tokio::test]
    async fn test_hub_list_projects_list_navigation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        acquire_mock_3leg_token(&client, &server.url).await;

        // Navigate: hubs -> projects
        let hubs = client.list_hubs().await.expect("list_hubs failed");
        assert!(!hubs.is_empty(), "should have hubs");

        for hub in &hubs {
            assert_eq!(hub.hub_type, "hubs");
        }

        let hub_id = &hubs[0].id;
        let projects = client.list_projects(hub_id).await.expect("list_projects failed");
        assert!(!projects.is_empty(), "should have projects");

        for project in &projects {
            assert_eq!(project.project_type, "projects");
        }
    }

    #[tokio::test]
    async fn test_list_hubs_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!("{}/project/v1/hubs", server.url))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list hubs returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["data"].is_array(), "data should be an array");
        let hubs = body["data"].as_array().unwrap();
        assert!(!hubs.is_empty(), "hubs should not be empty");
        assert_eq!(hubs[0]["type"], "hubs");
    }

    #[tokio::test]
    async fn test_list_projects_raw_http() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let token = acquire_mock_3leg_token(&client, &server.url).await;

        let hubs = client.list_hubs().await.expect("list_hubs failed");
        let hub_id = &hubs[0].id;

        let http = reqwest::Client::new();
        let resp = http
            .get(format!(
                "{}/project/v1/hubs/{}/projects",
                server.url, hub_id
            ))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "list projects returned {}",
            resp.status()
        );
        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(body["data"].is_array(), "data should be an array");
    }
}
