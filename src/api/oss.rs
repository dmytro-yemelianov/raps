//! Object Storage Service (OSS) API module
//!
//! Handles bucket and object operations for storing files in APS.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::AuthClient;
use crate::config::Config;

/// Bucket retention policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Region {
    US,
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedS3UploadResponse {
    /// Upload key to use for completion
    pub upload_key: String,
    /// Pre-signed S3 URLs for upload
    pub urls: Vec<String>,
    /// Expiration timestamp
    pub upload_expiration: Option<String>,
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
pub struct OssClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
}

impl OssClient {
    /// Create a new OSS client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self {
            config,
            auth,
            http_client: reqwest::Client::new(),
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

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token)
            .header("x-ads-region", region.to_string())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to create bucket")?;

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

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Failed to get bucket details")?;

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

    /// Upload a file to a bucket using S3 signed URLs (new API)
    pub async fn upload_object(
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
