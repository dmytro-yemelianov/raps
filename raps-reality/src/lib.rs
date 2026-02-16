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
use raps_kernel::http::{self, HttpClientConfig};

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

/// List photoscenes response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ListPhotoscenesResponse {
    #[serde(default)]
    pub photoscenes: PhotoscenesList,
}

#[derive(Debug, Default, Deserialize)]
pub struct PhotoscenesList {
    #[serde(default)]
    pub photoscene: Vec<Photoscene>,
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

/// API-level error returned with HTTP 200 (documented Reality Capture API quirk)
#[derive(Debug, Deserialize)]
pub struct RcApiError {
    pub code: Option<String>,
    #[serde(alias = "message")]
    pub msg: Option<String>,
}

/// Progress response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProgressResponse {
    pub photoscene: Option<PhotosceneProgress>,
    pub error: Option<RcApiError>,
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
    pub photoscene: Option<PhotosceneResult>,
    pub error: Option<RcApiError>,
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .form(&params)
        })
        .await?;

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

        // Read all files into memory so the retry closure can rebuild the multipart form
        let mut file_parts: Vec<(String, Vec<u8>)> = Vec::new();
        for path in photo_paths {
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

            file_parts.push((filename, buffer));
        }

        let photoscene_id_owned = photoscene_id.to_string();
        let response = http::send_with_retry(&self.config.http_config, || {
            let mut form = reqwest::multipart::Form::new()
                .text("photosceneid", photoscene_id_owned.clone())
                .text("type", "image");

            for (i, (filename, buffer)) in file_parts.iter().enumerate() {
                let part = reqwest::multipart::Part::bytes(buffer.clone())
                    .file_name(filename.clone())
                    .mime_str("image/jpeg")
                    .expect("valid MIME type");

                form = form.part(format!("file[{}]", i), part);
            }

            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .multipart(form)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.post(&url).bearer_auth(&token)
        })
        .await?;

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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get progress ({status}): {error_text}");
        }

        let progress_response: ProgressResponse = response
            .json()
            .await
            .context("Failed to parse progress response")?;

        if let Some(err) = progress_response.error {
            let code = err.code.unwrap_or_default();
            let msg = err.msg.unwrap_or_default();
            anyhow::bail!("Reality Capture API error ({code}): {msg}");
        }

        progress_response
            .photoscene
            .ok_or_else(|| anyhow::anyhow!("Progress response missing Photoscene data"))
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

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get result ({status}): {error_text}");
        }

        let result_response: ResultResponse = response
            .json()
            .await
            .context("Failed to parse result response")?;

        if let Some(err) = result_response.error {
            let code = err.code.unwrap_or_default();
            let msg = err.msg.unwrap_or_default();
            anyhow::bail!("Reality Capture API error ({code}): {msg}");
        }

        result_response
            .photoscene
            .ok_or_else(|| anyhow::anyhow!("Result response missing Photoscene data"))
    }

    /// Delete a photoscene
    pub async fn delete_photoscene(&self, photoscene_id: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/photoscene/{}",
            self.config.reality_capture_url(),
            photoscene_id
        );

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete photoscene ({status}): {error_text}");
        }

        Ok(())
    }

    /// List all photoscenes
    pub async fn list_photoscenes(&self) -> Result<Vec<Photoscene>> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/photoscene", self.config.reality_capture_url());

        let response = http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to list photoscenes ({status}): {error_text}");
        }

        let list_response: ListPhotoscenesResponse = response
            .json()
            .await
            .context("Failed to parse photoscenes response")?;

        Ok(list_response.photoscenes.photoscene)
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

    #[test]
    fn test_list_photoscenes_response_deserialization() {
        let json = r#"{
            "Photoscenes": {
                "photoscene": [
                    {
                        "photosceneid": "scene-1",
                        "name": "Scene One",
                        "scenetype": "aerial",
                        "status": "Complete",
                        "progress": "100"
                    },
                    {
                        "photosceneid": "scene-2",
                        "name": "Scene Two",
                        "scenetype": "object",
                        "status": "Created",
                        "progress": "0"
                    }
                ]
            }
        }"#;

        let response: ListPhotoscenesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.photoscenes.photoscene.len(), 2);
        assert_eq!(response.photoscenes.photoscene[0].photoscene_id, "scene-1");
        assert_eq!(response.photoscenes.photoscene[1].photoscene_id, "scene-2");
        assert_eq!(
            response.photoscenes.photoscene[0].name,
            Some("Scene One".to_string())
        );
    }

    #[test]
    fn test_list_photoscenes_response_empty() {
        let json = r#"{
            "Photoscenes": {
                "photoscene": []
            }
        }"#;

        let response: ListPhotoscenesResponse = serde_json::from_str(json).unwrap();
        assert!(response.photoscenes.photoscene.is_empty());
    }

    #[test]
    fn test_list_photoscenes_response_missing_photoscenes() {
        let json = r#"{}"#;

        let response: ListPhotoscenesResponse = serde_json::from_str(json).unwrap();
        assert!(response.photoscenes.photoscene.is_empty());
    }

    #[test]
    fn test_upload_response_deserialization() {
        let json = r#"{
            "Files": {
                "file": [
                    {
                        "filename": "photo1.jpg",
                        "fileid": "file-001",
                        "filesize": "2048000",
                        "msg": "File uploaded"
                    }
                ]
            },
            "Usage": "1",
            "Resource": "/file"
        }"#;

        let response: UploadResponse = serde_json::from_str(json).unwrap();
        assert!(response.files.is_some());
        let files = response.files.unwrap().file.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].filename, "photo1.jpg");
        assert_eq!(files[0].fileid, "file-001");
    }

    #[test]
    fn test_progress_response_deserialization() {
        let json = r#"{
            "Photoscene": {
                "photosceneid": "scene-789",
                "progress": "75",
                "progressmsg": "Processing images",
                "status": "InProgress"
            }
        }"#;

        let response: ProgressResponse = serde_json::from_str(json).unwrap();
        let ps = response.photoscene.unwrap();
        assert_eq!(ps.photoscene_id, "scene-789");
        assert_eq!(ps.progress, "75");
        assert_eq!(ps.progress_msg, Some("Processing images".to_string()));
    }

    #[test]
    fn test_progress_response_with_error() {
        let json = r#"{
            "Usage": "0.51",
            "Resource": "/photoscene/xyz/progress",
            "Error": {
                "code": "ERR-001",
                "msg": "Scene not found"
            }
        }"#;

        let response: ProgressResponse = serde_json::from_str(json).unwrap();
        assert!(response.photoscene.is_none());
        assert!(response.error.is_some());
        let err = response.error.unwrap();
        assert_eq!(err.code.unwrap(), "ERR-001");
        assert_eq!(err.msg.unwrap(), "Scene not found");
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

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

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_reality_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }

    #[tokio::test]
    async fn test_list_photoscenes() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_reality_client(&server.url);
        let result = client.list_photoscenes().await;
        let _ = result;
    }
}
