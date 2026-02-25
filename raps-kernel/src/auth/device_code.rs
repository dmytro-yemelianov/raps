// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Device code OAuth flow for headless environments

use anyhow::{Context, Result};
use colored::Colorize;
use std::time::{Duration, Instant};

use super::types::{DeviceCodeResponse, TokenResponse};
use super::AuthClient;
use crate::types::StoredToken;

impl AuthClient {
    /// Login with 3-legged OAuth using device code flow (headless-friendly)
    pub async fn login_device(&self, scopes: &[&str]) -> Result<StoredToken> {
        let url = format!("{}/authentication/v2/device", self.config.base_url);

        // Request device code
        let scope_str = scopes.join(" ");
        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("scope", scope_str.as_str()),
        ];
        let _auth_start = std::time::Instant::now();
        let response = self
            .http_client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to request device code")?;
        crate::profiler::record_http_request(_auth_start.elapsed());

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Device code request failed ({status}): {error_text}");
        }

        let device_response: DeviceCodeResponse = response
            .json()
            .await
            .context("Failed to parse device code response")?;

        // Display instructions to user
        println!("\n{}", "Device Code Authentication".bold().cyan());
        println!("{}", "-".repeat(50));
        println!(
            "  {} {}",
            "User Code:".bold(),
            device_response.user_code.bold().yellow()
        );
        println!(
            "  {} {}",
            "Verification URL:".bold(),
            device_response.verification_uri.cyan()
        );
        if let Some(ref complete_url) = device_response.verification_uri_complete {
            println!("  {} {}", "Complete URL:".bold(), complete_url.cyan());
        }
        println!(
            "\n{}",
            "Please visit the URL above and enter the user code to authorize.".dimmed()
        );
        println!(
            "{}",
            format!(
                "Waiting for authorization (expires in {} seconds)...",
                device_response.expires_in
            )
            .dimmed()
        );
        println!("{}", "-".repeat(50));

        // Poll for token
        let poll_interval = Duration::from_secs(device_response.interval.unwrap_or(5));
        let expires_at = Instant::now() + Duration::from_secs(device_response.expires_in);
        let mut last_poll = Instant::now();

        loop {
            // Check if expired
            if Instant::now() >= expires_at {
                anyhow::bail!("Device code expired. Please try again.");
            }

            // Wait for polling interval
            let elapsed = last_poll.elapsed();
            if elapsed < poll_interval {
                tokio::time::sleep(poll_interval - elapsed).await;
            }
            last_poll = Instant::now();

            // Poll for token
            let token_url = self.config.auth_url();
            let poll_params = [
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", &device_response.device_code),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
            ];

            let _auth_start = std::time::Instant::now();
            let poll_response = self
                .http_client
                .post(&token_url)
                .form(&poll_params)
                .send()
                .await
                .context("Failed to poll for token")?;
            crate::profiler::record_http_request(_auth_start.elapsed());

            if poll_response.status().is_success() {
                let token: TokenResponse = poll_response
                    .json()
                    .await
                    .context("Failed to parse token response")?;

                println!("\n{} Authorization successful!", "OK".green().bold());

                // Store the token
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

                return Ok(stored);
            }

            // Check error response
            let error_text = poll_response.text().await.unwrap_or_default();
            if error_text.contains("authorization_pending") {
                // Still waiting, continue polling
                print!(".");
                use std::io::Write;
                std::io::stdout().flush().ok();
                continue;
            }
            if error_text.contains("slow_down") {
                // Slow down polling
                tokio::time::sleep(poll_interval * 2).await;
                continue;
            }
            if error_text.contains("expired_token") {
                anyhow::bail!("Device code expired. Please try again.");
            }

            anyhow::bail!("Token polling failed: {error_text}");
        }
    }
}
