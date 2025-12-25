//! Model Derivative API module
//!
//! Handles translation of CAD files and retrieval of derivative manifests.
//! Supports downloading translated derivatives directly from manifest.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use super::AuthClient;
use crate::config::Config;

/// Supported output formats for translation
#[derive(Debug, Clone, Copy, Serialize)]
pub enum OutputFormat {
    /// Streaming format for Viewer (recommended)
    #[serde(rename = "svf2")]
    Svf2,
    /// Legacy streaming format
    #[serde(rename = "svf")]
    Svf,
    /// Thumbnail images
    #[serde(rename = "thumbnail")]
    Thumbnail,
    /// OBJ format (mesh export)
    #[serde(rename = "obj")]
    Obj,
    /// STL format (3D printing)
    #[serde(rename = "stl")]
    Stl,
    /// STEP format (CAD interchange)
    #[serde(rename = "step")]
    Step,
    /// IGES format (CAD interchange)
    #[serde(rename = "iges")]
    Iges,
    /// IFC format (BIM)
    #[serde(rename = "ifc")]
    Ifc,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Svf2 => write!(f, "SVF2 (Viewer)"),
            OutputFormat::Svf => write!(f, "SVF (Legacy Viewer)"),
            OutputFormat::Thumbnail => write!(f, "Thumbnail"),
            OutputFormat::Obj => write!(f, "OBJ (Mesh)"),
            OutputFormat::Stl => write!(f, "STL (3D Print)"),
            OutputFormat::Step => write!(f, "STEP (CAD)"),
            OutputFormat::Iges => write!(f, "IGES (CAD)"),
            OutputFormat::Ifc => write!(f, "IFC (BIM)"),
        }
    }
}

impl OutputFormat {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Svf2,
            Self::Svf,
            Self::Thumbnail,
            Self::Obj,
            Self::Stl,
            Self::Step,
            Self::Iges,
            Self::Ifc,
        ]
    }

    pub fn type_name(&self) -> &str {
        match self {
            OutputFormat::Svf2 => "svf2",
            OutputFormat::Svf => "svf",
            OutputFormat::Thumbnail => "thumbnail",
            OutputFormat::Obj => "obj",
            OutputFormat::Stl => "stl",
            OutputFormat::Step => "step",
            OutputFormat::Iges => "iges",
            OutputFormat::Ifc => "ifc",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "svf2" => Some(Self::Svf2),
            "svf" => Some(Self::Svf),
            "thumbnail" => Some(Self::Thumbnail),
            "obj" => Some(Self::Obj),
            "stl" => Some(Self::Stl),
            "step" => Some(Self::Step),
            "iges" => Some(Self::Iges),
            "ifc" => Some(Self::Ifc),
            _ => None,
        }
    }
}

/// Request to start a translation job
#[derive(Debug, Serialize)]
pub struct TranslationRequest {
    pub input: TranslationInput,
    pub output: TranslationOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationInput {
    pub urn: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressed_urn: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_filename: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TranslationOutput {
    pub destination: OutputDestination,
    pub formats: Vec<OutputFormatSpec>,
}

#[derive(Debug, Serialize)]
pub struct OutputDestination {
    pub region: String,
}

#[derive(Debug, Serialize)]
pub struct OutputFormatSpec {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<Vec<String>>,
}

/// Translation job response
#[derive(Debug, Deserialize)]
pub struct TranslationResponse {
    pub result: String,
    pub urn: String,
    #[serde(rename = "acceptedJobs")]
    pub accepted_jobs: Option<AcceptedJobs>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptedJobs {
    pub output: OutputJobInfo,
}

#[derive(Debug, Deserialize)]
pub struct OutputJobInfo {
    pub formats: Vec<FormatJobInfo>,
}

#[derive(Debug, Deserialize)]
pub struct FormatJobInfo {
    #[serde(rename = "type")]
    pub format_type: String,
}

/// Manifest response (translation status and derivatives)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(rename = "type")]
    pub manifest_type: String,
    pub has_thumbnail: String,
    pub status: String,
    pub progress: String,
    pub region: String,
    pub urn: String,
    pub version: Option<String>,
    #[serde(default)]
    pub derivatives: Vec<Derivative>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Derivative {
    pub name: Option<String>,
    pub has_thumbnail: Option<String>,
    pub status: String,
    pub progress: Option<String>,
    pub output_type: String,
    #[serde(default)]
    pub children: Vec<DerivativeChild>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivativeChild {
    pub guid: String,
    #[serde(rename = "type")]
    pub child_type: String,
    pub role: String,
    pub name: Option<String>,
    pub status: Option<String>,
    /// URN for downloadable derivatives
    pub urn: Option<String>,
    /// MIME type for downloadable files
    pub mime: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
    #[serde(default)]
    pub children: Vec<DerivativeChild>,
}

/// Information about a downloadable derivative
#[derive(Debug, Clone, Serialize)]
pub struct DownloadableDerivative {
    pub guid: String,
    pub name: String,
    pub output_type: String,
    pub role: String,
    pub urn: String,
    pub mime: Option<String>,
    pub size: Option<u64>,
}

/// Model Derivative API client
pub struct DerivativeClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DerivativeClient {
    /// Create a new Model Derivative client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Model Derivative client with custom HTTP config
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

    /// Start a translation job
    pub async fn translate(
        &self,
        urn: &str,
        format: OutputFormat,
        root_filename: Option<&str>,
    ) -> Result<TranslationResponse> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/designdata/job", self.config.derivative_url());

        let request = TranslationRequest {
            input: TranslationInput {
                urn: urn.to_string(),
                compressed_urn: None,
                root_filename: root_filename.map(|s| s.to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: "us".to_string(),
                },
                formats: vec![OutputFormatSpec {
                    format_type: format.type_name().to_string(),
                    views: if matches!(format, OutputFormat::Svf2 | OutputFormat::Svf) {
                        Some(vec!["2d".to_string(), "3d".to_string()])
                    } else {
                        None
                    },
                }],
            },
        };

        // Log request in verbose/debug mode
        crate::logging::log_request("POST", &url);

        // Use retry logic for translation requests
        let http_config = crate::http::HttpClientConfig::default();
        let response = crate::http::execute_with_retry(&http_config, || {
            let client = self.http_client.clone();
            let url = url.clone();
            let token = token.clone();
            let request_json = serde_json::to_value(&request).ok();
            Box::pin(async move {
                let mut req = client
                    .post(&url)
                    .bearer_auth(&token)
                    .header("Content-Type", "application/json")
                    .header("x-ads-force", "true");
                if let Some(json) = request_json {
                    req = req.json(&json);
                }
                req.send().await.context("Failed to start translation")
            })
        })
        .await?;

        // Log response in verbose/debug mode
        crate::logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start translation ({}): {}", status, error_text);
        }

        let translation_response: TranslationResponse = response
            .json()
            .await
            .context("Failed to parse translation response")?;

        Ok(translation_response)
    }

    /// Get the manifest (translation status and available derivatives)
    pub async fn get_manifest(&self, urn: &str) -> Result<Manifest> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get manifest")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get manifest ({}): {}", status, error_text);
        }

        let manifest: Manifest = response
            .json()
            .await
            .context("Failed to parse manifest response")?;

        Ok(manifest)
    }

    /// Delete manifest (and all derivatives)
    #[allow(dead_code)]
    pub async fn delete_manifest(&self, urn: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete manifest")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete manifest ({}): {}", status, error_text);
        }

        Ok(())
    }

    /// Check translation status and return progress percentage
    pub async fn get_status(&self, urn: &str) -> Result<(String, String)> {
        let manifest = self.get_manifest(urn).await?;
        Ok((manifest.status, manifest.progress))
    }

    /// Get list of downloadable derivatives from manifest
    pub async fn list_downloadable_derivatives(
        &self,
        urn: &str,
    ) -> Result<Vec<DownloadableDerivative>> {
        let manifest = self.get_manifest(urn).await?;
        let mut downloadables = Vec::new();

        for derivative in &manifest.derivatives {
            self.collect_downloadables(derivative, &derivative.output_type, &mut downloadables);
        }

        Ok(downloadables)
    }

    /// Recursively collect downloadable items from derivative tree
    fn collect_downloadables(
        &self,
        derivative: &Derivative,
        output_type: &str,
        downloadables: &mut Vec<DownloadableDerivative>,
    ) {
        for child in &derivative.children {
            self.collect_downloadables_from_child(child, output_type, downloadables);
        }
    }

    /// Recursively collect downloadable items from child nodes
    fn collect_downloadables_from_child(
        &self,
        child: &DerivativeChild,
        output_type: &str,
        downloadables: &mut Vec<DownloadableDerivative>,
    ) {
        // Check if this child has a URN (is downloadable)
        if let Some(ref urn) = child.urn {
            let name = child.name.clone().unwrap_or_else(|| {
                // Generate name from GUID and type
                format!(
                    "{}.{}",
                    &child.guid[..8.min(child.guid.len())],
                    output_type.to_lowercase()
                )
            });

            downloadables.push(DownloadableDerivative {
                guid: child.guid.clone(),
                name,
                output_type: output_type.to_string(),
                role: child.role.clone(),
                urn: urn.clone(),
                mime: child.mime.clone(),
                size: child.size,
            });
        }

        // Recurse into children
        for grandchild in &child.children {
            self.collect_downloadables_from_child(grandchild, output_type, downloadables);
        }
    }

    /// Filter derivatives by format (output type)
    pub fn filter_by_format(
        derivatives: &[DownloadableDerivative],
        format: &str,
    ) -> Vec<DownloadableDerivative> {
        derivatives
            .iter()
            .filter(|d| d.output_type.to_lowercase() == format.to_lowercase())
            .cloned()
            .collect()
    }

    /// Filter derivatives by GUID
    pub fn filter_by_guid(
        derivatives: &[DownloadableDerivative],
        guid: &str,
    ) -> Option<DownloadableDerivative> {
        derivatives.iter().find(|d| d.guid == guid).cloned()
    }

    /// Download a derivative to a local file
    pub async fn download_derivative(
        &self,
        source_urn: &str,
        derivative_urn: &str,
        output_path: &Path,
    ) -> Result<u64> {
        let token = self.auth.get_token().await?;

        // The derivative URN needs to be URL-encoded
        let encoded_derivative_urn = urlencoding::encode(derivative_urn);
        let url = format!(
            "{}/designdata/{}/manifest/{}",
            self.config.derivative_url(),
            source_urn,
            encoded_derivative_urn
        );

        crate::logging::log_request("GET", &url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to download derivative")?;

        crate::logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to download derivative ({}): {}", status, error_text);
        }

        let total_size = response.content_length().unwrap_or(0);

        // Create progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("█▓░"),
        );

        let filename = output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("derivative");
        pb.set_message(format!("Downloading {}", filename));

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Stream download
        let mut file = File::create(output_path)
            .await
            .context("Failed to create output file")?;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error while downloading")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write to file")?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message(format!("Downloaded {}", filename));

        Ok(downloaded)
    }

    /// Download all derivatives matching a format
    pub async fn download_derivatives_by_format(
        &self,
        source_urn: &str,
        format: &str,
        output_dir: &Path,
    ) -> Result<Vec<(String, u64)>> {
        let downloadables = self.list_downloadable_derivatives(source_urn).await?;
        let filtered = Self::filter_by_format(&downloadables, format);

        if filtered.is_empty() {
            anyhow::bail!("No derivatives found with format '{}'", format);
        }

        let mut results = Vec::new();

        for derivative in filtered {
            let output_path = output_dir.join(&derivative.name);
            let size = self
                .download_derivative(source_urn, &derivative.urn, &output_path)
                .await?;
            results.push((derivative.name, size));
        }

        Ok(results)
    }
}
