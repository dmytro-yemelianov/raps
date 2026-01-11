// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object Storage Service (OSS) API module
//!
//! Handles bucket and object operations for storing files in APS.
//! Supports multipart chunked uploads for large files with resume capability.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;
use raps_kernel::logging;

/// Bucket retention policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RetentionPolicy {
    /// Files are automatically deleted after 24 hours
    Transient,
    /// Files are automatically deleted after 30 days
    Temporary,
    /// Files are kept until explicitly deleted
    Persistent,
}

impl std::fmt::Display for RetentionPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RetentionPolicy::Transient => write!(f, "transient"),
            RetentionPolicy::Temporary => write!(f, "temporary"),
            RetentionPolicy::Persistent => write!(f, "persistent"),
        }
    }
}

impl RetentionPolicy {
    pub fn all() -> Vec<Self> {
        vec![Self::Transient, Self::Temporary, Self::Persistent]
    }
}

impl FromStr for RetentionPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "transient" => Ok(Self::Transient),
            "temporary" => Ok(Self::Temporary),
            "persistent" => Ok(Self::Persistent),
            _ => Err("Invalid retention policy".to_string()),
        }
    }
}

/// Region for bucket storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    US,
    #[allow(clippy::upper_case_acronyms)]
    EMEA,
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Region::US => write!(f, "US"),
            Region::EMEA => write!(f, "EMEA"),
        }
    }
}

impl Region {
    pub fn all() -> Vec<Self> {
        vec![Self::US, Self::EMEA]
    }
}

/// Request to create a new bucket
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBucketRequest {
    pub bucket_key: String,
    pub policy_key: String,
}

/// Bucket information returned from API
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub bucket_key: String,
    pub bucket_owner: String,
    pub created_date: u64,
    pub permissions: Vec<Permission>,
    pub policy_key: String,
}

/// Permission information for a bucket
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    pub auth_id: String,
    pub access: String,
}

/// Response when listing buckets
#[derive(Debug, Deserialize)]
pub struct BucketsResponse {
    pub items: Vec<BucketItem>,
    pub next: Option<String>,
}

/// Bucket item in list response
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BucketItem {
    pub bucket_key: String,
    pub created_date: u64,
    pub policy_key: String,
    /// Region where the bucket is stored (added by client, not from API)
    #[serde(skip)]
    pub region: Option<String>,
}

/// Signed S3 download response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedS3DownloadResponse {
    /// Pre-signed S3 URL for direct download
    pub url: Option<String>,
    /// Multiple URLs if object was uploaded in chunks
    pub urls: Option<Vec<String>>,
    /// Object size in bytes
    pub size: Option<u64>,
    /// SHA-1 hash
    pub sha1: Option<String>,
    /// Status of the object
    pub status: Option<String>,
}

/// Signed S3 upload response
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedS3UploadResponse {
    /// Upload key to use for completion
    pub upload_key: String,
    /// Pre-signed S3 URLs for upload
    pub urls: Vec<String>,
    /// Expiration timestamp
    pub upload_expiration: Option<String>,
}

/// Multipart upload state for resume capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultipartUploadState {
    /// Bucket key
    pub bucket_key: String,
    /// Object key
    pub object_key: String,
    /// Local file path
    pub file_path: String,
    /// Total file size
    pub file_size: u64,
    /// Chunk size used
    pub chunk_size: u64,
    /// Total number of parts
    pub total_parts: u32,
    /// Completed part numbers (1-indexed)
    pub completed_parts: Vec<u32>,
    /// ETags for completed parts (part_number -> etag)
    pub part_etags: std::collections::HashMap<u32, String>,
    /// Upload key from signed URL request
    pub upload_key: String,
    /// Timestamp when upload started
    pub started_at: i64,
    /// File modification time for validation
    pub file_mtime: i64,
}

impl MultipartUploadState {
    /// Default chunk size: 5MB (minimum for S3 multipart)
    pub const DEFAULT_CHUNK_SIZE: u64 = 5 * 1024 * 1024;
    /// Maximum chunk size: 100MB
    pub const MAX_CHUNK_SIZE: u64 = 100 * 1024 * 1024;
    /// Threshold for multipart upload: 5MB
    pub const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024;

    /// Get the state file path for a given upload
    pub fn state_file_path(bucket_key: &str, object_key: &str) -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
            .context("Failed to get project directories")?;
        let cache_dir = proj_dirs.cache_dir();
        std::fs::create_dir_all(cache_dir)?;

        // Create a safe filename from bucket and object key
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

    /// Save state to file
    pub fn save(&self) -> Result<()> {
        let path = Self::state_file_path(&self.bucket_key, &self.object_key)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;
        Ok(())
    }

    /// Load state from file
    pub fn load(bucket_key: &str, object_key: &str) -> Result<Option<Self>> {
        let path = Self::state_file_path(bucket_key, object_key)?;
        if !path.exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&json)?;
        Ok(Some(state))
    }

    /// Delete state file
    pub fn delete(bucket_key: &str, object_key: &str) -> Result<()> {
        let path = Self::state_file_path(bucket_key, object_key)?;
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Check if the upload can be resumed (file hasn't changed)
    pub fn can_resume(&self, file_path: &Path) -> bool {
        if let Ok(metadata) = std::fs::metadata(file_path) {
            let current_size = metadata.len();
            let current_mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            current_size == self.file_size && current_mtime == self.file_mtime
        } else {
            false
        }
    }

    /// Calculate which parts still need to be uploaded
    pub fn remaining_parts(&self) -> Vec<u32> {
        (1..=self.total_parts)
            .filter(|p| !self.completed_parts.contains(p))
            .collect()
    }
}

/// Object information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectInfo {
    pub bucket_key: String,
    pub object_key: String,
    pub object_id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    pub size: u64,
    #[serde(default)]
    pub location: Option<String>,
    /// Content type (may be returned by some endpoints)
    #[serde(default)]
    pub content_type: Option<String>,
}

/// Response when listing objects
#[derive(Debug, Deserialize)]
pub struct ObjectsResponse {
    pub items: Vec<ObjectItem>,
    pub next: Option<String>,
}

/// Object item in list response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectItem {
    pub bucket_key: String,
    pub object_key: String,
    pub object_id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    pub size: u64,
}

/// OSS API client
#[derive(Clone)]
pub struct OssClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl OssClient {
    /// Create a new OSS client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
    }

    /// Create a new OSS client with custom HTTP config
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

    /// Create a new bucket
    pub async fn create_bucket(
        &self,
        bucket_key: &str,
        policy: RetentionPolicy,
        region: Region,
    ) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets", self.config.oss_url());

        let request = CreateBucketRequest {
            bucket_key: bucket_key.to_string(),
            policy_key: policy.to_string(),
        };

        // Log request in verbose/debug mode
        logging::log_request("POST", &url);

        // Use retry logic for bucket creation
        let http_config = HttpClientConfig::default();
        let response = raps_kernel::http::execute_with_retry(&http_config, || {
            let client = self.http_client.clone();
            let url = url.clone();
            let token = token.clone();
            let region_str = region.to_string();
            let request_json = serde_json::to_value(&request).ok();
            Box::pin(async move {
                let mut req = client
                    .post(&url)
                    .bearer_auth(&token)
                    .header("x-ads-region", region_str)
                    .header("Content-Type", "application/json");
                if let Some(json) = request_json {
                    req = req.json(&json);
                }
                req.send().await.context("Failed to create bucket")
            })
        })
        .await?;

        // Log response in verbose/debug mode
        logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create bucket ({status}): {error_text}");
        }

        let bucket: Bucket = response
            .json()
            .await
            .context("Failed to parse bucket response")?;

        Ok(bucket)
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
    async fn list_buckets_in_region(&self, region: Region) -> Result<Vec<BucketItem>> {
        let token = self.auth.get_token().await?;
        let mut buckets = Vec::new();
        let mut start_at: Option<String> = None;

        loop {
            let mut url = format!("{}/buckets", self.config.oss_url());
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = raps_kernel::http::execute_with_retry(&self.config.http_config, || {
                let client = self.http_client.clone();
                let url = url.clone();
                let token = token.clone();
                let region = region.to_string();
                Box::pin(async move {
                    client
                        .get(&url)
                        .bearer_auth(&token)
                        .header("x-ads-region", region)
                        .send()
                        .await
                        .context("Failed to list buckets")
                })
            })
            .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list buckets ({status}): {error_text}");
            }

            let buckets_response: BucketsResponse = response
                .json()
                .await
                .context("Failed to parse buckets response")?;

            buckets.extend(buckets_response.items);

            if buckets_response.next.is_none() {
                break;
            }
            start_at = buckets_response.next;
        }

        Ok(buckets)
    }

    /// Get bucket details
    pub async fn get_bucket_details(&self, bucket_key: &str) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}/details", self.config.oss_url(), bucket_key);

        // Log request in verbose/debug mode
        logging::log_request("GET", &url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get bucket details")?;

        // Log response in verbose/debug mode
        logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get bucket details ({status}): {error_text}");
        }

        let bucket: Bucket = response
            .json()
            .await
            .context("Failed to parse bucket details")?;

        Ok(bucket)
    }

    /// Delete a bucket
    pub async fn delete_bucket(&self, bucket_key: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}", self.config.oss_url(), bucket_key);

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete bucket")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete bucket ({status}): {error_text}");
        }

        Ok(())
    }

    /// Upload a file to a bucket using S3 signed URLs
    /// Automatically uses multipart upload for files larger than 5MB
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
    /// If resume is true, will attempt to resume an interrupted upload
    pub async fn upload_object_with_options(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
        resume: bool,
    ) -> Result<ObjectInfo> {
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

        // Step 1: Get signed S3 upload URL
        pb.set_message(format!("Getting upload URL for {}", object_key));
        let signed = self
            .get_signed_upload_url(bucket_key, object_key, None, None)
            .await?;

        if signed.urls.is_empty() {
            anyhow::bail!("No upload URLs returned from signed upload request");
        }

        // Step 2: Stream upload directly to S3 instead of loading into memory
        pb.set_message(format!("Uploading {} to S3", object_key));
        let s3_url = &signed.urls[0];

        // Create a streaming body that reads the file in chunks
        use futures_util::stream::TryStreamExt;
        use tokio_util::codec::{BytesCodec, FramedRead};

        // Reset file position to start
        file.seek(std::io::SeekFrom::Start(0)).await?;

        // Create a stream that reads the file in chunks
        let file_stream = FramedRead::new(file, BytesCodec::new())
            .map_ok(|bytes| bytes.freeze())
            .map_err(std::io::Error::other);

        let body = reqwest::Body::wrap_stream(file_stream);

        let response = self
            .http_client
            .put(s3_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size.to_string())
            .body(body)
            .send()
            .await
            .context("Failed to upload to S3")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to upload to S3 ({status}): {error_text}");
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

        let (mut state, initial_urls) = if resume {
            if let Some(existing_state) = MultipartUploadState::load(bucket_key, object_key)? {
                if existing_state.can_resume(file_path) {
                    logging::log_verbose(&format!(
                        "Resuming upload: {}/{} completed parts",
                        existing_state.completed_parts.len(),
                        existing_state.total_parts
                    ));
                    (existing_state, None)
                } else {
                    logging::log_verbose("File changed since last upload, starting fresh");
                    MultipartUploadState::delete(bucket_key, object_key)?;
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
                    new_state.save()?;
                    (new_state, Some(signed.urls))
                }
            } else {
                // No state, start fresh
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
                new_state.save()?;
                (new_state, Some(signed.urls))
            }
        } else {
            // Not resuming, clear any existing state
            MultipartUploadState::delete(bucket_key, object_key)?;

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
            new_state.save()?;
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

        // Update progress if resuming
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

        // Get remaining parts to upload
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

        // Upload remaining parts in parallel with bounded concurrency
        use futures_util::stream::FuturesUnordered;
        use std::sync::Arc;
        use tokio::sync::{Mutex, Semaphore};

        const MAX_CONCURRENT_UPLOADS: usize = 5;
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS));
        let upload_key = state.upload_key.clone();
        let state_mutex = Arc::new(Mutex::new(&mut state));
        let pb_arc = Arc::new(Mutex::new(pb));
        let file_path_clone = file_path.to_path_buf();

        // Create upload tasks
        let upload_tasks: FuturesUnordered<_> = remaining_parts
            .into_iter()
            .map(|part_num| {
                let part_index = (part_num - 1) as usize;
                let start = (part_num as u64 - 1) * chunk_size;
                let end = std::cmp::min(start + chunk_size, file_size);
                let part_size = end - start;
                let s3_url = urls[part_index].clone();
                let client = self.http_client.clone();
                let semaphore = semaphore.clone();
                let state_mutex = state_mutex.clone();
                let pb_arc = pb_arc.clone();
                let object_key = object_key.to_string();
                let file_path = file_path_clone.clone();

                async move {
                    // Acquire semaphore permit to limit concurrency
                    let _permit = semaphore.acquire().await.unwrap();

                    // Read file chunk
                    let buffer = {
                        let mut file =
                            tokio::fs::File::open(&file_path).await.with_context(|| {
                                format!("Failed to open file for part {}", part_num)
                            })?;
                        file.seek(SeekFrom::Start(start)).await?;
                        let mut buffer = vec![0u8; part_size as usize];
                        file.read_exact(&mut buffer).await?;
                        buffer
                    };

                    // Upload part with retry logic
                    let mut attempts = 0;
                    const MAX_RETRIES: usize = 3;

                    loop {
                        attempts += 1;

                        let response = client
                            .put(&s3_url)
                            .header("Content-Type", "application/octet-stream")
                            .header("Content-Length", part_size.to_string())
                            .body(buffer.clone())
                            .send()
                            .await;

                        match response {
                            Ok(resp) if resp.status().is_success() => {
                                // Get ETag from response
                                let etag = resp
                                    .headers()
                                    .get("etag")
                                    .and_then(|v| v.to_str().ok())
                                    .map(|s| s.trim_matches('"').to_string())
                                    .unwrap_or_default();

                                // Update state atomically
                                {
                                    let mut state_guard = state_mutex.lock().await;
                                    state_guard.completed_parts.push(part_num);
                                    state_guard.part_etags.insert(part_num, etag);
                                    if let Err(e) = state_guard.save() {
                                        eprintln!("Warning: Failed to save upload state: {}", e);
                                    }
                                }

                                // Update progress bar
                                {
                                    let pb_guard = pb_arc.lock().await;
                                    pb_guard.set_position(end);
                                    pb_guard.set_message(format!(
                                        "Uploading {} ({} parts completed)",
                                        object_key, part_num
                                    ));
                                }

                                return Ok::<_, anyhow::Error>(part_num);
                            }
                            Ok(resp) => {
                                let status = resp.status();
                                let error_text = resp.text().await.unwrap_or_default();
                                if attempts >= MAX_RETRIES {
                                    anyhow::bail!(
                                        "Failed to upload part {} after {} attempts ({}): {}",
                                        part_num,
                                        attempts,
                                        status,
                                        error_text
                                    );
                                }
                                // Wait before retry with exponential backoff
                                let delay =
                                    std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                                tokio::time::sleep(delay).await;
                            }
                            Err(e) => {
                                if attempts >= MAX_RETRIES {
                                    anyhow::bail!(
                                        "Failed to upload part {} after {} attempts: {}",
                                        part_num,
                                        attempts,
                                        e
                                    );
                                }
                                // Wait before retry
                                let delay =
                                    std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                                tokio::time::sleep(delay).await;
                            }
                        }
                    }
                }
            })
            .collect();

        // Execute all upload tasks concurrently
        let mut upload_results = Vec::new();
        let mut upload_stream = upload_tasks;

        while let Some(result) = upload_stream.next().await {
            match result {
                Ok(part_num) => {
                    upload_results.push(part_num);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // Get the progress bar back from the Arc<Mutex<>>
        let pb = Arc::try_unwrap(pb_arc).unwrap().into_inner();

        // Complete the upload
        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &upload_key)
            .await?;

        // Clean up state file
        MultipartUploadState::delete(bucket_key, object_key)?;

        pb.finish_with_message(format!("Uploaded {} (multipart)", object_key));

        Ok(object_info)
    }

    /// Download an object from a bucket using S3 signed URLs (new API)
    pub async fn download_object(
        &self,
        bucket_key: &str,
        object_key: &str,
        output_path: &Path,
    ) -> Result<()> {
        // Step 1: Get signed S3 download URL
        let signed = self
            .get_signed_download_url(bucket_key, object_key, None)
            .await?;

        let download_url = signed
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL returned"))?;

        // Step 2: Download from S3 with retry logic
        let response = raps_kernel::http::execute_with_retry(&self.config.http_config, || {
            let client = self.http_client.clone();
            let url = download_url.clone();
            Box::pin(async move {
                client
                    .get(&url)
                    .send()
                    .await
                    .context("Failed to download from S3")
            })
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to download from S3 ({status}): {error_text}");
        }

        let total_size = signed
            .size
            .unwrap_or(response.content_length().unwrap_or(0));

        // Create progress bar
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({percent}%)")
                .unwrap()
                .progress_chars("█▓░"),
        );
        pb.set_message(format!("Downloading {}", object_key));

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

        pb.finish_with_message(format!("Downloaded {}", object_key));
        Ok(())
    }

    /// List objects in a bucket
    pub async fn list_objects(&self, bucket_key: &str) -> Result<Vec<ObjectItem>> {
        let token = self.auth.get_token().await?;
        let mut all_objects = Vec::new();
        let mut start_at: Option<String> = None;

        loop {
            let mut url = format!("{}/buckets/{}/objects", self.config.oss_url(), bucket_key);
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = raps_kernel::http::execute_with_retry(&self.config.http_config, || {
                let client = self.http_client.clone();
                let url = url.clone();
                let token = token.clone();
                Box::pin(async move {
                    client
                        .get(&url)
                        .bearer_auth(&token)
                        .send()
                        .await
                        .context("Failed to list objects")
                })
            })
            .await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list objects ({status}): {error_text}");
            }

            let response_text = response
                .text()
                .await
                .context("Failed to read objects response")?;

            let objects_response: ObjectsResponse = serde_json::from_str(&response_text)
                .with_context(|| format!("Failed to parse objects response: {}", response_text))?;

            all_objects.extend(objects_response.items);

            if objects_response.next.is_none() {
                break;
            }
            start_at = objects_response.next;
        }

        Ok(all_objects)
    }

    /// Delete an object from a bucket
    pub async fn delete_object(&self, bucket_key: &str, object_key: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.config.oss_url(),
            bucket_key,
            object_key
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to delete object")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to delete object ({status}): {error_text}");
        }

        Ok(())
    }

    /// Generate a base64-encoded URN for an object
    pub fn get_urn(&self, bucket_key: &str, object_key: &str) -> String {
        use base64::Engine;
        let object_id = format!("urn:adsk.objects:os.object:{}/{}", bucket_key, object_key);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(object_id)
    }

    /// Get a signed S3 URL for direct download (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    pub async fn get_signed_download_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3DownloadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3download",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        if let Some(mins) = minutes_expiration {
            url = format!("{}?minutesExpiration={}", url, mins);
        }

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get signed download URL")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to get signed download URL ({}): {}",
                status,
                error_text
            );
        }

        let signed: SignedS3DownloadResponse = response
            .json()
            .await
            .context("Failed to parse signed URL response")?;

        Ok(signed)
    }

    /// Get a signed S3 URL for direct upload (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    /// Returns an upload key that must be used to complete the upload.
    pub async fn get_signed_upload_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        parts: Option<u32>,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3UploadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
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
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get signed upload URL")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to get signed upload URL ({}): {}",
                status,
                error_text
            );
        }

        let signed: SignedS3UploadResponse = response
            .json()
            .await
            .context("Failed to parse signed URL response")?;

        Ok(signed)
    }

    /// Complete an S3 signed upload
    pub async fn complete_signed_upload(
        &self,
        bucket_key: &str,
        object_key: &str,
        upload_key: &str,
    ) -> Result<ObjectInfo> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        let body = serde_json::json!({
            "uploadKey": upload_key
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .context("Failed to complete signed upload")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to complete signed upload ({}): {}",
                status,
                error_text
            );
        }

        // Get response text for debugging
        let response_text = response
            .text()
            .await
            .context("Failed to read upload completion response")?;

        // Try to parse as ObjectInfo
        let object_info: ObjectInfo = serde_json::from_str(&response_text).with_context(|| {
            format!(
                "Failed to parse upload completion response: {}",
                response_text
            )
        })?;

        Ok(object_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;

    fn create_test_oss_client() -> OssClient {
        let config = Config {
            client_id: "test".to_string(),
            client_secret: "secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        OssClient::new(config, auth)
    }

    #[test]
    fn test_multipart_upload_state_constants() {
        assert_eq!(MultipartUploadState::DEFAULT_CHUNK_SIZE, 5 * 1024 * 1024);
        assert_eq!(MultipartUploadState::MAX_CHUNK_SIZE, 100 * 1024 * 1024);
        assert_eq!(MultipartUploadState::MULTIPART_THRESHOLD, 5 * 1024 * 1024);
    }

    #[test]
    fn test_multipart_upload_state_remaining_parts() {
        let state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: 20 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 4,
            completed_parts: vec![1, 3],
            part_etags: std::collections::HashMap::new(),
            upload_key: "test-key".to_string(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert_eq!(remaining, vec![2, 4]);
    }

    #[test]
    fn test_get_urn() {
        let client = create_test_oss_client();
        let urn = client.get_urn("my-bucket", "my-object.dwg");

        assert!(!urn.contains("urn:adsk.objects:os.object:"));
        assert!(!urn.contains("my-bucket"));
        assert!(!urn.contains("my-object.dwg"));
        assert!(!urn.contains("+"));
        assert!(!urn.contains("/"));
        assert!(!urn.contains("="));
    }

    #[test]
    fn test_retention_policy_display() {
        assert_eq!(RetentionPolicy::Transient.to_string(), "transient");
        assert_eq!(RetentionPolicy::Temporary.to_string(), "temporary");
        assert_eq!(RetentionPolicy::Persistent.to_string(), "persistent");
    }

    #[test]
    fn test_retention_policy_from_str() {
        assert_eq!(
            RetentionPolicy::from_str("transient"),
            Ok(RetentionPolicy::Transient)
        );
        assert_eq!(
            RetentionPolicy::from_str("TRANSIENT"),
            Ok(RetentionPolicy::Transient)
        );
        assert!(RetentionPolicy::from_str("invalid").is_err());
    }

    #[test]
    fn test_region_display() {
        assert_eq!(Region::US.to_string(), "US");
        assert_eq!(Region::EMEA.to_string(), "EMEA");
    }
}
