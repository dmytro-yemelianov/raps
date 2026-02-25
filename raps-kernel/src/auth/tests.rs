// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Tests for authentication module

use super::types::*;
use super::*;
use crate::http::HttpClientConfig;
use crate::types::StoredToken;
use std::time::{Duration, Instant};

#[test]
fn test_cached_token_validity() {
    let token = CachedToken {
        access_token: "test".to_string(),
        expires_at: Instant::now() + Duration::from_secs(3600),
    };
    assert!(token.is_valid());

    let expired_token = CachedToken {
        access_token: "test".to_string(),
        expires_at: Instant::now() - Duration::from_secs(1),
    };
    assert!(!expired_token.is_valid());
}

#[test]
fn test_cached_token_near_expiry() {
    // Token expiring in less than 60 seconds should be invalid
    let token = CachedToken {
        access_token: "test".to_string(),
        expires_at: Instant::now() + Duration::from_secs(30),
    };
    assert!(!token.is_valid());

    // Token expiring in more than 60 seconds should be valid
    let token = CachedToken {
        access_token: "test".to_string(),
        expires_at: Instant::now() + Duration::from_secs(120),
    };
    assert!(token.is_valid());
}

#[test]
fn test_stored_token_validity() {
    let now = chrono::Utc::now().timestamp();

    // Valid token (expires in 1 hour)
    let token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: Some("refresh".to_string()),
        expires_at: now + 3600,
        scopes: vec!["data:read".to_string()],
    };
    assert!(token.is_valid());

    // Expired token
    let expired_token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: Some("refresh".to_string()),
        expires_at: now - 100,
        scopes: vec!["data:read".to_string()],
    };
    assert!(!expired_token.is_valid());

    // Token expiring soon (within 60 seconds) should be invalid
    let soon_expiring = StoredToken {
        access_token: "test".to_string(),
        refresh_token: Some("refresh".to_string()),
        expires_at: now + 30,
        scopes: vec!["data:read".to_string()],
    };
    assert!(!soon_expiring.is_valid());
}

#[test]
fn test_stored_token_without_refresh() {
    let now = chrono::Utc::now().timestamp();
    let token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: now + 3600,
        scopes: vec!["data:read".to_string()],
    };
    // Should still be valid if not expired
    assert!(token.is_valid());
}

#[test]
fn test_token_response_serialization() {
    let token = TokenResponse {
        access_token: "test_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some("refresh_token".to_string()),
        scope: None,
    };

    let json = serde_json::to_string(&token).unwrap();
    assert!(json.contains("test_token"));
    assert!(json.contains("Bearer"));
    assert!(json.contains("refresh_token"));

    let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.access_token, "test_token");
    assert_eq!(deserialized.token_type, "Bearer");
    assert_eq!(deserialized.expires_in, 3600);
    assert_eq!(
        deserialized.refresh_token,
        Some("refresh_token".to_string())
    );
}

#[test]
fn test_token_response_without_refresh() {
    let token = TokenResponse {
        access_token: "test_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
        scope: None,
    };

    let json = serde_json::to_string(&token).unwrap();
    // refresh_token should be omitted when None
    assert!(!json.contains("refresh_token"));

    let deserialized: TokenResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.refresh_token, None);
}

#[test]
fn test_token_response_with_scope() {
    let token = TokenResponse {
        access_token: "test_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: None,
        scope: Some("data:read data:write".to_string()),
    };

    let json = serde_json::to_string(&token).unwrap();
    assert!(json.contains("scope"));
    assert!(json.contains("data:read"));
}

#[test]
fn test_user_info_deserialization() {
    let json = r#"{
        "sub": "user-id-123",
        "name": "John Doe",
        "given_name": "John",
        "family_name": "Doe",
        "email": "john.doe@example.com",
        "email_verified": true
    }"#;

    let user: UserInfo = serde_json::from_str(json).unwrap();
    assert_eq!(user.sub, "user-id-123");
    assert_eq!(user.name, Some("John Doe".to_string()));
    assert_eq!(user.email, Some("john.doe@example.com".to_string()));
    assert_eq!(user.email_verified, Some(true));
}

#[test]
fn test_user_info_minimal() {
    let json = r#"{
        "sub": "user-id-456"
    }"#;

    let user: UserInfo = serde_json::from_str(json).unwrap();
    assert_eq!(user.sub, "user-id-456");
    assert!(user.name.is_none());
    assert!(user.email.is_none());
}

#[test]
fn test_stored_token_expiry_edge_cases() {
    let now = chrono::Utc::now().timestamp();

    // Token expiring exactly at the threshold (60 seconds) should be invalid
    let threshold_token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: now + 60,
        scopes: vec![],
    };
    assert!(!threshold_token.is_valid());

    // Token expiring at 61 seconds should be valid
    let valid_token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: None,
        expires_at: now + 61,
        scopes: vec![],
    };
    assert!(valid_token.is_valid());
}

#[test]
fn test_stored_token_with_scopes() {
    let now = chrono::Utc::now().timestamp();
    let token = StoredToken {
        access_token: "test".to_string(),
        refresh_token: Some("refresh".to_string()),
        expires_at: now + 3600,
        scopes: vec![
            "data:read".to_string(),
            "data:write".to_string(),
            "bucket:create".to_string(),
        ],
    };

    assert!(token.is_valid());
    assert_eq!(token.scopes.len(), 3);
    assert!(token.scopes.contains(&"data:read".to_string()));
}

/// Integration tests for AuthClient using raps-mock
mod integration_tests {
    use super::*;
    use crate::config::Config;

    fn create_mock_auth_client(mock_url: &str) -> AuthClient {
        let config = Config {
            client_id: "test-client-id".to_string(),
            client_secret: "test-client-secret".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        AuthClient::new(config)
    }

    #[tokio::test]
    async fn test_get_2leg_token_success() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        // Pre-populate the 2-legged token cache since raps-mock's token
        // endpoint expects JSON but OAuth2 uses form-urlencoded
        client
            .set_2leg_token_for_testing("mock-2leg-token".to_string(), 3600)
            .await;

        let result = client.get_token().await;
        assert!(result.is_ok(), "get_token failed: {:?}", result.err());
        assert_eq!(result.unwrap(), "mock-2leg-token");
    }

    #[tokio::test]
    async fn test_get_3leg_token_not_logged_in() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        let result = client.get_3leg_token().await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not logged in"));
    }

    #[tokio::test]
    async fn test_is_logged_in_false_initially() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        let result = client.is_logged_in().await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_test_auth_success() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        // Pre-populate cache (test_auth calls get_token internally)
        client
            .set_2leg_token_for_testing("mock-test-token".to_string(), 3600)
            .await;

        let result = client.test_auth().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_config_accessor() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        let config = client.config();
        assert_eq!(config.client_id, "test-client-id");
        assert_eq!(config.base_url, server.url);
    }

    #[tokio::test]
    async fn test_get_token_expiry_none_when_not_logged_in() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        let expiry = client.get_token_expiry().await;
        assert!(expiry.is_none());
    }

    #[tokio::test]
    async fn test_logout_clears_token() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        // Logout should succeed even if not logged in
        let result = client.logout().await;
        // May fail because no token to delete, but shouldn't panic
        let _ = result;

        // Should not be logged in after logout
        assert!(!client.is_logged_in().await);
    }

    #[tokio::test]
    async fn test_get_token_with_mock_server() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_mock_auth_client(&server.url);

        // Clear any existing cache
        client.clear_cache().await;

        // Pre-populate cache and verify the caching mechanism works correctly.
        // Note: raps-mock's token endpoint expects JSON Content-Type but OAuth2
        // uses form-urlencoded, so we test the caching layer instead.
        client
            .set_2leg_token_for_testing("mock-cached-token".to_string(), 3600)
            .await;

        let result = client.get_token().await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock-cached-token");
    }

    /// Helper to create an AuthClient with empty credentials (simulates from_env_lenient with no config)
    fn create_empty_creds_client(mock_url: &str) -> AuthClient {
        let config = Config {
            client_id: "".to_string(),
            client_secret: "".to_string(),
            base_url: mock_url.to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        AuthClient::new(config)
    }

    #[tokio::test]
    async fn test_get_token_fails_with_empty_credentials() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_empty_creds_client(&server.url);

        let result = client.get_token().await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("APS_CLIENT_ID"),
            "Error should mention APS_CLIENT_ID, got: {msg}"
        );
    }

    #[tokio::test]
    async fn test_refresh_resets_flag_on_credential_error() {
        let server = raps_mock::TestServer::start_default().await.unwrap();
        let client = create_empty_creds_client(&server.url);

        // Simulate a logged-in state with an expired token that has a refresh token
        let expired_token = StoredToken {
            access_token: "expired".to_string(),
            refresh_token: Some("refresh-token".to_string()),
            expires_at: chrono::Utc::now().timestamp() - 100,
            scopes: vec!["data:read".to_string()],
        };
        client.set_3leg_token_for_testing(expired_token).await;

        // get_3leg_token should fail because credentials are empty
        let result = client.get_3leg_token().await;
        assert!(result.is_err());

        // The refreshing flag must have been reset so the next caller doesn't spin-wait
        let cache = client.cached_3leg_token.lock().await;
        assert!(
            !cache.refreshing,
            "refreshing flag should be reset after credential error"
        );
    }
}
