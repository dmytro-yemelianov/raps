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
use raps_kernel::logging;

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
        logging::log_request("POST", &job_url);

        // Use retry logic for translation requests
        let http_config = HttpClientConfig::default();
        let response = raps_kernel::http::execute_with_retry(&http_config, || {
            let client = self.http_client.clone();
            let url = job_url.clone();
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
        logging::log_response(response.status().as_u16(), &job_url);

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

        let response = self
            .http_client
            .get(&manifest_url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get manifest")?;

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

        let response = self
            .http_client
            .delete(&manifest_url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete manifest")?;

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

        logging::log_request("GET", &download_url);

        let response = self
            .http_client
            .get(&download_url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to download derivative")?;

        logging::log_response(response.status().as_u16(), &download_url);

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
}
