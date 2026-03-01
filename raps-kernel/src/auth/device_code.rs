// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Manual PKCE OAuth flow for headless environments
//!
//! Instead of the device code grant (which APS doesn't support), this module
//! implements a manual authorization code flow with PKCE (S256). The user is
//! shown an authorize URL, opens it on any device, and pastes the resulting
//! callback URL (or bare authorization code) back into the terminal.

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use colored::Colorize;
use sha2::{Digest, Sha256};

use super::AuthClient;
use super::types::TokenResponse;
use crate::types::StoredToken;

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

impl AuthClient {
    /// Login with 3-legged OAuth using manual PKCE flow (headless-friendly).
    ///
    /// 1. Generates a PKCE code verifier / challenge pair.
    /// 2. Prints an authorization URL for the user to open in any browser.
    /// 3. Prompts the user to paste back the callback URL (or bare code).
    /// 4. Exchanges the authorization code + verifier for tokens.
    pub async fn login_device(&self, scopes: &[&str]) -> Result<StoredToken> {
        self.config.require_credentials()?;

        if crate::interactive::is_non_interactive() {
            anyhow::bail!(
                "3-legged OAuth (device/PKCE) requires interactive mode.\n\
                 Use 2-legged auth (raps auth login) for CI/CD, or pass \
                 credentials via APS_CLIENT_ID and APS_CLIENT_SECRET environment variables."
            );
        }

        // --- PKCE ---
        let code_verifier = generate_code_verifier();
        let code_challenge = derive_code_challenge(&code_verifier);

        // --- CSRF state ---
        let state = uuid::Uuid::new_v4().to_string();

        // --- Build authorize URL ---
        let scope_str = scopes.join(" ");
        let redirect_uri = &self.config.callback_url;
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            self.config.authorize_url(),
            urlencoding::encode(&self.config.client_id),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(&scope_str),
            urlencoding::encode(&state),
            urlencoding::encode(&code_challenge),
        );

        // --- Display instructions ---
        println!("\n{}", "Manual PKCE Authentication".bold().cyan());
        println!("{}", "-".repeat(50));
        println!(
            "{}",
            "Open the following URL in any browser to authorize:".dimmed()
        );
        println!("\n  {}\n", auth_url.cyan());
        println!(
            "{}",
            "After authorizing, you will be redirected to your callback URL.".dimmed()
        );
        println!(
            "{}",
            "Paste the full callback URL (or just the authorization code) below.".dimmed()
        );
        println!("{}", "-".repeat(50));

        // --- Prompt user for the callback URL / code ---
        let input: String = crate::prompts::spawn_prompt(|| {
            Ok(dialoguer::Input::new()
                .with_prompt("Callback URL or authorization code")
                .interact_text()?)
        })
        .await
        .context("Failed to read user input")?;

        let input = input.trim().to_string();
        if input.is_empty() {
            anyhow::bail!("No authorization code provided. Please try again.");
        }

        // Parse the authorization code and validate state
        let auth_code = if input.contains("code=") || input.starts_with("http") {
            // User pasted a full URL — extract query parameters
            let parsed_url = url::Url::parse(&input)
                .context("Failed to parse the pasted URL. Please try again with a valid URL.")?;
            let params: std::collections::HashMap<_, _> = parsed_url.query_pairs().collect();

            // Check for OAuth error in the callback
            if let Some(error) = params.get("error") {
                let desc = params
                    .get("error_description")
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                anyhow::bail!("Authorization error: {error} - {desc}");
            }

            // Validate CSRF state
            let returned_state = params
                .get("state")
                .ok_or_else(|| anyhow::anyhow!("Missing state parameter in callback URL"))?;
            if returned_state.as_ref() != state.as_str() {
                anyhow::bail!("State mismatch — possible CSRF attack. Please try again.");
            }

            params
                .get("code")
                .ok_or_else(|| anyhow::anyhow!("No authorization code found in callback URL"))?
                .to_string()
        } else {
            // User pasted a bare authorization code
            input
        };

        println!("Authorization code received, exchanging for token...");

        // --- Exchange code for tokens ---
        let token = self
            .exchange_code_pkce(&auth_code, redirect_uri, &code_verifier)
            .await?;

        println!("\n{} Authorization successful!", "OK".green().bold());

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

    /// Exchange an authorization code for tokens using PKCE (no client_secret required).
    ///
    /// Uses HTTP Basic auth with client_id/client_secret for compatibility with APS,
    /// while also sending the PKCE code_verifier.
    async fn exchange_code_pkce(
        &self,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
    ) -> Result<TokenResponse> {
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

        let token: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;

        Ok(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
