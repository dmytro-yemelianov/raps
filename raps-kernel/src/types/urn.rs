// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! URN (Uniform Resource Name) type for APS resources

use crate::error::{RapsError, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Base64-URL encoded URN for translation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Urn(String);

impl Urn {
    /// Create URN from bucket/object path
    pub fn from_path(bucket: &str, object: &str) -> Self {
        let path = format!("urn:adsk.objects:os.object:{}/{}", bucket, object);
        let encoded = URL_SAFE_NO_PAD.encode(path.as_bytes());
        Self(encoded)
    }

    /// Decode URN to original path
    pub fn decode(&self) -> Result<String> {
        let bytes = URL_SAFE_NO_PAD
            .decode(&self.0)
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to decode URN: {}", e),
            })?;
        String::from_utf8(bytes).map_err(|e| RapsError::Internal {
            message: format!("Invalid UTF-8 in URN: {}", e),
        })
    }

    /// Get the encoded URN string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Urn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Urn {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Urn {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urn_from_path() {
        let urn = Urn::from_path("my-bucket", "model.dwg");
        assert!(urn.as_str().starts_with("dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6"));
    }

    #[test]
    fn test_urn_decode() {
        let urn = Urn::from_path("my-bucket", "model.dwg");
        let decoded = urn.decode().unwrap();
        assert_eq!(decoded, "urn:adsk.objects:os.object:my-bucket/model.dwg");
    }

    #[test]
    fn test_urn_decode_invalid_base64() {
        let urn = Urn("invalid-base64!!!".to_string());
        let result = urn.decode();
        assert!(result.is_err());
    }

    #[test]
    fn test_urn_display() {
        let urn = Urn::from_path("bucket", "file");
        let display = format!("{}", urn);
        assert!(!display.is_empty());
    }

    #[test]
    fn test_urn_from_string() {
        let urn = Urn::from("test-urn".to_string());
        assert_eq!(urn.as_str(), "test-urn");
    }

    #[test]
    fn test_urn_from_str() {
        let urn = Urn::from("test-urn");
        assert_eq!(urn.as_str(), "test-urn");
    }

    #[test]
    fn test_urn_serialize() {
        let urn = Urn::from("test-urn");
        let json = serde_json::to_string(&urn);
        assert!(json.is_ok());
    }

    #[test]
    fn test_urn_deserialize() {
        let json = r#""test-urn""#;
        let urn: std::result::Result<Urn, _> = serde_json::from_str(json);
        assert!(urn.is_ok());
        assert_eq!(urn.unwrap().as_str(), "test-urn");
    }

    #[test]
    fn test_urn_equality() {
        let urn1 = Urn::from("test");
        let urn2 = Urn::from("test");
        let urn3 = Urn::from("different");
        assert_eq!(urn1, urn2);
        assert_ne!(urn1, urn3);
    }
}
