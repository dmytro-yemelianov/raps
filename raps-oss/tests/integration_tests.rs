// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for raps-oss
//!
//! These tests verify OSS service functionality.

use raps_oss::{UploadConfig, MultipartUploadState};
use raps_kernel::{BucketKey, ObjectKey};

#[test]
fn test_upload_config_defaults() {
    let config = UploadConfig::default();
    
    // Default chunk size should be 5MB
    assert_eq!(config.chunk_size, 5 * 1024 * 1024);
    
    // Default concurrency should be reasonable
    assert!(config.concurrency >= 1);
    assert!(config.concurrency <= 16);
    
    // Resume disabled by default
    assert!(!config.resume);
}

#[test]
fn test_upload_config_custom() {
    let config = UploadConfig {
        chunk_size: 10 * 1024 * 1024, // 10MB
        concurrency: 8,
        resume: true,
    };
    
    assert_eq!(config.chunk_size, 10 * 1024 * 1024);
    assert_eq!(config.concurrency, 8);
    assert!(config.resume);
}

#[test]
fn test_multipart_upload_state_remaining_parts_all() {
    let state = MultipartUploadState {
        bucket_key: "test".to_string(),
        object_key: "file.bin".to_string(),
        file_path: "/tmp/test.bin".to_string(),
        file_size: 25 * 1024 * 1024, // 25MB
        chunk_size: 5 * 1024 * 1024, // 5MB chunks
        total_parts: 5,
        completed_parts: vec![],
        part_etags: std::collections::HashMap::new(),
        upload_key: String::new(),
        started_at: 0,
        file_mtime: 0,
    };

    let remaining = state.remaining_parts();
    assert_eq!(remaining, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_multipart_upload_state_remaining_parts_some_done() {
    let state = MultipartUploadState {
        bucket_key: "test".to_string(),
        object_key: "file.bin".to_string(),
        file_path: "/tmp/test.bin".to_string(),
        file_size: 25 * 1024 * 1024,
        chunk_size: 5 * 1024 * 1024,
        total_parts: 5,
        completed_parts: vec![1, 3, 5],
        part_etags: std::collections::HashMap::new(),
        upload_key: String::new(),
        started_at: 0,
        file_mtime: 0,
    };

    let remaining = state.remaining_parts();
    assert_eq!(remaining, vec![2, 4]);
}

#[test]
fn test_multipart_upload_state_remaining_parts_all_done() {
    let state = MultipartUploadState {
        bucket_key: "test".to_string(),
        object_key: "file.bin".to_string(),
        file_path: "/tmp/test.bin".to_string(),
        file_size: 15 * 1024 * 1024,
        chunk_size: 5 * 1024 * 1024,
        total_parts: 3,
        completed_parts: vec![1, 2, 3],
        part_etags: std::collections::HashMap::new(),
        upload_key: String::new(),
        started_at: 0,
        file_mtime: 0,
    };

    let remaining = state.remaining_parts();
    assert!(remaining.is_empty());
}

#[test]
fn test_multipart_constants() {
    // Verify important constants
    assert_eq!(MultipartUploadState::DEFAULT_CHUNK_SIZE, 5 * 1024 * 1024);
    assert_eq!(MultipartUploadState::MAX_CHUNK_SIZE, 100 * 1024 * 1024);
    assert_eq!(MultipartUploadState::MULTIPART_THRESHOLD, 5 * 1024 * 1024);
}

#[test]
fn test_bucket_key_in_context() {
    // Verify BucketKey works with OSS operations
    let key = BucketKey::new("test-bucket-name").unwrap();
    assert_eq!(key.as_str(), "test-bucket-name");
}

#[test]
fn test_object_key_in_context() {
    // Verify ObjectKey works with OSS operations (no validation)
    let key = ObjectKey::new("path/to/file.dwg");
    assert_eq!(key.as_str(), "path/to/file.dwg");
}

// Tests requiring credentials are ignored by default
#[test]
#[ignore = "requires APS credentials"]
fn test_list_buckets() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_create_bucket() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_upload_object() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_multipart_upload() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_download_object() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_signed_url_generation() {
    // Requires real credentials
}
