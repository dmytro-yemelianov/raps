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

use super::AuthClient;
use crate::config::Config;

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
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Reality Capture client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: crate::http::HttpClientConfig,
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
            anyhow::bail!("Failed to create photoscene ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to upload photos ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to start processing ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to get progress ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to get result ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to delete photoscene ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// Get available output formats
    pub fn available_formats(&self) -> Vec<OutputFormat> {
        OutputFormat::all()
    }
}
