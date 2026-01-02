// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Signed URL operations

use crate::types::*;
use raps_kernel::{AuthClient, BucketKey, Config, HttpClient, ObjectKey, RapsError, Result};

/// Signed URL client for OSS operations
pub struct SignedUrlClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl SignedUrlClient {
    /// Create new signed URL client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// Get a signed S3 URL for direct download (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    pub async fn get_signed_download_url(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3DownloadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3download",
            self.base_url,
            bucket_key.as_str(),
            urlencoding::encode(object_key.as_str())
        );

        if let Some(mins) = minutes_expiration {
            url = format!("{}?minutesExpiration={}", url, mins);
        }

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get signed download URL".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!(
                    "Failed to get signed download URL ({}): {}",
                    status, error_text
                ),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let signed: SignedS3DownloadResponse =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse signed URL response: {}", e),
            })?;

        Ok(signed)
    }

    /// Get a signed S3 URL for direct upload (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    /// Returns an upload key that must be used to complete the upload.
    pub async fn get_signed_upload_url(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
        parts: Option<u32>,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3UploadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.base_url,
            bucket_key.as_str(),
            urlencoding::encode(object_key.as_str())
        );

        let mut params = Vec::new();
        if let Some(p) = parts {
            params.push(format!("parts={}", p));
        }
        if let Some(mins) = minutes_expiration {
            params.push(format!("minutesExpiration={}", mins));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get signed upload URL".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!(
                    "Failed to get signed upload URL ({}): {}",
                    status, error_text
                ),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let signed: SignedS3UploadResponse =
            response.json().await.map_err(|e| RapsError::Internal {
                message: format!("Failed to parse signed URL response: {}", e),
            })?;

        Ok(signed)
    }

    /// Complete an S3 signed upload
    pub async fn complete_signed_upload(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
        upload_key: &str,
    ) -> Result<ObjectInfo> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.base_url,
            bucket_key.as_str(),
            urlencoding::encode(object_key.as_str())
        );

        let body = serde_json::json!({ "uploadKey": upload_key });

        let response = self
            .http
            .inner()
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to complete signed upload".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!(
                    "Failed to complete signed upload ({}): {}",
                    status, error_text
                ),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let object_info: ObjectInfo = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse completion response: {}", e),
        })?;

        Ok(object_info)
    }
}
