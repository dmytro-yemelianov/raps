// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Download operations for the Model Derivative API.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use crate::DerivativeClient;

impl DerivativeClient {
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
        let pb = raps_kernel::progress::file_progress(total_size, "");

        let filename = output_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("derivative");
        pb.set_message(format!("Downloading {}", filename));

        // Validate output path stays within current directory
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        if let (Ok(canon_cwd), Ok(canon_target)) = (cwd.canonicalize(), output_path.canonicalize())
            && !canon_target.starts_with(&canon_cwd)
        {
            anyhow::bail!(
                "Path '{}' escapes working directory '{}'",
                output_path.display(),
                cwd.display()
            );
        }

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
}
