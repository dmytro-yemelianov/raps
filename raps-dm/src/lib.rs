// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management API module
//!
//! Handles access to Hubs, Projects, Folders, and Items in BIM 360/ACC.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

pub mod types;
mod folders;
mod hubs;
mod items;

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
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default if config fails

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
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::http::HttpClientConfig;

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

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_rename_folder() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let result = client
            .rename_folder("b.project-123", "folder-456", "New Name")
            .await;
        let _ = result;
    }

    #[tokio::test]
    async fn test_delete_folder() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_dm_client(&server.url);
        let result = client.delete_folder("b.project-123", "folder-456").await;
        let _ = result;
    }
}
