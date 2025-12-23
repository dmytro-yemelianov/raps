//! Model Derivative API module
//!
//! Handles translation of CAD files and retrieval of derivative manifests.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivativeChild {
    pub guid: String,
    #[serde(rename = "type")]
    pub child_type: String,
    pub role: String,
    pub name: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub children: Vec<DerivativeChild>,
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
        Self {
            config,
            auth,
            http_client: reqwest::Client::new(),
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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .header("x-ads-force", "true")
            .json(&request)
            .send()
            .await
            .context("Failed to start translation")?;

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
}
