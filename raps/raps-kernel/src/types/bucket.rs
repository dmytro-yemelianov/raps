// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bucket key type with validation

use crate::error::{RapsError, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Bucket key with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BucketKey(String);

impl BucketKey {
    /// Create validated bucket key (lowercase alphanumeric + hyphens, 3-128 chars)
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.len() < 3 || key.len() > 128 {
            return Err(RapsError::Config {
                message: "Bucket key must be 3-128 characters".to_string(),
            });
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(RapsError::Config {
                message: "Bucket key must be lowercase alphanumeric with hyphens".to_string(),
            });
        }
        Ok(Self(key))
    }

    /// Get the bucket key as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BucketKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<BucketKey> for String {
    fn from(key: BucketKey) -> Self {
        key.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_bucket_key() {
        let key = BucketKey::new("my-bucket-123").unwrap();
        assert_eq!(key.as_str(), "my-bucket-123");
    }

    #[test]
    fn test_invalid_bucket_key_too_short() {
        assert!(BucketKey::new("ab").is_err());
    }

    #[test]
    fn test_invalid_bucket_key_too_long() {
        let long_key = "a".repeat(129);
        assert!(BucketKey::new(long_key).is_err());
    }

    #[test]
    fn test_invalid_bucket_key_uppercase() {
        assert!(BucketKey::new("MyBucket").is_err());
    }

    #[test]
    fn test_invalid_bucket_key_special_chars() {
        assert!(BucketKey::new("my_bucket").is_err());
        assert!(BucketKey::new("my.bucket").is_err());
        assert!(BucketKey::new("my bucket").is_err());
    }

    #[test]
    fn test_valid_bucket_key_min_length() {
        let key = BucketKey::new("abc").unwrap();
        assert_eq!(key.as_str(), "abc");
    }

    #[test]
    fn test_valid_bucket_key_max_length() {
        let long_key = "a".repeat(128);
        let key = BucketKey::new(&long_key).unwrap();
        assert_eq!(key.as_str(), long_key);
    }

    #[test]
    fn test_valid_bucket_key_with_numbers() {
        let key = BucketKey::new("bucket123").unwrap();
        assert_eq!(key.as_str(), "bucket123");
    }

    #[test]
    fn test_valid_bucket_key_with_hyphens() {
        let key = BucketKey::new("my-bucket-name").unwrap();
        assert_eq!(key.as_str(), "my-bucket-name");
    }

    #[test]
    fn test_bucket_key_display() {
        let key = BucketKey::new("test-bucket").unwrap();
        assert_eq!(format!("{}", key), "test-bucket");
    }

    #[test]
    fn test_bucket_key_to_string() {
        let key = BucketKey::new("test-bucket").unwrap();
        let s: String = key.into();
        assert_eq!(s, "test-bucket");
    }

    #[test]
    fn test_bucket_key_serialize() {
        let key = BucketKey::new("test-bucket").unwrap();
        let json = serde_json::to_string(&key);
        assert!(json.is_ok());
    }

    #[test]
    fn test_bucket_key_deserialize() {
        let json = r#""test-bucket""#;
        let key: std::result::Result<BucketKey, _> = serde_json::from_str(json);
        assert!(key.is_ok());
        assert_eq!(key.unwrap().as_str(), "test-bucket");
    }

    #[test]
    fn test_bucket_key_equality() {
        let key1 = BucketKey::new("test").unwrap();
        let key2 = BucketKey::new("test").unwrap();
        let key3 = BucketKey::new("other").unwrap();
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }
}
