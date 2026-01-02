// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! OSS types and enums

use serde::{Deserialize, Serialize};

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
    /// Bucket key (name)
    pub bucket_key: String,
    /// Retention policy key (transient, temporary, persistent)
    pub policy_key: String,
}

/// Bucket information returned from API
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub bucket_key: String,
    pub bucket_owner: String,
    pub created_date: u64,
    pub permissions: Vec<Permission>,
    pub policy_key: String,
}

/// Permission information for a bucket
#[derive(Debug, Clone, Deserialize)]
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

/// Object information
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectItem {
    pub bucket_key: String,
    pub object_key: String,
    pub object_id: String,
    #[serde(default)]
    pub sha1: Option<String>,
    pub size: u64,
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
