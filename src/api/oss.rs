// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object Storage Service (OSS) API module
//!
//! This module is now an adapter that wraps raps-oss service crate
//! to maintain backward compatibility with existing commands.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio::sync::Mutex;

use super::AuthClient;
use crate::config::Config;

// Re-export types from raps-oss for backward compatibility
pub use raps_oss::types::{
    RetentionPolicy, Region, CreateBucketRequest, Permission, Bucket, BucketItem,
    ObjectInfo, ObjectItem, SignedS3DownloadResponse, SignedS3UploadResponse,
};

// Re-export MultipartUploadState from raps-oss
pub use raps_oss::upload::{MultipartUploadState, UploadConfig};

/// OSS API client (adapter wrapping raps-oss service crate with cached clients)
#[derive(Clone)]
pub struct OssClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
    // Cached kernel clients (lazy-initialized)
    kernel_config: Arc<Mutex<Option<raps_kernel::Config>>>,
    kernel_http: Arc<Mutex<Option<raps_kernel::HttpClient>>>,
    kernel_auth: Arc<Mutex<Option<raps_kernel::AuthClient>>>,
}

impl OssClient {
    /// Create a new OSS client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new OSS client with custom HTTP config
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
            kernel_config: Arc::new(Mutex::new(None)),
            kernel_http: Arc::new(Mutex::new(None)),
            kernel_auth: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or create kernel config (cached)
    async fn get_kernel_config(&self) -> Result<raps_kernel::Config> {
        let mut config = self.kernel_config.lock().await;
        if config.is_none() {
            *config = Some(raps_kernel::Config {
                client_id: self.config.client_id.clone(),
                client_secret: self.config.client_secret.clone(),
                base_url: self.config.base_url.clone(),
                callback_url: self.config.callback_url.clone(),
                da_nickname: self.config.da_nickname.clone(),
            });
        }
        Ok(config.as_ref().unwrap().clone())
    }

    /// Get or create kernel HTTP client (cached)
    async fn get_kernel_http(&self) -> Result<raps_kernel::HttpClient> {
        let mut http = self.kernel_http.lock().await;
        if http.is_none() {
            let config = raps_kernel::HttpClientConfig {
                timeout: std::time::Duration::from_secs(120),
                connect_timeout: std::time::Duration::from_secs(30),
                max_retries: 3,
                retry_base_delay: std::time::Duration::from_secs(1),
                retry_max_delay: std::time::Duration::from_secs(60),
                retry_jitter: true,
            };
            *http = Some(raps_kernel::HttpClient::new(config)
                .map_err(|e| anyhow::anyhow!("Failed to create kernel HTTP client: {}", e))?);
        }
        Ok(http.as_ref().unwrap().clone())
    }

    /// Get or create kernel auth client (cached)
    async fn get_kernel_auth(&self) -> Result<raps_kernel::AuthClient> {
        let mut auth = self.kernel_auth.lock().await;
        if auth.is_none() {
            let kernel_config = self.get_kernel_config().await?;
            *auth = Some(raps_kernel::AuthClient::new(kernel_config)
                .map_err(|e| anyhow::anyhow!("Failed to create kernel auth client: {}", e))?);
        }
        Ok(auth.as_ref().unwrap().clone())
    }

    /// Create a new bucket
    pub async fn create_bucket(
        &self,
        bucket_key: &str,
        policy: RetentionPolicy,
        region: Region,
    ) -> Result<Bucket> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let bucket_client = raps_oss::BucketClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;

        bucket_client.create_bucket(&bucket_key_typed, policy, region).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List all buckets from all regions
    pub async fn list_buckets(&self) -> Result<Vec<BucketItem>> {
        let mut all_buckets = Vec::new();

        // Query both US and EMEA regions
        for region in Region::all() {
            let mut region_buckets = self.list_buckets_in_region(region).await?;
            // Tag each bucket with its region
            for bucket in &mut region_buckets {
                bucket.region = Some(region.to_string());
            }
            all_buckets.extend(region_buckets);
        }

        Ok(all_buckets)
    }

    /// List buckets in a specific region
    pub async fn list_buckets_in_region(&self, region: Region) -> Result<Vec<BucketItem>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let bucket_client = raps_oss::BucketClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        bucket_client.list_buckets_in_region(region).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get bucket details
    pub async fn get_bucket_details(&self, bucket_key: &str) -> Result<Bucket> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let bucket_client = raps_oss::BucketClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;

        bucket_client.get_bucket_details(&bucket_key_typed).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Delete a bucket
    pub async fn delete_bucket(&self, bucket_key: &str) -> Result<()> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let bucket_client = raps_oss::BucketClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;

        bucket_client.delete_bucket(&bucket_key_typed).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Upload a file
    pub async fn upload_object(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
    ) -> Result<ObjectInfo> {
        self.upload_object_with_options(bucket_key, object_key, file_path, false)
            .await
    }

    /// Upload a file with resume option
    pub async fn upload_object_with_options(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
        resume: bool,
    ) -> Result<ObjectInfo> {
        // For now, use the old implementation since raps-oss upload is not fully implemented
        // TODO: Migrate to raps-oss once upload is complete
        let metadata = tokio::fs::metadata(file_path)
            .await
            .context("Failed to get file metadata")?;
        let file_size = metadata.len();

        // Use multipart upload for files larger than threshold
        if file_size > MultipartUploadState::MULTIPART_THRESHOLD {
            self.upload_multipart(bucket_key, object_key, file_path, resume)
                .await
        } else {
            self.upload_single_part(bucket_key, object_key, file_path)
                .await
        }
    }

    /// Upload a small file using single-part upload
    async fn upload_single_part(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
    ) -> Result<ObjectInfo> {
        // Read file
        let mut file = File::open(file_path)
            .await
            .context("Failed to open file for upload")?;

        let metadata = file
            .metadata()
            .await
            .context("Failed to get file metadata")?;
        let file_size = metadata.len();

        // Create progress bar
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message(format!("Uploading {}", object_key));

        // Read file contents
        let mut buffer = Vec::with_capacity(file_size as usize);
        file.read_to_end(&mut buffer)
            .await
            .context("Failed to read file")?;

        // Step 1: Get signed S3 upload URL
        pb.set_message(format!("Getting upload URL for {}", object_key));
        let signed = self
            .get_signed_upload_url(bucket_key, object_key, None, None)
            .await?;

        if signed.urls.is_empty() {
            anyhow::bail!("No upload URLs returned from signed upload request");
        }

        // Step 2: Upload directly to S3
        pb.set_message(format!("Uploading {} to S3", object_key));
        let s3_url = &signed.urls[0];

        let response = self
            .http_client
            .put(s3_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size.to_string())
            .body(buffer)
            .send()
            .await
            .context("Failed to upload to S3")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upload to S3 ({}): {}", status, error_text);
        }

        pb.set_position(file_size);

        // Step 3: Complete the upload
        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &signed.upload_key)
            .await?;

        pb.finish_with_message(format!("Uploaded {}", object_key));

        Ok(object_info)
    }

    /// Upload a large file using multipart upload with resume capability
    pub async fn upload_multipart(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
        resume: bool,
    ) -> Result<ObjectInfo> {
        // Use the old implementation for now since it has full resume support
        // TODO: Migrate to raps-oss once upload is complete
        let metadata = tokio::fs::metadata(file_path)
            .await
            .context("Failed to get file metadata")?;
        let file_size = metadata.len();
        let file_mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let chunk_size = MultipartUploadState::DEFAULT_CHUNK_SIZE;
        let total_parts = file_size.div_ceil(chunk_size) as u32;

        // Helper functions for state management (not in raps-oss yet)
        fn state_file_path(bucket_key: &str, object_key: &str) -> Result<PathBuf> {
            let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
                .context("Failed to get project directories")?;
            let cache_dir = proj_dirs.cache_dir();
            std::fs::create_dir_all(cache_dir)?;
            let safe_name = format!("{}_{}", bucket_key, object_key)
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            Ok(cache_dir.join(format!("upload_{}.json", safe_name)))
        }

        fn load_state(bucket_key: &str, object_key: &str) -> Result<Option<MultipartUploadState>> {
            let path = state_file_path(bucket_key, object_key)?;
            if !path.exists() {
                return Ok(None);
            }
            let json = std::fs::read_to_string(&path)?;
            let state: MultipartUploadState = serde_json::from_str(&json)?;
            Ok(Some(state))
        }

        fn save_state(state: &MultipartUploadState) -> Result<()> {
            let path = state_file_path(&state.bucket_key, &state.object_key)?;
            let json = serde_json::to_string_pretty(state)?;
            std::fs::write(&path, json)?;
            Ok(())
        }

        fn delete_state(bucket_key: &str, object_key: &str) -> Result<()> {
            let path = state_file_path(bucket_key, object_key)?;
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            Ok(())
        }

        fn can_resume(state: &MultipartUploadState, file_path: &Path) -> bool {
            if let Ok(metadata) = std::fs::metadata(file_path) {
                let current_size = metadata.len();
                let current_mtime = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                current_size == state.file_size && current_mtime == state.file_mtime
            } else {
                false
            }
        }

        let (mut state, initial_urls) = if resume {
            if let Some(existing_state) = load_state(bucket_key, object_key)? {
                if can_resume(&existing_state, file_path) {
                    crate::logging::log_verbose(&format!(
                        "Resuming upload: {}/{} completed parts",
                        existing_state.completed_parts.len(),
                        existing_state.total_parts
                    ));
                    (existing_state, None)
                } else {
                    crate::logging::log_verbose("File changed since last upload, starting fresh");
                    delete_state(bucket_key, object_key)?;
                    let signed = self
                        .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                        .await?;

                    if signed.urls.len() != total_parts as usize {
                        anyhow::bail!(
                            "Expected {} URLs but got {}",
                            total_parts,
                            signed.urls.len()
                        );
                    }

                    let new_state = MultipartUploadState {
                        bucket_key: bucket_key.to_string(),
                        object_key: object_key.to_string(),
                        file_path: file_path.to_string_lossy().to_string(),
                        file_size,
                        chunk_size,
                        total_parts,
                        completed_parts: Vec::new(),
                        part_etags: std::collections::HashMap::new(),
                        upload_key: signed.upload_key,
                        started_at: chrono::Utc::now().timestamp(),
                        file_mtime,
                    };
                    save_state(&new_state)?;
                    (new_state, Some(signed.urls))
                }
            } else {
                let signed = self
                    .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                    .await?;

                if signed.urls.len() != total_parts as usize {
                    anyhow::bail!(
                        "Expected {} URLs but got {}",
                        total_parts,
                        signed.urls.len()
                    );
                }

                let new_state = MultipartUploadState {
                    bucket_key: bucket_key.to_string(),
                    object_key: object_key.to_string(),
                    file_path: file_path.to_string_lossy().to_string(),
                    file_size,
                    chunk_size,
                    total_parts,
                    completed_parts: Vec::new(),
                    part_etags: std::collections::HashMap::new(),
                    upload_key: signed.upload_key,
                    started_at: chrono::Utc::now().timestamp(),
                    file_mtime,
                };
                save_state(&new_state)?;
                (new_state, Some(signed.urls))
            }
        } else {
            delete_state(bucket_key, object_key)?;

            let signed = self
                .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                .await?;

            if signed.urls.len() != total_parts as usize {
                anyhow::bail!(
                    "Expected {} URLs but got {}",
                    total_parts,
                    signed.urls.len()
                );
            }

            let new_state = MultipartUploadState {
                bucket_key: bucket_key.to_string(),
                object_key: object_key.to_string(),
                file_path: file_path.to_string_lossy().to_string(),
                file_size,
                chunk_size,
                total_parts,
                completed_parts: Vec::new(),
                part_etags: std::collections::HashMap::new(),
                upload_key: signed.upload_key,
                started_at: chrono::Utc::now().timestamp(),
                file_mtime,
            };
            save_state(&new_state)?;
            (new_state, Some(signed.urls))
        };

        // Create progress bar
        let pb = ProgressBar::new(file_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("█▓░"),
        );

        if !state.completed_parts.is_empty() {
            let completed_bytes: u64 = state
                .completed_parts
                .iter()
                .map(|&part| {
                    let start = (part as u64 - 1) * state.chunk_size;
                    let end = std::cmp::min(start + state.chunk_size, state.file_size);
                    end - start
                })
                .sum();
            pb.set_position(completed_bytes);
            pb.set_message(format!(
                "Resuming {} ({} parts done)",
                object_key,
                state.completed_parts.len()
            ));
        } else {
            pb.set_message(format!("Starting multipart upload for {}", object_key));
        }

        let remaining_parts = state.remaining_parts();

        if remaining_parts.is_empty() {
            pb.set_message(format!("All parts uploaded, completing {}", object_key));
        } else {
            pb.set_message(format!(
                "Uploading {} ({} parts remaining)",
                object_key,
                remaining_parts.len()
            ));
        }

        let urls = if let Some(u) = initial_urls {
            u
        } else {
            let signed = self
                .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                .await?;
            signed.urls
        };

        let mut file = File::open(file_path)
            .await
            .context("Failed to open file for upload")?;

        for part_num in remaining_parts {
            let part_index = (part_num - 1) as usize;
            let start = (part_num as u64 - 1) * chunk_size;
            let end = std::cmp::min(start + chunk_size, file_size);
            let part_size = end - start;

            file.seek(SeekFrom::Start(start)).await?;

            let mut buffer = vec![0u8; part_size as usize];
            file.read_exact(&mut buffer).await?;

            let s3_url = &urls[part_index];
            let response = self
                .http_client
                .put(s3_url)
                .header("Content-Type", "application/octet-stream")
                .header("Content-Length", part_size.to_string())
                .body(buffer)
                .send()
                .await
                .with_context(|| format!("Failed to upload part {}", part_num))?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!(
                    "Failed to upload part {} ({}): {}",
                    part_num,
                    status,
                    error_text
                );
            }

            let etag = response
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_default();

            state.completed_parts.push(part_num);
            state.part_etags.insert(part_num, etag);
            save_state(&state)?;

            pb.set_position(end);
            pb.set_message(format!(
                "Uploading {} ({}/{})",
                object_key,
                state.completed_parts.len(),
                total_parts
            ));
        }

        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &state.upload_key)
            .await?;

        delete_state(bucket_key, object_key)?;

        pb.finish_with_message(format!("Uploaded {} (multipart)", object_key));

        Ok(object_info)
    }

    /// Download an object
    pub async fn download_object(
        &self,
        bucket_key: &str,
        object_key: &str,
        output_path: &Path,
    ) -> Result<()> {
        // Use old implementation since raps-oss download is not yet implemented
        let signed = self
            .get_signed_download_url(bucket_key, object_key, None)
            .await?;

        let download_url = signed
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL returned"))?;

        let response = self
            .http_client
            .get(&download_url)
            .send()
            .await
            .context("Failed to download from S3")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to download from S3 ({}): {}", status, error_text);
        }

        let total_size = signed
            .size
            .unwrap_or(response.content_length().unwrap_or(0));

        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message(format!("Downloading {}", object_key));

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

        pb.finish_with_message(format!("Downloaded {}", object_key));
        Ok(())
    }

    /// List objects in a bucket
    pub async fn list_objects(&self, bucket_key: &str) -> Result<Vec<ObjectItem>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let object_client = raps_oss::ObjectClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;

        object_client.list_objects(&bucket_key_typed).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Delete an object
    pub async fn delete_object(&self, bucket_key: &str, object_key: &str) -> Result<()> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let object_client = raps_oss::ObjectClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;
        let object_key_typed = raps_kernel::ObjectKey::new(object_key);

        object_client.delete_object(&bucket_key_typed, &object_key_typed).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get URN for an object (synchronous, as expected by commands)
    pub fn get_urn(&self, bucket_key: &str, object_key: &str) -> String {
        // Use the same logic as raps-oss ObjectClient::get_urn
        use base64::Engine;
        let object_id = format!(
            "urn:adsk.objects:os.object:{}/{}",
            bucket_key,
            object_key
        );
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(object_id)
    }

    /// Get a signed S3 URL for direct download
    pub async fn get_signed_download_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        minutes: Option<u32>,
    ) -> Result<SignedS3DownloadResponse> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let signed_url_client = raps_oss::SignedUrlClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;
        let object_key_typed = raps_kernel::ObjectKey::new(object_key);

        signed_url_client.get_signed_download_url(&bucket_key_typed, &object_key_typed, minutes).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get a signed S3 URL for direct upload
    pub async fn get_signed_upload_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        parts: Option<u32>,
        minutes: Option<u32>,
    ) -> Result<SignedS3UploadResponse> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let signed_url_client = raps_oss::SignedUrlClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;
        let object_key_typed = raps_kernel::ObjectKey::new(object_key);

        signed_url_client.get_signed_upload_url(&bucket_key_typed, &object_key_typed, parts, minutes).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Complete an S3 signed upload
    pub async fn complete_signed_upload(
        &self,
        bucket_key: &str,
        object_key: &str,
        upload_key: &str,
    ) -> Result<ObjectInfo> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let oss_url = kernel_config.oss_url();
        
        let signed_url_client = raps_oss::SignedUrlClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            oss_url,
        );

        let bucket_key_typed = raps_kernel::BucketKey::new(bucket_key)
            .map_err(|e| anyhow::anyhow!("Invalid bucket key: {}", e))?;
        let object_key_typed = raps_kernel::ObjectKey::new(object_key);

        signed_url_client.complete_signed_upload(&bucket_key_typed, &object_key_typed, upload_key).await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
