// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

//! Reality Capture API module
//!
//! Handles photogrammetry processing to create 3D models from photos.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// Photoscene information
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Photoscene {
    #[serde(rename = "photosceneid")]
    pub photoscene_id: String,
    pub name: Option<String>,
    #[serde(rename = "scenetype")]
    pub scene_type: Option<String>,
    #[serde(rename = "convertformat")]
    pub convert_format: Option<String>,
    pub status: Option<String>,
    pub progress: Option<String>,
    #[serde(rename = "progressmsg")]
    pub progress_msg: Option<String>,
}

/// Photoscene creation response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CreatePhotosceneResponse {
    pub photoscene: Photoscene,
}

/// Upload response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UploadResponse {
    pub files: Option<UploadFiles>,
    pub usage: Option<String>,
    pub resource: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadFiles {
    pub file: Option<Vec<UploadedFile>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFile {
    pub filename: String,
    pub fileid: String,
    pub filesize: Option<String>,
    pub msg: Option<String>,
}

/// Progress response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProgressResponse {
    pub photoscene: PhotosceneProgress,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotosceneProgress {
    #[serde(rename = "photosceneid")]
    pub photoscene_id: String,
    pub progress: String,
    #[serde(rename = "progressmsg")]
    pub progress_msg: Option<String>,
    pub status: Option<String>,
}

/// Result response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ResultResponse {
    pub photoscene: PhotosceneResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotosceneResult {
    #[serde(rename = "photosceneid")]
    pub photoscene_id: String,
    pub progress: String,
    #[serde(rename = "progressmsg")]
    pub progress_msg: Option<String>,
    #[serde(rename = "scenelink")]
    pub scene_link: Option<String>,
    #[serde(rename = "filesize")]
    pub file_size: Option<String>,
}

/// Supported output formats
#[derive(Debug, Clone, Copy)]
pub enum OutputFormat {
    Rcm,   // Autodesk ReCap format
    Rcs,   // ReCap scan
    Obj,   // Wavefront OBJ
    Fbx,   // Autodesk FBX
    Ortho, // Orthophoto
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Rcm => write!(f, "rcm"),
            OutputFormat::Rcs => write!(f, "rcs"),
            OutputFormat::Obj => write!(f, "obj"),
            OutputFormat::Fbx => write!(f, "fbx"),
            OutputFormat::Ortho => write!(f, "ortho"),
        }
    }
}

impl OutputFormat {
    pub fn all() -> Vec<Self> {
        vec![Self::Rcm, Self::Rcs, Self::Obj, Self::Fbx, Self::Ortho]
    }

    pub fn description(&self) -> &str {
        match self {
            OutputFormat::Rcm => "Autodesk ReCap format (point cloud)",
            OutputFormat::Rcs => "ReCap scan format",
            OutputFormat::Obj => "Wavefront OBJ (mesh)",
            OutputFormat::Fbx => "Autodesk FBX (mesh)",
            OutputFormat::Ortho => "Orthophoto (2D image)",
        }
    }
}

/// Scene type for photoscene
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SceneType {
    Aerial,
    Object,
}

impl std::fmt::Display for SceneType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SceneType::Aerial => write!(f, "aerial"),
            SceneType::Object => write!(f, "object"),
        }
    }
}

/// Reality Capture API client
pub struct RealityCaptureClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl RealityCaptureClient {
    /// Create a new Reality Capture client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Reality Capture client with custom HTTP config
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

    /// Create a new photoscene
    pub async fn create_photoscene(
        &self,
        name: &str,
        scene_type: SceneType,
        format: OutputFormat,
    ) -> Result<Photoscene> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/photoscene", self.config.reality_capture_url());

        let params = [
            ("scenename", name),
            ("scenetype", &scene_type.to_string()),
            ("format", &format.to_string()),
        ];

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .form(&params)
            .send()
            .await
            .context("Failed to create photoscene")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create photoscene ({status}): {error_text}");
        }

        let create_response: CreatePhotosceneResponse = response
            .json()
            .await
            .context("Failed to parse photoscene response")?;

        Ok(create_response.photoscene)
    }

    /// Upload photos to a photoscene
    pub async fn upload_photos(
        &self,
        photoscene_id: &str,
        photo_paths: &[&Path],
    ) -> Result<Vec<UploadedFile>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/file", self.config.reality_capture_url());

        let mut form = reqwest::multipart::Form::new()
            .text("photosceneid", photoscene_id.to_string())
            .text("type", "image");

        for (i, path) in photo_paths.iter().enumerate() {
            let mut file = File::open(path)
                .await
                .context(format!("Failed to open file: {}", path.display()))?;

            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .await
                .context("Failed to read file")?;

            let filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("photo.jpg")
                .to_string();

            let part = reqwest::multipart::Part::bytes(buffer)
                .file_name(filename.clone())
                .mime_str("image/jpeg")?;

            form = form.part(format!("file[{}]", i), part);
        }

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await
            .context("Failed to upload photos")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upload photos ({status}): {error_text}");
        }

        let upload_response: UploadResponse = response
            .json()
            .await
            .context("Failed to parse upload response")?;

        let files = upload_response
            .files
            .and_then(|f| f.file)
            .unwrap_or_default();

        Ok(files)
    }

    /// Start processing a photoscene
    pub async fn start_processing(&self, photoscene_id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/photoscene/{}",
            self.config.reality_capture_url(),
            photoscene_id
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to start processing")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start processing ({status}): {error_text}");
        }

        Ok(())
    }

    /// Get photoscene progress
    pub async fn get_progress(&self, photoscene_id: &str) -> Result<PhotosceneProgress> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/photoscene/{}/progress",
            self.config.reality_capture_url(),
            photoscene_id
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get progress")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get progress ({status}): {error_text}");
        }

        let progress_response: ProgressResponse = response
            .json()
            .await
            .context("Failed to parse progress response")?;

        Ok(progress_response.photoscene)
    }

    /// Get photoscene result (download link)
    pub async fn get_result(
        &self,
        photoscene_id: &str,
        format: OutputFormat,
    ) -> Result<PhotosceneResult> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/photoscene/{}?format={}",
            self.config.reality_capture_url(),
            photoscene_id,
            format
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get result")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get result ({status}): {error_text}");
        }

        let result_response: ResultResponse = response
            .json()
            .await
            .context("Failed to parse result response")?;

        Ok(result_response.photoscene)
    }

    /// Delete a photoscene
    pub async fn delete_photoscene(&self, photoscene_id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/photoscene/{}",
            self.config.reality_capture_url(),
            photoscene_id
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete photoscene")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete photoscene ({status}): {error_text}");
        }

        Ok(())
    }

    /// Get available output formats
    pub fn available_formats(&self) -> Vec<OutputFormat> {
        OutputFormat::all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_all() {
        let formats = OutputFormat::all();
        assert_eq!(formats.len(), 5);
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Rcm.to_string(), "rcm");
        assert_eq!(OutputFormat::Rcs.to_string(), "rcs");
        assert_eq!(OutputFormat::Obj.to_string(), "obj");
        assert_eq!(OutputFormat::Fbx.to_string(), "fbx");
        assert_eq!(OutputFormat::Ortho.to_string(), "ortho");
    }

    #[test]
    fn test_output_format_description() {
        assert!(!OutputFormat::Rcm.description().is_empty());
        assert!(OutputFormat::Rcm.description().contains("ReCap"));
        assert!(OutputFormat::Obj.description().contains("OBJ"));
    }

    #[test]
    fn test_scene_type_display() {
        assert_eq!(SceneType::Aerial.to_string(), "aerial");
        assert_eq!(SceneType::Object.to_string(), "object");
    }

    #[test]
    fn test_scene_type_serialization() {
        assert_eq!(
            serde_json::to_string(&SceneType::Aerial).unwrap(),
            "\"aerial\""
        );
        assert_eq!(
            serde_json::to_string(&SceneType::Object).unwrap(),
            "\"object\""
        );
    }

    #[test]
    fn test_photoscene_deserialization() {
        let json = r#"{
            "photosceneid": "scene-123",
            "name": "Test Scene",
            "scenetype": "object",
            "convertformat": "rcm",
            "status": "Created",
            "progress": "0"
        }"#;

        let scene: Photoscene = serde_json::from_str(json).unwrap();
        assert_eq!(scene.photoscene_id, "scene-123");
        assert_eq!(scene.name, Some("Test Scene".to_string()));
    }

    #[test]
    fn test_photoscene_progress_deserialization() {
        let json = r#"{
            "photosceneid": "scene-123",
            "progress": "50",
            "progressmsg": "Processing images"
        }"#;

        let progress: PhotosceneProgress = serde_json::from_str(json).unwrap();
        assert_eq!(progress.photoscene_id, "scene-123");
        assert_eq!(progress.progress, "50");
    }

    #[test]
    fn test_photoscene_result_deserialization() {
        let json = r#"{
            "photosceneid": "scene-123",
            "progress": "100",
            "progressmsg": "Complete",
            "filesize": "5242880",
            "scenelink": "https://example.com/download/scene.rcm"
        }"#;

        let result: PhotosceneResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.photoscene_id, "scene-123");
        assert!(result.scene_link.is_some());
    }

    #[test]
    fn test_create_photoscene_response_deserialization() {
        let json = r#"{
            "Photoscene": {
                "photosceneid": "new-scene-456",
                "name": "New Scene"
            }
        }"#;

        let response: CreatePhotosceneResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.photoscene.photoscene_id, "new-scene-456");
    }
}

/// Integration tests using wiremock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use wiremock::matchers::{header, method, path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Create a Reality Capture client configured to use the mock server
    fn create_mock_reality_client(mock_url: &str) -> RealityCaptureClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        RealityCaptureClient::new(config, auth)
    }

    /// Setup mock for 2-legged auth token
    async fn setup_auth_mock(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/authentication/v2/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "test-token-12345",
                "token_type": "Bearer",
                "expires_in": 3600
            })))
            .mount(server)
            .await;
    }

    // ==================== Create Photoscene ====================

    #[tokio::test]
    async fn test_create_photoscene_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/photo-to-3d/v1/photoscene"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "name": "Test Scene",
                    "scenetype": "object",
                    "convertformat": "rcm"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client
            .create_photoscene("Test Scene", SceneType::Object, OutputFormat::Rcm)
            .await;

        assert!(result.is_ok());
        let scene = result.unwrap();
        assert_eq!(scene.photoscene_id, "scene-123");
        assert_eq!(scene.name, Some("Test Scene".to_string()));
    }

    #[tokio::test]
    async fn test_create_photoscene_aerial() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/photo-to-3d/v1/photoscene"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "aerial-scene-456",
                    "name": "Aerial Scene",
                    "scenetype": "aerial",
                    "convertformat": "obj"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client
            .create_photoscene("Aerial Scene", SceneType::Aerial, OutputFormat::Obj)
            .await;

        assert!(result.is_ok());
        let scene = result.unwrap();
        assert_eq!(scene.photoscene_id, "aerial-scene-456");
    }

    #[tokio::test]
    async fn test_create_photoscene_unauthorized() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/photo-to-3d/v1/photoscene"))
            .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
                "Error": {
                    "code": "Unauthorized",
                    "msg": "Invalid token"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client
            .create_photoscene("Test", SceneType::Object, OutputFormat::Rcm)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("401"));
    }

    // ==================== Start Processing ====================

    #[tokio::test]
    async fn test_start_processing_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "status": "Processing"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.start_processing("scene-123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_processing_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "Error": {
                    "code": "NotFound",
                    "msg": "Photoscene not found"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.start_processing("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    #[tokio::test]
    async fn test_start_processing_no_photos() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "Error": {
                    "code": "BadRequest",
                    "msg": "No photos uploaded to photoscene"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.start_processing("empty-scene").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("400"));
    }

    // ==================== Get Progress ====================

    #[tokio::test]
    async fn test_get_progress_processing() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+/progress"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "progress": "50",
                    "progressmsg": "Processing images",
                    "status": "Processing"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_progress("scene-123").await;

        assert!(result.is_ok());
        let progress = result.unwrap();
        assert_eq!(progress.photoscene_id, "scene-123");
        assert_eq!(progress.progress, "50");
    }

    #[tokio::test]
    async fn test_get_progress_complete() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+/progress"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "progress": "100",
                    "progressmsg": "Complete",
                    "status": "Done"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_progress("scene-123").await;

        assert!(result.is_ok());
        let progress = result.unwrap();
        assert_eq!(progress.progress, "100");
    }

    #[tokio::test]
    async fn test_get_progress_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+/progress"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "Error": {
                    "code": "NotFound",
                    "msg": "Photoscene not found"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_progress("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    // ==================== Get Result ====================

    #[tokio::test]
    async fn test_get_result_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "progress": "100",
                    "progressmsg": "Complete",
                    "scenelink": "https://example.com/download/scene.rcm",
                    "filesize": "5242880"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_result("scene-123", OutputFormat::Rcm).await;

        assert!(result.is_ok());
        let scene_result = result.unwrap();
        assert_eq!(scene_result.photoscene_id, "scene-123");
        assert!(scene_result.scene_link.is_some());
        assert!(scene_result.file_size.is_some());
    }

    #[tokio::test]
    async fn test_get_result_still_processing() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Photoscene": {
                    "photosceneid": "scene-123",
                    "progress": "75",
                    "progressmsg": "Still processing"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_result("scene-123", OutputFormat::Rcm).await;

        assert!(result.is_ok());
        let scene_result = result.unwrap();
        // No download link while still processing
        assert!(scene_result.scene_link.is_none());
    }

    // ==================== Delete Photoscene ====================

    #[tokio::test]
    async fn test_delete_photoscene_success() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .and(header("Authorization", "Bearer test-token-12345"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "msg": "Photoscene deleted"
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.delete_photoscene("scene-123").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_photoscene_not_found() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("DELETE"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "Error": {
                    "code": "NotFound",
                    "msg": "Photoscene not found"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.delete_photoscene("nonexistent").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("404"));
    }

    // ==================== Error Handling ====================

    #[tokio::test]
    async fn test_server_error() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("POST"))
            .and(path("/photo-to-3d/v1/photoscene"))
            .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
                "Error": {
                    "code": "InternalError",
                    "msg": "Internal server error"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client
            .create_photoscene("Test", SceneType::Object, OutputFormat::Rcm)
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("500"));
    }

    #[tokio::test]
    async fn test_rate_limit() {
        let server = MockServer::start().await;
        setup_auth_mock(&server).await;

        Mock::given(method("GET"))
            .and(path_regex(r"/photo-to-3d/v1/photoscene/.+/progress"))
            .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "Error": {
                    "code": "RateLimited",
                    "msg": "Too many requests"
                }
            })))
            .mount(&server)
            .await;

        let client = create_mock_reality_client(&server.uri());
        let result = client.get_progress("scene-123").await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("429"));
    }
}
