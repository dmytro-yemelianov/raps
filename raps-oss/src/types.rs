// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Type definitions for the OSS API module.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;

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

/// Result from a single region query (used by streaming bucket listing).
#[derive(Debug)]
pub struct RegionResult {
    pub region: Region,
    pub buckets: anyhow::Result<Vec<BucketItem>>,
    pub elapsed: std::time::Duration,
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

    /// Get the cache directory used for upload state files
    pub fn cache_dir() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "autodesk", "raps")
            .context("Failed to get project directories")?;
        let dir = proj_dirs.cache_dir().to_path_buf();
        Ok(dir)
    }

    /// List all pending upload state files, returning parsed states.
    /// Corrupt files are returned as Err entries.
    pub fn list_all() -> Result<Vec<(PathBuf, Result<Self>)>> {
        let cache_dir = Self::cache_dir()?;
        if !cache_dir.exists() {
            return Ok(Vec::new());
        }
        let mut results = Vec::new();
        for entry in std::fs::read_dir(&cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.starts_with("upload_") && f.ends_with(".json"))
            {
                let parsed = std::fs::read_to_string(&path)
                    .context("Failed to read state file")
                    .and_then(|json| {
                        serde_json::from_str::<Self>(&json).context("Failed to parse state file")
                    });
                results.push((path, parsed));
            }
        }
        Ok(results)
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

/// Extended object metadata returned by object details endpoint
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectDetails {
    /// Bucket key
    pub bucket_key: String,
    /// Object key (filename)
    pub object_key: String,
    /// Object ID (URN format)
    pub object_id: String,
    /// SHA-1 hash of the object (may be null immediately after upload while APS computes it)
    #[serde(default)]
    pub sha1: Option<String>,
    /// Object size in bytes
    pub size: u64,
    /// MIME content type
    pub content_type: String,
    /// Content disposition header value
    #[serde(default)]
    pub content_disposition: Option<String>,
    /// Creation timestamp (ISO 8601)
    #[serde(alias = "createdDate")]
    pub created_date: Option<String>,
    /// Last modified timestamp (ISO 8601)
    #[serde(alias = "lastModifiedDate")]
    pub last_modified_date: Option<String>,
    /// Location URL
    #[serde(default)]
    pub location: Option<String>,
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

// ============== BATCH OPERATION TYPES ==============

/// Result of a batch operation with per-item tracking
#[derive(Debug, Serialize)]
pub struct BatchResult<T: std::fmt::Debug> {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<BatchItemResult<T>>,
}

/// Result of a single item within a batch operation
#[derive(Debug, Serialize)]
pub struct BatchItemResult<T: std::fmt::Debug> {
    pub key: String,
    #[serde(skip)]
    pub result: std::result::Result<T, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_region_all() {
        let regions = Region::all();
        assert_eq!(regions.len(), 2);
        assert!(regions.contains(&Region::US));
        assert!(regions.contains(&Region::EMEA));
    }

    #[test]
    fn test_retention_policy_all() {
        let policies = RetentionPolicy::all();
        assert_eq!(policies.len(), 3);
        assert!(policies.contains(&RetentionPolicy::Transient));
        assert!(policies.contains(&RetentionPolicy::Temporary));
        assert!(policies.contains(&RetentionPolicy::Persistent));
    }

    #[test]
    fn test_retention_policy_temporary() {
        assert_eq!(
            RetentionPolicy::from_str("temporary"),
            Ok(RetentionPolicy::Temporary)
        );
        assert_eq!(
            RetentionPolicy::from_str("TEMPORARY"),
            Ok(RetentionPolicy::Temporary)
        );
    }

    #[test]
    fn test_retention_policy_persistent() {
        assert_eq!(
            RetentionPolicy::from_str("persistent"),
            Ok(RetentionPolicy::Persistent)
        );
        assert_eq!(
            RetentionPolicy::from_str("PERSISTENT"),
            Ok(RetentionPolicy::Persistent)
        );
    }

    #[test]
    fn test_multipart_upload_state_chunk_calculation() {
        // File of 12 MB with 5 MB chunks = 3 parts
        let file_size: u64 = 12 * 1024 * 1024;
        let chunk_size = MultipartUploadState::DEFAULT_CHUNK_SIZE;
        let total_parts = file_size.div_ceil(chunk_size);
        assert_eq!(total_parts, 3);
    }

    #[test]
    fn test_multipart_upload_state_all_parts_remaining() {
        let state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: 15 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 3,
            completed_parts: vec![], // No parts completed
            part_etags: std::collections::HashMap::new(),
            upload_key: "test-key".to_string(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert_eq!(remaining, vec![1, 2, 3]);
    }

    #[test]
    fn test_multipart_upload_state_no_parts_remaining() {
        let state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: 15 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 3,
            completed_parts: vec![1, 2, 3], // All parts completed
            part_etags: std::collections::HashMap::new(),
            upload_key: "test-key".to_string(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_create_bucket_request_serialization() {
        let request = CreateBucketRequest {
            bucket_key: "test-bucket".to_string(),
            policy_key: "transient".to_string(),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["bucketKey"], "test-bucket");
        assert_eq!(json["policyKey"], "transient");
    }

    #[test]
    fn test_bucket_deserialization() {
        let json = r#"{
            "bucketKey": "test-bucket",
            "bucketOwner": "test-owner",
            "createdDate": 1609459200000,
            "permissions": [{"authId": "test-auth", "access": "full"}],
            "policyKey": "transient"
        }"#;

        let bucket: Bucket = serde_json::from_str(json).unwrap();
        assert_eq!(bucket.bucket_key, "test-bucket");
        assert_eq!(bucket.bucket_owner, "test-owner");
        assert_eq!(bucket.policy_key, "transient");
        assert_eq!(bucket.permissions.len(), 1);
    }

    #[test]
    fn test_buckets_response_deserialization() {
        let json = r#"{
            "items": [
                {"bucketKey": "bucket1", "createdDate": 1609459200000, "policyKey": "transient"},
                {"bucketKey": "bucket2", "createdDate": 1609459200000, "policyKey": "persistent"}
            ],
            "next": "bucket3"
        }"#;

        let response: BucketsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].bucket_key, "bucket1");
        assert_eq!(response.next, Some("bucket3".to_string()));
    }

    #[test]
    fn test_buckets_response_no_next() {
        let json = r#"{
            "items": [
                {"bucketKey": "bucket1", "createdDate": 1609459200000, "policyKey": "transient"}
            ]
        }"#;

        let response: BucketsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.items.len(), 1);
        assert!(response.next.is_none());
    }

    #[test]
    fn test_object_info_deserialization() {
        let json = r#"{
            "bucketKey": "test-bucket",
            "objectKey": "test-object.dwg",
            "objectId": "urn:adsk.objects:os.object:test-bucket/test-object.dwg",
            "sha1": "abc123",
            "size": 1024,
            "location": "https://example.com/object"
        }"#;

        let object: ObjectInfo = serde_json::from_str(json).unwrap();
        assert_eq!(object.bucket_key, "test-bucket");
        assert_eq!(object.object_key, "test-object.dwg");
        assert_eq!(object.size, 1024);
    }

    #[test]
    fn test_objects_response_deserialization() {
        let json = r#"{
            "items": [
                {"bucketKey": "bucket", "objectKey": "file1.dwg", "objectId": "urn:1", "size": 100},
                {"bucketKey": "bucket", "objectKey": "file2.rvt", "objectId": "urn:2", "size": 200}
            ],
            "next": "file3.dwg"
        }"#;

        let response: ObjectsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.items.len(), 2);
        assert_eq!(response.items[0].object_key, "file1.dwg");
        assert_eq!(response.items[1].size, 200);
    }

    #[test]
    fn test_signed_s3_download_response_deserialization() {
        let json = r#"{
            "url": "https://s3.amazonaws.com/signed-url",
            "size": 1048576,
            "sha1": "abc123"
        }"#;

        let response: SignedS3DownloadResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            response.url,
            Some("https://s3.amazonaws.com/signed-url".to_string())
        );
        assert_eq!(response.size, Some(1048576));
    }

    #[test]
    fn test_signed_s3_upload_response_deserialization() {
        let json = r#"{
            "uploadKey": "upload-key-123",
            "urls": ["https://s3.amazonaws.com/part1", "https://s3.amazonaws.com/part2"],
            "uploadExpiration": "2024-01-15T12:00:00Z"
        }"#;

        let response: SignedS3UploadResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.upload_key, "upload-key-123");
        assert_eq!(response.urls.len(), 2);
    }

    #[test]
    fn test_retention_policy_serialization() {
        let policy = RetentionPolicy::Persistent;
        let json = serde_json::to_value(policy).unwrap();
        assert_eq!(json, "persistent");
    }

    #[test]
    fn test_region_serialization() {
        let region = Region::EMEA;
        let json = serde_json::to_value(region).unwrap();
        assert_eq!(json, "EMEA");
    }

    #[test]
    fn test_batch_result_summary() {
        let result: BatchResult<ObjectDetails> = BatchResult {
            total: 3,
            succeeded: 2,
            failed: 1,
            results: vec![],
        };
        assert_eq!(result.total, 3);
        assert_eq!(result.succeeded, 2);
        assert_eq!(result.failed, 1);
    }

    #[test]
    fn test_batch_item_result_success_and_failure() {
        let success: BatchItemResult<String> = BatchItemResult {
            key: "file.txt".to_string(),
            result: Ok("done".to_string()),
        };
        assert!(success.result.is_ok());

        let failure: BatchItemResult<String> = BatchItemResult {
            key: "missing.txt".to_string(),
            result: Err("not found".to_string()),
        };
        assert!(failure.result.is_err());
        assert_eq!(failure.result.unwrap_err(), "not found");
    }

    // ==================== Contract Tests ====================

    #[test]
    fn test_contract_buckets_list() {
        let json = include_str!("../../tests/fixtures/buckets_list.json");
        let response: BucketsResponse = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }

    #[test]
    fn test_contract_objects_list() {
        let json = include_str!("../../tests/fixtures/objects_list.json");
        let response: ObjectsResponse = serde_json::from_str(json).unwrap();
        insta::assert_debug_snapshot!(response);
    }
}
