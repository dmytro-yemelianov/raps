// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Upload operations with parallel multipart support

use raps_kernel::{HttpClient, Result, RapsError, BucketKey, ObjectKey};
use crate::types::*;
use futures_util::stream::{FuturesUnordered, StreamExt};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::Semaphore;

/// Upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// Concurrency level for parallel uploads
    pub concurrency: usize,
    /// Chunk size for multipart uploads (bytes)
    pub chunk_size: u64,
    /// Whether to resume interrupted uploads
    pub resume: bool,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            concurrency: 5,
            chunk_size: 5 * 1024 * 1024, // 5MB
            resume: false,
        }
    }
}

/// Multipart upload state for resume capability
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

    /// Calculate which parts still need to be uploaded
    pub fn remaining_parts(&self) -> Vec<u32> {
        (1..=self.total_parts)
            .filter(|p| !self.completed_parts.contains(p))
            .collect()
    }
}

/// Upload client for OSS operations
pub struct UploadClient {
    http: HttpClient,
    base_url: String,
}

impl UploadClient {
    /// Create new upload client
    pub fn new(http: HttpClient, base_url: String) -> Self {
        Self { http, base_url }
    }

    /// Upload a file (automatically chooses single-part or multipart)
    pub async fn upload(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
        file_path: &Path,
        config: UploadConfig,
    ) -> Result<ObjectInfo> {
        let metadata = tokio::fs::metadata(file_path)
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to get file metadata: {}", e),
            })?;
        let file_size = metadata.len();

        // Use multipart upload for files larger than threshold
        if file_size > MultipartUploadState::MULTIPART_THRESHOLD {
            self.upload_parallel(bucket_key, object_key, file_path, config)
                .await
        } else {
            // TODO: Implement single-part upload
            Err(RapsError::Internal {
                message: "Single-part upload not yet implemented".to_string(),
            })
        }
    }

    /// Upload using parallel multipart chunks (Issue #70)
    ///
    /// This implementation uses FuturesUnordered to upload multiple chunks
    /// concurrently, significantly improving performance for large files.
    pub async fn upload_parallel(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
        file_path: &Path,
        config: UploadConfig,
    ) -> Result<ObjectInfo> {
        let metadata = tokio::fs::metadata(file_path)
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to get file metadata: {}", e),
            })?;
        let file_size = metadata.len();
        let file_mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let chunk_size = config.chunk_size;
        let total_parts = file_size.div_ceil(chunk_size) as u32;

        // TODO: Load existing state if resume is enabled
        // For now, always start fresh
        let state = MultipartUploadState {
            bucket_key: bucket_key.as_str().to_string(),
            object_key: object_key.as_str().to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            file_size,
            chunk_size,
            total_parts,
            completed_parts: Vec::new(),
            part_etags: std::collections::HashMap::new(),
            upload_key: String::new(), // TODO: Get from signed URL request
            started_at: chrono::Utc::now().timestamp(),
            file_mtime,
        };

        // TODO: Get signed upload URLs for all parts
        // For now, stub this
        let urls: Vec<String> = (0..total_parts)
            .map(|i| format!("https://s3.example.com/upload/part-{}", i + 1))
            .collect();

        // Open file for reading
        let file = Arc::new(tokio::sync::Mutex::new(
            File::open(file_path)
                .await
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to open file: {}", e),
                })?,
        ));

        // Create semaphore to limit concurrent uploads
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        let http_client = Arc::new(self.http.inner().clone());

        // Create futures for all parts
        let mut uploads = FuturesUnordered::new();

        for part_num in 1..=total_parts {
            let permit = semaphore.clone().acquire_owned().await
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to acquire semaphore permit: {}", e),
                })?;
            
            let part_index = (part_num - 1) as usize;
            let start = (part_num as u64 - 1) * chunk_size;
            let end = std::cmp::min(start + chunk_size, file_size);
            let part_size = end - start;
            let url = urls[part_index].clone();
            let file_clone = file.clone();
            let http_clone = http_client.clone();

            uploads.push(async move {
                let _permit = permit; // Hold permit until upload completes

                // Read chunk from file
                let mut file_guard = file_clone.lock().await;
                file_guard
                    .seek(SeekFrom::Start(start))
                    .await
                    .map_err(|e| RapsError::Internal {
                        message: format!("Failed to seek to part start: {}", e),
                    })?;

                let mut buffer = vec![0u8; part_size as usize];
                file_guard
                    .read_exact(&mut buffer)
                    .await
                    .map_err(|e| RapsError::Internal {
                        message: format!("Failed to read part data: {}", e),
                    })?;
                drop(file_guard); // Release file lock

                // Upload chunk to S3
                let response = http_clone
                    .put(&url)
                    .header("Content-Type", "application/octet-stream")
                    .header("Content-Length", part_size.to_string())
                    .body(buffer)
                    .send()
                    .await
                    .map_err(|e| RapsError::Network {
                        message: format!("Failed to upload part {}: {}", part_num, e),
                        source: Some(e),
                    })?;

                if !response.status().is_success() {
                    let status = response.status();
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(RapsError::Api {
                        message: format!("Failed to upload part {} ({}): {}", part_num, status, error_text),
                        status: Some(status.as_u16()),
                        source: None,
                    });
                }

                // Get ETag from response
                let etag = response
                    .headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.trim_matches('"').to_string())
                    .unwrap_or_default();

                Ok((part_num, etag))
            });
        }

        // Collect results as they complete
        let mut results: Vec<(u32, String)> = Vec::with_capacity(total_parts as usize);
        while let Some(result) = uploads.next().await {
            match result {
                Ok((part_num, etag)) => {
                    results.push((part_num, etag));
                }
                Err(e) => {
                    // If any part fails, we could retry or fail the whole upload
                    return Err(e);
                }
            }
        }

        // Sort results by part number to ensure correct order
        results.sort_by_key(|(part_num, _)| *part_num);

        // TODO: Complete the multipart upload with all ETags
        // TODO: Return ObjectInfo

        // For now, return stub
        Ok(ObjectInfo {
            bucket_key: bucket_key.as_str().to_string(),
            object_key: object_key.as_str().to_string(),
            object_id: String::new(),
            sha1: None,
            size: file_size,
            location: None,
            content_type: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_config_default() {
        let config = UploadConfig::default();
        assert_eq!(config.concurrency, 5);
        assert_eq!(config.chunk_size, 5 * 1024 * 1024);
        assert!(!config.resume);
    }

    #[test]
    fn test_upload_config_custom() {
        let config = UploadConfig {
            concurrency: 10,
            chunk_size: 10 * 1024 * 1024,
            resume: true,
        };
        assert_eq!(config.concurrency, 10);
        assert_eq!(config.chunk_size, 10 * 1024 * 1024);
        assert!(config.resume);
    }

    #[test]
    fn test_upload_config_clone() {
        let config = UploadConfig::default();
        let cloned = config.clone();
        assert_eq!(config.concurrency, cloned.concurrency);
        assert_eq!(config.chunk_size, cloned.chunk_size);
        assert_eq!(config.resume, cloned.resume);
    }

    #[test]
    fn test_multipart_upload_state_remaining_parts() {
        let state = MultipartUploadState {
            bucket_key: "test".to_string(),
            object_key: "file".to_string(),
            file_path: "/tmp/file".to_string(),
            file_size: 20 * 1024 * 1024, // 20MB
            chunk_size: 5 * 1024 * 1024, // 5MB chunks
            total_parts: 4,
            completed_parts: vec![1, 3],
            part_etags: std::collections::HashMap::new(),
            upload_key: String::new(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert_eq!(remaining, vec![2, 4]);
    }

    #[test]
    fn test_multipart_upload_state_no_remaining() {
        let state = MultipartUploadState {
            bucket_key: "test".to_string(),
            object_key: "file".to_string(),
            file_path: "/tmp/file".to_string(),
            file_size: 10 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 2,
            completed_parts: vec![1, 2],
            part_etags: std::collections::HashMap::new(),
            upload_key: String::new(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_multipart_upload_state_all_remaining() {
        let state = MultipartUploadState {
            bucket_key: "test".to_string(),
            object_key: "file".to_string(),
            file_path: "/tmp/file".to_string(),
            file_size: 15 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 3,
            completed_parts: vec![],
            part_etags: std::collections::HashMap::new(),
            upload_key: String::new(),
            started_at: 0,
            file_mtime: 0,
        };

        let remaining = state.remaining_parts();
        assert_eq!(remaining, vec![1, 2, 3]);
    }

    #[test]
    fn test_multipart_constants() {
        assert_eq!(MultipartUploadState::DEFAULT_CHUNK_SIZE, 5 * 1024 * 1024);
        assert_eq!(MultipartUploadState::MAX_CHUNK_SIZE, 100 * 1024 * 1024);
        assert_eq!(MultipartUploadState::MULTIPART_THRESHOLD, 5 * 1024 * 1024);
    }

    #[test]
    fn test_multipart_upload_state_serialization() {
        let state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: 20 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: 4,
            completed_parts: vec![1, 2],
            part_etags: {
                let mut m = std::collections::HashMap::new();
                m.insert(1, "etag1".to_string());
                m.insert(2, "etag2".to_string());
                m
            },
            upload_key: "upload-key-123".to_string(),
            started_at: 1704067200,
            file_mtime: 1704067000,
        };

        let json = serde_json::to_string(&state).expect("serialize");
        assert!(json.contains("test-bucket"));
        assert!(json.contains("test-object"));
        assert!(json.contains("etag1"));

        let deserialized: MultipartUploadState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.bucket_key, "test-bucket");
        assert_eq!(deserialized.total_parts, 4);
        assert_eq!(deserialized.completed_parts, vec![1, 2]);
    }

    #[test]
    fn test_concurrency_limits() {
        // Test that concurrency value is within reasonable bounds
        let config = UploadConfig::default();
        assert!(config.concurrency >= 1, "Concurrency should be at least 1");
        assert!(config.concurrency <= 20, "Concurrency should not exceed 20");
    }

    #[test]
    fn test_chunk_size_bounds() {
        // Test that chunk size is within S3 limits
        let min_chunk = MultipartUploadState::DEFAULT_CHUNK_SIZE;
        let max_chunk = MultipartUploadState::MAX_CHUNK_SIZE;

        // S3 minimum: 5MB, maximum: 5GB (we cap at 100MB for practicality)
        assert!(min_chunk >= 5 * 1024 * 1024, "Min chunk must be >= 5MB");
        assert!(max_chunk <= 5 * 1024 * 1024 * 1024, "Max chunk must be <= 5GB");
    }
}
