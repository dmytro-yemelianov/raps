// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object key type

use serde::{Deserialize, Serialize};
use std::fmt;

/// Object key (filename/path in bucket)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectKey(String);

impl ObjectKey {
    /// Create object key
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Get the object key as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ObjectKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for ObjectKey {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for ObjectKey {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_key_new() {
        let key = ObjectKey::new("file.txt");
        assert_eq!(key.as_str(), "file.txt");
    }

    #[test]
    fn test_object_key_from_string() {
        let key = ObjectKey::from("file.txt".to_string());
        assert_eq!(key.as_str(), "file.txt");
    }

    #[test]
    fn test_object_key_from_str() {
        let key = ObjectKey::from("file.txt");
        assert_eq!(key.as_str(), "file.txt");
    }

    #[test]
    fn test_object_key_display() {
        let key = ObjectKey::new("file.txt");
        assert_eq!(format!("{}", key), "file.txt");
    }

    #[test]
    fn test_object_key_serialize() {
        let key = ObjectKey::new("file.txt");
        let json = serde_json::to_string(&key);
        assert!(json.is_ok());
    }

    #[test]
    fn test_object_key_deserialize() {
        let json = r#""file.txt""#;
        let key: std::result::Result<ObjectKey, _> = serde_json::from_str(json);
        assert!(key.is_ok());
        assert_eq!(key.unwrap().as_str(), "file.txt");
    }

    #[test]
    fn test_object_key_equality() {
        let key1 = ObjectKey::new("file.txt");
        let key2 = ObjectKey::new("file.txt");
        let key3 = ObjectKey::new("other.txt");
        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_object_key_with_path() {
        let key = ObjectKey::new("folder/subfolder/file.txt");
        assert_eq!(key.as_str(), "folder/subfolder/file.txt");
    }
}
