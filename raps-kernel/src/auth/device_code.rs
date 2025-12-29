// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Device Code OAuth 2.0 flow for APS
//!
//! This flow is used for devices without browser capability or for
//! improved user experience. The user authenticates on a separate device
//! by visiting a URL and entering a code.
//!
//! Flow:
//! 1. Request device code from APS
//! 2. Display verification URL and user code to user
//! 3. Poll for token while user authenticates
//! 4. Receive tokens when user completes authentication

use crate::config::Config;
use crate::error::{ExitCode, RapsError, Result};
use crate::http::HttpClient;
use super::types::{TokenResponse, DeviceCodeResponse, Scopes};
use std::time::Duration;
use tokio::time::sleep;

/// Device code polling errors
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceCodeError {
    /// User has not yet completed authorization
    AuthorizationPending,
    /// Polling too frequently
    SlowDown,
    /// Authorization was denied by user
    AccessDenied,
    /// Device code has expired
    ExpiredToken,
}

/// Device Code OAuth 2.0 authentication
///
/// Implements the device authorization grant flow.
pub struct DeviceCodeAuth<'a> {
    config: &'a Config,
    http: &'a HttpClient,
}

impl<'a> DeviceCodeAuth<'a> {
    /// Create a new device code auth handler
    pub fn new(config: &'a Config, http: &'a HttpClient) -> Self {
        Self { config, http }
    }

    /// Request a device code for user authentication
    pub async fn request_device_code(&self) -> Result<DeviceCodeResponse> {
        self.request_device_code_with_scopes(&Scopes::three_legged_default()).await
    }

    /// Request a device code with custom scopes
    pub async fn request_device_code_with_scopes(&self, scopes: &[&str]) -> Result<DeviceCodeResponse> {
        let url = "https://developer.api.autodesk.com/authentication/v2/device/code";
        let scope_string = Scopes::join(scopes);

        let params = [
            ("client_id", self.config.client_id.as_str()),
            ("scope", &scope_string),
        ];

        let response = self
            .http
            .inner()
            .post(url)
            .form(&params)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to request device code".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Device code request failed ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let device_code: DeviceCodeResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse device code response: {}", e),
            })?;

        Ok(device_code)
    }

    /// Poll for token after user authorizes
    ///
    /// Returns `Ok(TokenResponse)` when user completes authorization,
    /// or an error if authorization fails or times out.
    pub async fn poll_for_token(&self, device_code: &DeviceCodeResponse) -> Result<TokenResponse> {
        let interval = Duration::from_secs(device_code.interval.unwrap_or(5));
        let max_attempts = (device_code.expires_in / interval.as_secs()) as usize;

        for _ in 0..max_attempts {
            sleep(interval).await;

            match self.try_get_token(&device_code.device_code).await {
                Ok(token) => return Ok(token),
                Err(e) => {
                    if let Some(poll_error) = Self::parse_poll_error(&e) {
                        match poll_error {
                            DeviceCodeError::AuthorizationPending => continue,
                            DeviceCodeError::SlowDown => {
                                // Wait extra time
                                sleep(Duration::from_secs(5)).await;
                                continue;
                            }
                            DeviceCodeError::AccessDenied => {
                                return Err(RapsError::Auth {
                                    message: "User denied authorization".to_string(),
                                    code: ExitCode::AuthFailure,
                                    source: None,
                                });
                            }
                            DeviceCodeError::ExpiredToken => {
                                return Err(RapsError::Auth {
                                    message: "Device code expired. Please try again.".to_string(),
                                    code: ExitCode::AuthFailure,
                                    source: None,
                                });
                            }
                        }
                    }
                    // Unknown error, propagate
                    return Err(e);
                }
            }
        }

        Err(RapsError::Auth {
            message: "Device code authentication timed out".to_string(),
            code: ExitCode::AuthFailure,
            source: None,
        })
    }

    /// Attempt to get token (single poll attempt)
    async fn try_get_token(&self, device_code: &str) -> Result<TokenResponse> {
        let url = self.config.auth_url();

        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
            ("client_id", &self.config.client_id),
        ];

        let response = self
            .http
            .inner()
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to poll for token".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: error_text,
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let token: TokenResponse = response
            .json()
            .await
            .map_err(|e| RapsError::Internal {
                message: format!("Failed to parse token response: {}", e),
            })?;

        Ok(token)
    }

    /// Parse a poll error to determine if it's a known device code error
    fn parse_poll_error(error: &RapsError) -> Option<DeviceCodeError> {
        if let RapsError::Api { message, .. } = error {
            if message.contains("authorization_pending") {
                return Some(DeviceCodeError::AuthorizationPending);
            }
            if message.contains("slow_down") {
                return Some(DeviceCodeError::SlowDown);
            }
            if message.contains("access_denied") {
                return Some(DeviceCodeError::AccessDenied);
            }
            if message.contains("expired_token") {
                return Some(DeviceCodeError::ExpiredToken);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_poll_error_authorization_pending() {
        let error = RapsError::Api {
            message: r#"{"error": "authorization_pending"}"#.to_string(),
            status: Some(400),
            source: None,
        };
        assert_eq!(
            DeviceCodeAuth::parse_poll_error(&error),
            Some(DeviceCodeError::AuthorizationPending)
        );
    }

    #[test]
    fn test_parse_poll_error_slow_down() {
        let error = RapsError::Api {
            message: r#"{"error": "slow_down"}"#.to_string(),
            status: Some(400),
            source: None,
        };
        assert_eq!(
            DeviceCodeAuth::parse_poll_error(&error),
            Some(DeviceCodeError::SlowDown)
        );
    }

    #[test]
    fn test_parse_poll_error_access_denied() {
        let error = RapsError::Api {
            message: r#"{"error": "access_denied"}"#.to_string(),
            status: Some(400),
            source: None,
        };
        assert_eq!(
            DeviceCodeAuth::parse_poll_error(&error),
            Some(DeviceCodeError::AccessDenied)
        );
    }

    #[test]
    fn test_parse_poll_error_expired() {
        let error = RapsError::Api {
            message: r#"{"error": "expired_token"}"#.to_string(),
            status: Some(400),
            source: None,
        };
        assert_eq!(
            DeviceCodeAuth::parse_poll_error(&error),
            Some(DeviceCodeError::ExpiredToken)
        );
    }

    #[test]
    fn test_parse_poll_error_unknown() {
        let error = RapsError::Api {
            message: r#"{"error": "unknown_error"}"#.to_string(),
            status: Some(400),
            source: None,
        };
        assert_eq!(DeviceCodeAuth::parse_poll_error(&error), None);
    }

    #[test]
    fn test_parse_poll_error_non_api() {
        let error = RapsError::Internal {
            message: "Some internal error".to_string(),
        };
        assert_eq!(DeviceCodeAuth::parse_poll_error(&error), None);
    }
}
