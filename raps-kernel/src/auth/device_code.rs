// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Device code OAuth flow for headless environments
//!
//! Uses a proxy at `RAPS_DEVICE_PROXY_URL` (default: https://rapscli.xyz) to
//! provide a GitHub-style "go to URL, enter short code" experience. PKCE
//! security is preserved end-to-end — the proxy never sees the code_verifier
//! or client_secret.

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use colored::Colorize;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::AuthClient;
use crate::types::StoredToken;

/// Default proxy URL for device code auth
const DEFAULT_PROXY_URL: &str = "https://rapscli.xyz";

/// Polling interval in seconds
const POLL_INTERVAL_SECS: u64 = 5;

/// Maximum polling duration in seconds
const MAX_POLL_DURATION_SECS: u64 = 300;

/// Response from `POST /device/authorize`
#[derive(Debug, Deserialize)]
struct DeviceAuthResponse {
    session_id: String,
    user_code: String,
    #[allow(dead_code)]
    expires_in: u64,
}

/// Response from `GET /device/token`
#[derive(Debug, Deserialize)]
struct DevicePollResponse {
    state: String,
    auth_code: Option<String>,
}

/// Generate a cryptographically random PKCE code verifier.
///
/// The verifier is 128 characters from the unreserved character set
/// defined in RFC 7636 §4.1: [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"
fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::thread_rng();
    (0..128)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Derive the S256 code challenge from a code verifier (RFC 7636 §4.2).
fn derive_code_challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hash)
}

/// Get the device proxy base URL from environment or default.
fn proxy_base_url() -> String {
    std::env::var("RAPS_DEVICE_PROXY_URL")
        .unwrap_or_else(|_| DEFAULT_PROXY_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

impl AuthClient {
    /// Login with 3-legged OAuth using device code flow (headless-friendly).
    ///
    /// 1. Generates a PKCE code verifier / challenge pair locally.
    /// 2. Initiates a device session via the proxy.
    /// 3. Displays a short user code for the user to enter at the proxy URL.
    /// 4. Polls the proxy until the user authorizes.
    /// 5. Exchanges the authorization code via PKCE against APS.
    pub async fn login_device(&self, scopes: &[&str]) -> Result<StoredToken> {
        self.config.require_credentials()?;

        // Note: no interactive check here — device code flow is designed for
        // headless/non-interactive environments. It only prints a code and polls;
        // no stdin input is required.

        let proxy_base = proxy_base_url();

        // --- PKCE ---
        let code_verifier = generate_code_verifier();
        let code_challenge = derive_code_challenge(&code_verifier);

        // --- Initiate device session ---
        let scope_str = scopes.join(" ");
        let session = self
            .initiate_device_session(&proxy_base, &scope_str, &code_challenge)
            .await?;

        // --- Display instructions ---
        println!("\n{}", "Device Authorization".bold().cyan());
        println!("{}", "-".repeat(50));
        println!(
            "  Go to: {}",
            format!("{}/device", proxy_base).cyan().bold()
        );
        println!(
            "  Enter code: {}",
            session.user_code.yellow().bold()
        );
        println!("{}", "-".repeat(50));
        println!(
            "{}",
            "Waiting for authorization...".dimmed()
        );

        // --- Poll for authorization ---
        let auth_code = self
            .poll_device_token(&proxy_base, &session.session_id)
            .await?;

        println!("Authorization code received, exchanging for token...");

        // --- Exchange code for tokens via APS ---
        // The redirect_uri must match what the proxy used for the APS authorize redirect
        let callback_uri = format!("{}/device/callback", proxy_base);
        let token = self
            .exchange_code_pkce(&auth_code, &callback_uri, &code_verifier)
            .await?;

        println!("\n{} Authorization successful!", "OK".green().bold());

        // --- Consume session (fire-and-forget) ---
        let proxy_base_clone = proxy_base.clone();
        let session_id_clone = session.session_id.clone();
        let http_clone = self.http_client.clone();
        tokio::spawn(async move {
            let _ = consume_device_session_static(
                &http_clone,
                &proxy_base_clone,
                &session_id_clone,
            )
            .await;
        });

        // --- Store token ---
        let stored = StoredToken {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            expires_at: chrono::Utc::now().timestamp() + token.expires_in as i64,
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
        };

        self.save_token(&stored)?;

        // Update cache
        {
            let mut cache = self.cached_3leg_token.lock().await;
            cache.token = Some(stored.clone());
        }

        Ok(stored)
    }

    /// Initiate a device auth session with the proxy.
    async fn initiate_device_session(
        &self,
        proxy_base: &str,
        scopes: &str,
        code_challenge: &str,
    ) -> Result<DeviceAuthResponse> {
        let url = format!("{}/device/authorize", proxy_base);

        let body = serde_json::json!({
            "client_id": self.config.client_id,
            "scopes": scopes,
            "code_challenge": code_challenge,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .context("Failed to connect to device auth proxy")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Device auth proxy error ({status}): {error_text}");
        }

        response
            .json::<DeviceAuthResponse>()
            .await
            .context("Failed to parse device auth response")
    }

    /// Poll the proxy for the authorization code.
    async fn poll_device_token(
        &self,
        proxy_base: &str,
        session_id: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/device/token?session_id={}",
            proxy_base,
            urlencoding::encode(session_id)
        );

        let start = std::time::Instant::now();
        let max_duration = std::time::Duration::from_secs(MAX_POLL_DURATION_SECS);
        let interval = std::time::Duration::from_secs(POLL_INTERVAL_SECS);

        loop {
            if start.elapsed() > max_duration {
                anyhow::bail!(
                    "Device authorization timed out after {}s. Please try again.",
                    MAX_POLL_DURATION_SECS
                );
            }

            tokio::time::sleep(interval).await;

            let response = match self.http_client.get(&url).send().await {
                Ok(resp) => resp,
                Err(_) => continue, // Transient network error, retry
            };

            let poll: DevicePollResponse = match response.json().await {
                Ok(p) => p,
                Err(_) => continue,
            };

            match poll.state.as_str() {
                "authorized" => {
                    let auth_code = poll.auth_code.ok_or_else(|| {
                        anyhow::anyhow!("Proxy returned authorized state without auth_code")
                    })?;
                    return Ok(auth_code);
                }
                "expired" => {
                    anyhow::bail!("Device code expired. Please try again.");
                }
                "pending" => {
                    // Keep polling
                }
                other => {
                    anyhow::bail!("Unexpected device session state: {other}");
                }
            }
        }
    }

    /// Exchange an authorization code for tokens using PKCE (no client_secret required).
    ///
    /// Uses HTTP Basic auth with client_id/client_secret for compatibility with APS,
    /// while also sending the PKCE code_verifier.
    async fn exchange_code_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<super::types::TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", code_verifier),
        ];

        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .context("Failed to exchange authorization code")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            let redacted = crate::logging::redact_secrets(&error_text);
            anyhow::bail!("Token exchange failed ({status}): {redacted}");
        }

        let token: super::types::TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token)
    }
}

/// Static helper for fire-and-forget session consumption (used from spawned task).
async fn consume_device_session_static(
    http_client: &reqwest::Client,
    proxy_base: &str,
    session_id: &str,
) -> Result<()> {
    let url = format!("{}/device/consume", proxy_base);
    let body = serde_json::json!({ "session_id": session_id });

    http_client
        .post(&url)
        .json(&body)
        .send()
        .await
        .context("Failed to consume device session")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // PKCE tests (preserved from original)
    // ========================================================================

    #[test]
    fn test_code_verifier_length_and_charset() {
        let verifier = generate_code_verifier();
        assert_eq!(verifier.len(), 128);
        for ch in verifier.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '-' || ch == '.' || ch == '_' || ch == '~',
                "Invalid character in code verifier: {ch}"
            );
        }
    }

    #[test]
    fn test_code_verifier_uniqueness() {
        let v1 = generate_code_verifier();
        let v2 = generate_code_verifier();
        assert_ne!(v1, v2, "Two verifiers should not be identical");
    }

    #[test]
    fn test_code_challenge_is_valid_base64url() {
        let verifier = generate_code_verifier();
        let challenge = derive_code_challenge(&verifier);
        // SHA-256 produces 32 bytes → 43 base64url chars (no padding)
        assert_eq!(challenge.len(), 43);
        for ch in challenge.chars() {
            assert!(
                ch.is_ascii_alphanumeric() || ch == '-' || ch == '_',
                "Invalid character in code challenge: {ch}"
            );
        }
    }

    #[test]
    fn test_code_challenge_deterministic() {
        let verifier = "test-verifier-value";
        let c1 = derive_code_challenge(verifier);
        let c2 = derive_code_challenge(verifier);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_code_challenge_known_vector() {
        // RFC 7636 Appendix B test vector
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = derive_code_challenge(verifier);
        assert_eq!(challenge, "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    // ========================================================================
    // Deserialization tests for new response structs
    // ========================================================================

    #[test]
    fn test_deserialize_device_auth_response() {
        let json = r#"{"session_id":"abc-123","user_code":"ABCD-1234","expires_in":300}"#;
        let resp: DeviceAuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_id, "abc-123");
        assert_eq!(resp.user_code, "ABCD-1234");
        assert_eq!(resp.expires_in, 300);
    }

    #[test]
    fn test_deserialize_device_poll_pending() {
        let json = r#"{"state":"pending"}"#;
        let resp: DevicePollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.state, "pending");
        assert!(resp.auth_code.is_none());
    }

    #[test]
    fn test_deserialize_device_poll_authorized() {
        let json = r#"{"state":"authorized","auth_code":"some-auth-code-from-aps"}"#;
        let resp: DevicePollResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.state, "authorized");
        assert_eq!(resp.auth_code.as_deref(), Some("some-auth-code-from-aps"));
    }

    #[test]
    fn test_proxy_base_url_default() {
        // When env var is not set, should return default
        // (This test may be affected by env, so we just verify the function doesn't panic)
        let url = proxy_base_url();
        assert!(!url.is_empty());
        assert!(!url.ends_with('/'));
    }
}
