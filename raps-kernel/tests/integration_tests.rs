// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for raps-kernel
//!
//! These tests verify the kernel components work correctly together.

use raps_kernel::{BucketKey, Config, HttpClient, HttpClientConfig, ObjectKey, RapsError, Urn};

#[test]
fn test_bucket_key_validation() {
    // Valid bucket keys
    assert!(BucketKey::new("my-bucket").is_ok());
    assert!(BucketKey::new("bucket123").is_ok());
    assert!(BucketKey::new("a-b-c").is_ok());

    // Invalid bucket keys (too short)
    assert!(BucketKey::new("ab").is_err());

    // Invalid bucket keys (uppercase)
    assert!(BucketKey::new("MyBucket").is_err());
}

#[test]
fn test_object_key_creation() {
    // ObjectKey accepts any string (no validation)
    let key1 = ObjectKey::new("file.txt");
    assert_eq!(key1.as_str(), "file.txt");

    let key2 = ObjectKey::new("path/to/file.dwg");
    assert_eq!(key2.as_str(), "path/to/file.dwg");

    let key3 = ObjectKey::new("my-model.rvt");
    assert_eq!(key3.as_str(), "my-model.rvt");
}

#[test]
fn test_urn_encoding() {
    // Create URN from bucket/object path (this does base64 encoding)
    let urn = Urn::from_path("mybucket", "myfile.dwg");

    // URN should be base64 encoded
    let encoded = urn.as_str();
    assert!(encoded.starts_with("dXJuOmFkc2sub2JqZWN0czpvcy5vYmplY3Q6"));

    // Should be decodable
    let decoded = urn.decode();
    assert!(decoded.is_ok());
    assert_eq!(
        decoded.expect("decode"),
        "urn:adsk.objects:os.object:mybucket/myfile.dwg"
    );
}

#[test]
fn test_urn_from_string() {
    // Urn::from just wraps the string (for pre-encoded URNs)
    let urn = Urn::from("already-encoded-urn");
    assert_eq!(urn.as_str(), "already-encoded-urn");
}

#[test]
fn test_http_client_config_defaults() {
    let config = HttpClientConfig::default();

    assert!(config.timeout.as_secs() >= 30);
    assert!(config.connect_timeout.as_secs() >= 10);
    assert!(config.max_retries >= 3);
}

#[test]
fn test_http_client_creation() {
    let config = HttpClientConfig::default();
    let result = HttpClient::new(config);

    assert!(result.is_ok());
}

#[test]
fn test_error_types() {
    // Test error creation and display
    let api_error = RapsError::Api {
        message: "Not found".to_string(),
        status: Some(404),
        source: None,
    };
    assert!(api_error.to_string().contains("Not found"));

    let not_found = RapsError::NotFound {
        resource: "bucket/object".to_string(),
    };
    assert!(not_found.to_string().contains("bucket/object"));
}

#[test]
fn test_config_struct() {
    // Config struct should have the expected fields
    let config = Config {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
    };

    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
}

#[test]
fn test_config_endpoints() {
    let config = Config {
        client_id: "test".to_string(),
        client_secret: "test".to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
    };

    // auth_url method should return the correct endpoint
    let auth_url = config.auth_url();
    assert!(auth_url.contains("developer.api.autodesk.com"));
    assert!(auth_url.contains("/authentication/"));
}

// Tests that require credentials are marked as ignored
// Run with: cargo test --ignored -- --test-threads=1

#[test]
#[ignore = "requires APS credentials"]
fn test_auth_client_token_fetch() {
    // This test requires real credentials
    // It's ignored by default
}

#[test]
#[ignore = "requires APS credentials"]
fn test_auth_client_refresh() {
    // This test requires real credentials
    // It's ignored by default
}
