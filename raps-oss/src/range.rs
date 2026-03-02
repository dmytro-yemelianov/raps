// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP Range request support for partial object downloads.

use anyhow::{Context, Result};

use crate::OssClient;

impl OssClient {
    /// Fetch a byte range from an object using a signed S3 URL.
    ///
    /// Returns the bytes for the requested range. The server may return
    /// fewer bytes if the range extends beyond the file.
    pub async fn fetch_range(
        &self,
        bucket_key: &str,
        object_key: &str,
        start: u64,
        end: u64,
    ) -> Result<Vec<u8>> {
        let signed = self
            .get_signed_download_url(bucket_key, object_key, None)
            .await?;

        let download_url = signed
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL returned"))?;

        let range_header = format!("bytes={}-{}", start, end);

        let response = self
            .http_client
            .get(&download_url)
            .header("Range", &range_header)
            .send()
            .await
            .context("Range request failed")?;

        let status = response.status();
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            anyhow::bail!("Server does not support Range requests for this object");
        }
        if !status.is_success() && status != reqwest::StatusCode::PARTIAL_CONTENT {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Range request failed ({status}): {error_text}");
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read range response")?;
        Ok(bytes.to_vec())
    }
}
