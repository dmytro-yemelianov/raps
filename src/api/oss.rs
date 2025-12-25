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
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};

use super::AuthClient;
use crate::config::Config;

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

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "transient" => Some(Self::Transient),
            "temporary" => Some(Self::Temporary),
            "persistent" => Some(Self::Persistent),
            _ => None,
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
        crate::logging::log_request("POST", &url);

        // Use retry logic for bucket creation
        let http_config = crate::http::HttpClientConfig::default();
        let response = crate::http::execute_with_retry(&http_config, || {
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
        crate::logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to create bucket ({}): {}", status, error_text);
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

            let response = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .header("x-ads-region", region.to_string())
                .send()
                .await
                .context("Failed to list buckets")?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list buckets ({}): {}", status, error_text);
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
        crate::logging::log_request("GET", &url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get bucket details")?;

        // Log response in verbose/debug mode
        crate::logging::log_response(response.status().as_u16(), &url);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Failed to get bucket details ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to delete bucket ({}): {}", status, error_text);
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
        let total_parts = ((file_size + chunk_size - 1) / chunk_size) as u32;

        // Container for signed URLs (either loaded from fresh request or empty if resuming)
        // We really only need them if we are starting fresh OR if we need to refresh them.
        // For simplicity/robustness:
        // 1. If resuming, state exists. We might need new URLs if old ones expired (which we don't track well here, so maybe safer to fetch).
        // 2. If new, we fetch URLs.

        // To optimize: If we JUST created the state, we have valid URLs. We should pass them through.

        let (mut state, initial_urls) = if resume {
            if let Some(existing_state) = MultipartUploadState::load(bucket_key, object_key)? {
                if existing_state.can_resume(file_path) {
                    crate::logging::log_verbose(&format!(
                        "Resuming upload: {}/{} completed parts",
                        existing_state.completed_parts.len(),
                        existing_state.total_parts
                    ));
                    (existing_state, None)
                } else {
                    crate::logging::log_verbose("File changed since last upload, starting fresh");
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

        // If we have initial_urls, use them. Otherwise (resuming), fetch new ones.
        // NOTE: If we are resuming, the URLs in state (if we stored them, which we don't currently) might be expired.
        // It's safest to fetch fresh URLs if we are resuming.
        // If we just created the state, we have fresh URLs in `initial_urls`.

        let urls = if let Some(u) = initial_urls {
            u
        } else {
            // Fetch fresh URLs for the remaining parts
            // Ideally we'd only fetch for remaining parts, but API returns all or we need logic to map
            // Let's just fetch all and index carefully.
            let signed = self
                .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                .await?;
            signed.urls
        };

        // Open file for reading
        let mut file = File::open(file_path)
            .await
            .context("Failed to open file for upload")?;

        // Upload remaining parts
        for part_num in remaining_parts {
            let part_index = (part_num - 1) as usize;
            let start = (part_num as u64 - 1) * chunk_size;
            let end = std::cmp::min(start + chunk_size, file_size);
            let part_size = end - start;

            // Seek to part start
            file.seek(SeekFrom::Start(start)).await?;

            // Read part data
            let mut buffer = vec![0u8; part_size as usize];
            file.read_exact(&mut buffer).await?;

            // Upload part
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

            // Get ETag from response
            let etag = response
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_default();

            // Update state
            state.completed_parts.push(part_num);
            state.part_etags.insert(part_num, etag);
            state.save()?;

            pb.set_position(end);
            pb.set_message(format!(
                "Uploading {} ({}/{})",
                object_key,
                state.completed_parts.len(),
                total_parts
            ));
        }

        // Complete the upload
        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &state.upload_key)
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

        // Step 2: Download from S3
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

            let response = self
                .http_client
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .context("Failed to list objects")?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                anyhow::bail!("Failed to list objects ({}): {}", status, error_text);
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
            anyhow::bail!("Failed to delete object ({}): {}", status, error_text);
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
    use crate::api::AuthClient;
    use crate::config::Config;

    fn create_test_oss_client() -> OssClient {
        let config = Config {
            client_id: "test".to_string(),
            client_secret: "secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
        };
        let auth = AuthClient::new(config.clone());
        OssClient::new(config, auth)
    }

    // ============== MULTIPART UPLOAD STATE TESTS ==============

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
            file_size: 20 * 1024 * 1024, // 20MB
            chunk_size: 5 * 1024 * 1024, // 5MB chunks
            total_parts: 4,
            completed_parts: vec![1, 3], // Parts 1 and 3 done
            part_etags: std::collections::HashMap::new(),
            upload_key: "test-key".to_string(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert_eq!(remaining, vec![2, 4]); // Parts 2 and 4 remaining
    }

    #[test]
    fn test_multipart_upload_state_all_complete() {
        let state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: 10 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 2,
            completed_parts: vec![1, 2],
            part_etags: std::collections::HashMap::new(),
            upload_key: "test-key".to_string(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_get_urn() {
        let client = create_test_oss_client();
        let urn = client.get_urn("my-bucket", "my-object.dwg");

        // URN should be base64 encoded
        assert!(!urn.contains("urn:adsk.objects:os.object:"));
        assert!(!urn.contains("my-bucket"));
        assert!(!urn.contains("my-object.dwg"));

        // Should be valid base64 URL-safe encoding
        assert!(!urn.contains("+"));
        assert!(!urn.contains("/"));
        assert!(!urn.contains("="));
    }

    #[test]
    fn test_get_urn_with_special_characters() {
        let client = create_test_oss_client();
        let urn = client.get_urn("bucket-with-dashes", "folder/file name with spaces.txt");

        // Should handle special characters in object key
        assert!(!urn.is_empty());
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
            Some(RetentionPolicy::Transient)
        );
        assert_eq!(
            RetentionPolicy::from_str("TRANSIENT"),
            Some(RetentionPolicy::Transient)
        );
        assert_eq!(
            RetentionPolicy::from_str("temporary"),
            Some(RetentionPolicy::Temporary)
        );
        assert_eq!(
            RetentionPolicy::from_str("TEMPORARY"),
            Some(RetentionPolicy::Temporary)
        );
        assert_eq!(
            RetentionPolicy::from_str("persistent"),
            Some(RetentionPolicy::Persistent)
        );
        assert_eq!(
            RetentionPolicy::from_str("PERSISTENT"),
            Some(RetentionPolicy::Persistent)
        );
        assert_eq!(RetentionPolicy::from_str("invalid"), None);
        assert_eq!(RetentionPolicy::from_str(""), None);
    }

    #[test]
    fn test_retention_policy_all() {
        let all = RetentionPolicy::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&RetentionPolicy::Transient));
        assert!(all.contains(&RetentionPolicy::Temporary));
        assert!(all.contains(&RetentionPolicy::Persistent));
    }

    #[test]
    fn test_region_display() {
        assert_eq!(Region::US.to_string(), "US");
        assert_eq!(Region::EMEA.to_string(), "EMEA");
    }

    #[test]
    fn test_region_all() {
        let all = Region::all();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&Region::US));
        assert!(all.contains(&Region::EMEA));
    }
}
