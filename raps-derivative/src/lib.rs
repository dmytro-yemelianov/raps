// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::uninlined_format_args)]
// Copyright 2024-2025 Dmytro Yemelianov

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
use std::{path::Path, str::FromStr};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// APS data center regions for Model Derivative service
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MdRegion {
    #[default]
    US,
    EMEA,
    AUS,
    CAN,
    DEU,
    IND,
    JPN,
    GBR,
}

impl std::fmt::Display for MdRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MdRegion::US => "US",
            MdRegion::EMEA => "EMEA",
            MdRegion::AUS => "AUS",
            MdRegion::CAN => "CAN",
            MdRegion::DEU => "DEU",
            MdRegion::IND => "IND",
            MdRegion::JPN => "JPN",
            MdRegion::GBR => "GBR",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for MdRegion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_uppercase().as_str() {
            "US" => Ok(MdRegion::US),
            "EMEA" => Ok(MdRegion::EMEA),
            "AUS" => Ok(MdRegion::AUS),
            "CAN" => Ok(MdRegion::CAN),
            "DEU" => Ok(MdRegion::DEU),
            "IND" => Ok(MdRegion::IND),
            "JPN" => Ok(MdRegion::JPN),
            "GBR" => Ok(MdRegion::GBR),
            _ => anyhow::bail!(
                "Invalid region '{}'. Valid values: US, EMEA, AUS, CAN, DEU, IND, JPN, GBR",
                s
            ),
        }
    }
}

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
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "svf2" => Ok(Self::Svf2),
            "svf" => Ok(Self::Svf),
            "thumbnail" => Ok(Self::Thumbnail),
            "obj" => Ok(Self::Obj),
            "stl" => Ok(Self::Stl),
            "step" => Ok(Self::Step),
            "iges" => Ok(Self::Iges),
            "ifc" => Ok(Self::Ifc),
            _ => Err(format!(
                "Invalid output format: {}. Use: {}",
                s,
                Self::all()
                    .iter()
                    .map(OutputFormat::type_name)
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
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

// ============== METADATA TYPES ==============

/// Response from GET /metadata — list model views/viewables
#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataResponse {
    pub data: MetadataData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub metadata: Vec<ModelView>,
}

/// A single view/viewable within a translated model
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub guid: String,
    pub name: String,
    pub role: String,
    #[serde(rename = "mime")]
    pub mime_type: Option<String>,
    pub has_thumbnail: Option<String>,
    pub progress: Option<String>,
}

/// Response from GET /metadata/{guid} — object tree hierarchy
#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectTreeResponse {
    pub data: ObjectTreeData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ObjectTreeData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub objects: Vec<ObjectTreeNode>,
}

/// A node in the model's object tree
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObjectTreeNode {
    #[serde(rename = "objectid")]
    pub object_id: i64,
    pub name: String,
    #[serde(default)]
    pub objects: Vec<ObjectTreeNode>,
}

/// Response from GET/POST /metadata/{guid}/properties
#[derive(Debug, Deserialize, Serialize)]
pub struct PropertiesResponse {
    pub data: PropertiesData,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PropertiesData {
    #[serde(rename = "type")]
    pub data_type: Option<String>,
    pub collection: Vec<PropertyObject>,
}

/// A single object's properties
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyObject {
    #[serde(rename = "objectid")]
    pub object_id: i64,
    pub name: String,
    pub external_id: Option<String>,
    #[serde(default)]
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

/// Request body for POST /metadata/{guid}/properties:query
#[derive(Debug, Serialize)]
pub struct PropertyQuery {
    pub query: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<PropertyPagination>,
}

impl PropertyQuery {
    /// Create a query filtering by object IDs
    pub fn by_object_ids(ids: Vec<i64>) -> Self {
        let mut filter: Vec<serde_json::Value> =
            vec![serde_json::Value::String("objectid".to_string())];
        filter.extend(
            ids.into_iter()
                .map(|id| serde_json::Value::Number(serde_json::Number::from(id))),
        );
        Self {
            query: serde_json::json!({ "$in": filter }),
            fields: None,
            pagination: None,
        }
    }
}

/// Pagination for property queries
#[derive(Debug, Serialize)]
pub struct PropertyPagination {
    pub offset: usize,
    pub limit: usize,
}

/// Model Derivative API client
#[derive(Clone)]
pub struct DerivativeClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl DerivativeClient {
    /// Create a new Model Derivative client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new Model Derivative client with custom HTTP config
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

    /// Start a translation job
    pub async fn translate(
        &self,
        urn: &str,
        format: OutputFormat,
        root_filename: Option<&str>,
        region: MdRegion,
        force: bool,
    ) -> Result<TranslationResponse> {
        let token = self.auth.get_token().await?;
        let job_url = format!("{}/designdata/job", self.config.derivative_url());

        let request = TranslationRequest {
            input: TranslationInput {
                urn: urn.to_string(),
                compressed_urn: None,
                root_filename: root_filename.map(|s| s.to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: region.to_string().to_lowercase(),
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
        tracing::info!(method = "POST", url = %raps_kernel::logging::redact_secrets(&job_url), "HTTP request");

        // Use retry logic for translation requests
        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self
                .http_client
                .post(&job_url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .header("x-ads-region", region.to_string());
            if force {
                req = req.header("x-ads-force", "true");
            }
            req.json(&request)
        })
        .await?;

        // Log response in verbose/debug mode
        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&job_url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to start translation ({status}): {error_text}");
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
        let manifest_url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&manifest_url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get manifest ({status}): {error_text}");
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
        let manifest_url = format!(
            "{}/designdata/{}/manifest",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&manifest_url).bearer_auth(&token)
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete manifest ({status}): {error_text}");
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
            Self::collect_downloadables(derivative, &derivative.output_type, &mut downloadables);
        }

        Ok(downloadables)
    }

    /// Recursively collect downloadable items from derivative tree
    fn collect_downloadables(
        derivative: &Derivative,
        output_type: &str,
        downloadables: &mut Vec<DownloadableDerivative>,
    ) {
        for child in &derivative.children {
            Self::collect_downloadables_from_child(child, output_type, downloadables);
        }
    }

    /// Recursively collect downloadable items from child nodes
    fn collect_downloadables_from_child(
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
            Self::collect_downloadables_from_child(grandchild, output_type, downloadables);
        }
    }

    /// Filter derivatives by format (output type)
    pub fn filter_by_format(
        derivatives: &[DownloadableDerivative],
        format: &str,
    ) -> Vec<DownloadableDerivative> {
        let target_format = format.to_ascii_lowercase();

        derivatives
            .iter()
            .filter(|d| d.output_type.to_ascii_lowercase() == target_format)
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
        let download_url = format!(
            "{}/designdata/{}/manifest/{}",
            self.config.derivative_url(),
            source_urn,
            encoded_derivative_urn
        );

        tracing::info!(method = "GET", url = %raps_kernel::logging::redact_secrets(&download_url), "HTTP request");

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&download_url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&download_url), "HTTP response");

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to download derivative ({status}): {error_text}");
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

    // ============== METADATA METHODS ==============

    /// Get metadata (list of model views/viewables) for a translated model
    pub async fn get_metadata(
        &self,
        urn: &str,
        region: Option<MdRegion>,
    ) -> Result<MetadataResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata",
            self.config.derivative_url(),
            urn
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get metadata ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse metadata response")
    }

    /// Get object tree hierarchy for a specific model view
    pub async fn get_object_tree(
        &self,
        urn: &str,
        model_guid: &str,
        region: Option<MdRegion>,
    ) -> Result<ObjectTreeResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata/{}",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get object tree ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse object tree response")
    }

    /// Get all properties for a specific model view
    pub async fn get_properties(
        &self,
        urn: &str,
        model_guid: &str,
        object_id: Option<i64>,
        region: Option<MdRegion>,
    ) -> Result<PropertiesResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/designdata/{}/metadata/{}/properties",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        if let Some(id) = object_id {
            url = format!("{}?objectid={}", url, id);
        }

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self.http_client.get(&url).bearer_auth(&token);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get properties ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse properties response")
    }

    /// Query specific properties by object IDs (POST endpoint)
    pub async fn query_properties(
        &self,
        urn: &str,
        model_guid: &str,
        query: PropertyQuery,
        region: Option<MdRegion>,
    ) -> Result<PropertiesResponse> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/designdata/{}/metadata/{}/properties:query",
            self.config.derivative_url(),
            urn,
            model_guid
        );

        let query_json =
            serde_json::to_value(&query).context("Failed to serialize property query")?;

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            let mut req = self
                .http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&query_json);
            if let Some(region) = region {
                req = req.header("x-ads-region", region.to_string());
            }
            req
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to query properties ({status}): {error_text}");
        }

        response
            .json()
            .await
            .context("Failed to parse query properties response")
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
            anyhow::bail!("No derivatives found with format '{format}'");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_format_serialization() {
        assert_eq!(
            serde_json::to_string(&OutputFormat::Svf2).unwrap(),
            "\"svf2\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Obj).unwrap(),
            "\"obj\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Ifc).unwrap(),
            "\"ifc\""
        );
    }

    #[test]
    fn test_output_format_display() {
        assert_eq!(OutputFormat::Svf2.to_string(), "SVF2 (Viewer)");
        assert_eq!(OutputFormat::Svf.to_string(), "SVF (Legacy Viewer)");
        assert_eq!(OutputFormat::Obj.to_string(), "OBJ (Mesh)");
        assert_eq!(OutputFormat::Stl.to_string(), "STL (3D Print)");
        assert_eq!(OutputFormat::Ifc.to_string(), "IFC (BIM)");
    }

    #[test]
    fn test_output_format_type_name() {
        assert_eq!(OutputFormat::Svf2.type_name(), "svf2");
        assert_eq!(OutputFormat::Obj.type_name(), "obj");
        assert_eq!(OutputFormat::Ifc.type_name(), "ifc");
    }

    #[test]
    fn test_output_format_from_str() {
        assert!(matches!(
            OutputFormat::from_str("svf2"),
            Ok(OutputFormat::Svf2)
        ));
        assert!(matches!(
            OutputFormat::from_str("SVF2"),
            Ok(OutputFormat::Svf2)
        ));
        assert!(matches!(
            OutputFormat::from_str("obj"),
            Ok(OutputFormat::Obj)
        ));
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_output_format_all() {
        let all = OutputFormat::all();
        assert_eq!(all.len(), 8);
    }

    #[test]
    fn test_filter_by_format() {
        let derivatives = vec![
            DownloadableDerivative {
                guid: "guid1".to_string(),
                name: "model.obj".to_string(),
                output_type: "obj".to_string(),
                role: "3d".to_string(),
                urn: "urn1".to_string(),
                mime: None,
                size: Some(1024),
            },
            DownloadableDerivative {
                guid: "guid2".to_string(),
                name: "model.stl".to_string(),
                output_type: "stl".to_string(),
                role: "3d".to_string(),
                urn: "urn2".to_string(),
                mime: None,
                size: None,
            },
        ];

        let filtered = DerivativeClient::filter_by_format(&derivatives, "obj");
        assert_eq!(filtered.len(), 1);

        let filtered = DerivativeClient::filter_by_format(&derivatives, "OBJ");
        assert_eq!(filtered.len(), 1);

        let filtered = DerivativeClient::filter_by_format(&derivatives, "ifc");
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_filter_by_guid() {
        let derivatives = vec![DownloadableDerivative {
            guid: "guid1".to_string(),
            name: "model.obj".to_string(),
            output_type: "obj".to_string(),
            role: "3d".to_string(),
            urn: "urn1".to_string(),
            mime: None,
            size: None,
        }];

        let found = DerivativeClient::filter_by_guid(&derivatives, "guid1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "model.obj");

        let not_found = DerivativeClient::filter_by_guid(&derivatives, "nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_translation_request_serialization() {
        let request = TranslationRequest {
            input: TranslationInput {
                urn: "test-urn".to_string(),
                compressed_urn: None,
                root_filename: Some("model.rvt".to_string()),
            },
            output: TranslationOutput {
                destination: OutputDestination {
                    region: "us".to_string(),
                },
                formats: vec![OutputFormatSpec {
                    format_type: "svf2".to_string(),
                    views: Some(vec!["2d".to_string(), "3d".to_string()]),
                }],
            },
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["input"]["rootFilename"], "model.rvt");
        assert_eq!(json["output"]["destination"]["region"], "us");
    }

    #[test]
    fn test_manifest_deserialization() {
        let json = r#"{
            "type": "manifest",
            "hasThumbnail": "true",
            "status": "success",
            "progress": "complete",
            "region": "US",
            "urn": "test-urn",
            "derivatives": []
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.status, "success");
        assert_eq!(manifest.progress, "complete");
        assert!(manifest.derivatives.is_empty());
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert!(OutputFormat::from_str("SVF2").is_ok());
        assert!(OutputFormat::from_str("svf2").is_ok());
        assert!(OutputFormat::from_str("Svf2").is_ok());
    }

    #[test]
    fn test_output_format_from_str_all_formats() {
        assert_eq!(OutputFormat::from_str("svf2").unwrap().type_name(), "svf2");
        assert_eq!(OutputFormat::from_str("svf").unwrap().type_name(), "svf");
        assert_eq!(
            OutputFormat::from_str("thumbnail").unwrap().type_name(),
            "thumbnail"
        );
        assert_eq!(OutputFormat::from_str("obj").unwrap().type_name(), "obj");
        assert_eq!(OutputFormat::from_str("stl").unwrap().type_name(), "stl");
        assert_eq!(OutputFormat::from_str("step").unwrap().type_name(), "step");
        assert_eq!(OutputFormat::from_str("iges").unwrap().type_name(), "iges");
        assert_eq!(OutputFormat::from_str("ifc").unwrap().type_name(), "ifc");
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        let result = OutputFormat::from_str("invalid");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid output format"));
        assert!(err.contains("svf2")); // Should list valid formats
    }

    #[test]
    fn test_translation_input_serialization_minimal() {
        let input = TranslationInput {
            urn: "test-urn".to_string(),
            compressed_urn: None,
            root_filename: None,
        };

        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["urn"], "test-urn");
        // Optional fields should not be present
        assert!(json.get("compressedUrn").is_none());
        assert!(json.get("rootFilename").is_none());
    }

    #[test]
    fn test_translation_input_serialization_with_options() {
        let input = TranslationInput {
            urn: "test-urn".to_string(),
            compressed_urn: Some(true),
            root_filename: Some("model.rvt".to_string()),
        };

        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(json["urn"], "test-urn");
        assert_eq!(json["compressedUrn"], true);
        assert_eq!(json["rootFilename"], "model.rvt");
    }

    #[test]
    fn test_output_format_spec_serialization() {
        let spec = OutputFormatSpec {
            format_type: "svf2".to_string(),
            views: Some(vec!["2d".to_string(), "3d".to_string()]),
        };

        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["type"], "svf2");
        assert_eq!(json["views"], serde_json::json!(["2d", "3d"]));
    }

    #[test]
    fn test_output_format_spec_serialization_no_views() {
        let spec = OutputFormatSpec {
            format_type: "obj".to_string(),
            views: None,
        };

        let json = serde_json::to_value(&spec).unwrap();
        assert_eq!(json["type"], "obj");
        assert!(json.get("views").is_none());
    }

    #[test]
    fn test_manifest_with_derivatives() {
        let json = r#"{
            "type": "manifest",
            "hasThumbnail": "true",
            "status": "success",
            "progress": "complete",
            "region": "US",
            "urn": "test-urn",
            "derivatives": [
                {
                    "status": "success",
                    "progress": "complete",
                    "outputType": "svf2",
                    "children": []
                }
            ]
        }"#;

        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.derivatives.len(), 1);
        assert_eq!(manifest.derivatives[0].output_type, "svf2");
    }

    #[test]
    fn test_filter_by_format_empty_list() {
        let derivatives: Vec<DownloadableDerivative> = vec![];
        let filtered = DerivativeClient::filter_by_format(&derivatives, "obj");
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_md_region_display() {
        assert_eq!(MdRegion::US.to_string(), "US");
        assert_eq!(MdRegion::EMEA.to_string(), "EMEA");
        assert_eq!(MdRegion::AUS.to_string(), "AUS");
        assert_eq!(MdRegion::CAN.to_string(), "CAN");
        assert_eq!(MdRegion::DEU.to_string(), "DEU");
        assert_eq!(MdRegion::IND.to_string(), "IND");
        assert_eq!(MdRegion::JPN.to_string(), "JPN");
        assert_eq!(MdRegion::GBR.to_string(), "GBR");
    }

    #[test]
    fn test_md_region_from_str() {
        assert_eq!(MdRegion::from_str("emea").unwrap(), MdRegion::EMEA);
        assert_eq!(MdRegion::from_str("US").unwrap(), MdRegion::US);
        assert_eq!(MdRegion::from_str("aus").unwrap(), MdRegion::AUS);
        assert_eq!(MdRegion::from_str("Can").unwrap(), MdRegion::CAN);
        assert_eq!(MdRegion::from_str("deu").unwrap(), MdRegion::DEU);
        assert_eq!(MdRegion::from_str("ind").unwrap(), MdRegion::IND);
        assert_eq!(MdRegion::from_str("jpn").unwrap(), MdRegion::JPN);
        assert_eq!(MdRegion::from_str("gbr").unwrap(), MdRegion::GBR);
        let err = MdRegion::from_str("invalid");
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("Valid values"));
    }

    #[test]
    fn test_md_region_default_is_us() {
        assert_eq!(MdRegion::default(), MdRegion::US);
    }

    #[test]
    fn test_metadata_response_deserialization() {
        let json = r#"{
            "data": {
                "type": "metadata",
                "metadata": [
                    {
                        "guid": "abc-123",
                        "name": "3D View",
                        "role": "3d",
                        "mime": "application/autodesk-svf2",
                        "hasThumbnail": "true",
                        "progress": "complete"
                    }
                ]
            }
        }"#;
        let resp: MetadataResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.metadata.len(), 1);
        assert_eq!(resp.data.metadata[0].guid, "abc-123");
        assert_eq!(resp.data.metadata[0].role, "3d");
    }

    #[test]
    fn test_object_tree_deserialization() {
        let json = r#"{
            "data": {
                "type": "objects",
                "objects": [
                    {
                        "objectid": 1,
                        "name": "Root",
                        "objects": [
                            {
                                "objectid": 2,
                                "name": "Child",
                                "objects": []
                            }
                        ]
                    }
                ]
            }
        }"#;
        let resp: ObjectTreeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.objects.len(), 1);
        assert_eq!(resp.data.objects[0].object_id, 1);
        assert_eq!(resp.data.objects[0].objects.len(), 1);
        assert_eq!(resp.data.objects[0].objects[0].name, "Child");
    }

    #[test]
    fn test_properties_response_deserialization() {
        let json = r#"{
            "data": {
                "type": "properties",
                "collection": [
                    {
                        "objectid": 42,
                        "name": "Wall",
                        "externalId": "ext-42",
                        "properties": {
                            "Dimensions": {
                                "Width": "300mm"
                            }
                        }
                    }
                ]
            }
        }"#;
        let resp: PropertiesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.data.collection.len(), 1);
        assert_eq!(resp.data.collection[0].object_id, 42);
        assert_eq!(resp.data.collection[0].name, "Wall");
        assert!(
            resp.data.collection[0]
                .properties
                .contains_key("Dimensions")
        );
    }

    #[test]
    fn test_property_query_by_object_ids() {
        let query = PropertyQuery::by_object_ids(vec![1, 2, 3]);
        let json = serde_json::to_value(&query).unwrap();
        let filter = &json["query"]["$in"];
        assert_eq!(filter[0], "objectid");
        assert_eq!(filter[1], 1);
        assert_eq!(filter[2], 2);
        assert_eq!(filter[3], 3);
        assert!(json.get("fields").is_none());
        assert!(json.get("pagination").is_none());
    }

    #[test]
    fn test_metadata_response_empty() {
        let json = r#"{"data": {"type": "metadata", "metadata": []}}"#;
        let resp: MetadataResponse = serde_json::from_str(json).unwrap();
        assert!(resp.data.metadata.is_empty());
    }
}

/// Integration tests using raps-mock
#[cfg(test)]
mod integration_tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_mock_client(mock_url: &str) -> DerivativeClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        DerivativeClient::new(config, auth)
    }

    #[tokio::test]
    async fn test_client_creation() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_client(&server.url);
        assert!(client.auth.config().base_url.starts_with("http://"));
    }
}
