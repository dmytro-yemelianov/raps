// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! HTTP client utilities
//!
//! Provides retry logic, timeouts, and HTTP client configuration.

use anyhow::{Context, Result};
use reqwest::Client;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use url::Url;

/// Resolve a configuration parameter with precedence: CLI flag > env var > default.
fn resolve_param<T: FromStr + Copy>(flag: Option<T>, env_var: &str, default: T) -> T {
    if let Some(v) = flag {
        return v;
    }
    if let Ok(val) = std::env::var(env_var)
        && let Ok(parsed) = val.parse::<T>()
    {
        return parsed;
    }
    default
}

/// Allowed domains for custom API calls (APS domains only)
pub const ALLOWED_DOMAINS: &[&str] = &[
    "developer.api.autodesk.com",
    "api.userprofile.autodesk.com",
    "acc.autodesk.com",
    "developer.autodesk.com",
    "b360dm.autodesk.com",
    "cdn.derivative.autodesk.io",
];

/// Check if a URL is allowed (belongs to an APS domain)
///
/// Returns true if the URL's host matches one of the allowed domains.
/// Used for custom API calls to prevent credential leakage to external URLs.
pub fn is_allowed_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(parsed) => {
            if let Some(host) = parsed.host_str() {
                // Check if host matches any allowed domain
                ALLOWED_DOMAINS.iter().any(|domain| {
                    host == *domain
                        || (host.len() > domain.len()
                            && host.ends_with(domain)
                            && host.as_bytes()[host.len() - domain.len() - 1] == b'.')
                })
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    /// Maximum wait time between retries (seconds)
    pub max_wait: u64,
    /// Base delay for exponential backoff (seconds)
    pub base_delay: u64,
    /// Request timeout (seconds)
    pub timeout: u64,
    /// Connect timeout (seconds)
    pub connect_timeout: u64,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            max_wait: 60,
            base_delay: 1,
            timeout: 120,
            connect_timeout: 30,
        }
    }
}

impl HttpClientConfig {
    /// Create HTTP client with configured timeouts, H2 multiplexing, and connection pooling.
    pub fn create_client(&self) -> Result<Client> {
        Client::builder()
            .http2_adaptive_window(true)
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(30))
            .timeout(Duration::from_secs(self.timeout))
            .connect_timeout(Duration::from_secs(self.connect_timeout))
            .build()
            .context("Failed to create HTTP client")
    }

    /// Create HTTP client config from CLI flags and environment variables.
    ///
    /// Precedence: CLI flag > environment variable > default.
    /// Use [`from_cli_and_env_full`] to pass all retry parameters.
    pub fn from_cli_and_env(timeout_flag: Option<u64>) -> Self {
        Self::from_cli_and_env_full(timeout_flag, None, None, None)
    }

    /// Create HTTP client config from CLI flags and environment variables
    /// with full control over retry parameters.
    ///
    /// Precedence: CLI flag > environment variable > default
    pub fn from_cli_and_env_full(
        timeout_flag: Option<u64>,
        max_retries_flag: Option<u32>,
        base_delay_flag: Option<u64>,
        max_wait_flag: Option<u64>,
    ) -> Self {
        let timeout = resolve_param(timeout_flag, "RAPS_TIMEOUT", 120);
        let max_retries = resolve_param(max_retries_flag, "RAPS_MAX_RETRIES", 3);
        let base_delay = resolve_param(base_delay_flag, "RAPS_BASE_DELAY", 1);
        let max_wait = resolve_param(max_wait_flag, "RAPS_MAX_WAIT", 60);

        Self {
            timeout,
            max_retries,
            base_delay,
            max_wait,
            ..Self::default()
        }
    }
}

/// Check if an HTTP status code is retryable (rate limit or server error)
pub fn is_retryable_status(status: u16) -> bool {
    matches!(status, 408 | 429 | 500 | 502 | 503 | 504)
}

/// Calculate retry delay from response headers or exponential backoff
///
/// Checks the `Retry-After` header first (seconds value), then falls back
/// to exponential backoff with jitter.
pub fn retry_delay_from_response(
    response: &reqwest::Response,
    attempt: u32,
    config: &HttpClientConfig,
) -> Duration {
    if let Some(retry_after) = response.headers().get("retry-after")
        && let Ok(secs) = retry_after.to_str().unwrap_or("").parse::<u64>()
    {
        return Duration::from_secs(secs.min(config.max_wait));
    }
    calculate_delay(attempt + 1, config.base_delay, config.max_wait)
}

/// Send HTTP request with automatic retry on 429/5xx and network errors
///
/// Inspects the HTTP response status code and retries on retryable status codes
/// (408, 429, 5xx). Also respects the `Retry-After` header for rate limiting.
///
/// The closure should return a `reqwest::RequestBuilder` (not a future),
/// which will be rebuilt on each retry attempt.
pub async fn send_with_retry<F>(
    config: &HttpClientConfig,
    build_request: F,
) -> Result<reqwest::Response>
where
    F: Fn() -> reqwest::RequestBuilder,
{
    // --- Pre-flight: circuit breaker check ---
    // We need the URL for endpoint classification.  Build once to peek, but
    // the closure will be called again for the actual send.
    let probe_req = build_request().build();
    let endpoint_group = match &probe_req {
        Ok(r) => crate::circuit_breaker::endpoint_group(r.url().as_str()).to_string(),
        Err(_) => "other".to_string(),
    };

    // Check circuit breaker — fail fast if endpoint is unhealthy
    if let Err(cb_err) = crate::circuit_breaker::registry().check(&endpoint_group) {
        tracing::warn!(endpoint = %endpoint_group, "circuit breaker open — rejecting request");
        anyhow::bail!(cb_err);
    }

    // --- Pre-flight: rate budget check ---
    match crate::rate_budget::registry().check(&endpoint_group) {
        crate::rate_budget::RateStatus::Exhausted { retry_after } => {
            tracing::warn!(
                endpoint = %endpoint_group,
                retry_after_ms = retry_after.as_millis() as u64,
                "rate limit exhausted — waiting before request"
            );
            sleep(retry_after).await;
        }
        crate::rate_budget::RateStatus::NearLimit { remaining, limit } => {
            tracing::debug!(
                endpoint = %endpoint_group,
                remaining,
                limit,
                "rate limit near exhaustion"
            );
        }
        _ => {}
    }

    // --- Request loop with retry ---
    let mut attempt = 0;
    let mut total_network_time = std::time::Duration::ZERO;
    // Load per-endpoint stats once; we update and save after each request.
    let mut ep_stats = crate::endpoint_stats::EndpointStats::load();
    // Peek at the URL for adaptive backoff lookups (best-effort).
    let request_url = probe_req
        .as_ref()
        .map(|r| r.url().to_string())
        .unwrap_or_default();
    loop {
        let start = std::time::Instant::now();
        match build_request().send().await {
            Ok(response) => {
                let elapsed = start.elapsed();
                total_network_time += elapsed;
                let status = response.status().as_u16();
                let elapsed_ms = elapsed.as_millis() as u64;
                tracing::debug!(
                    http.status = status,
                    url = %response.url(),
                    elapsed_ms,
                    "HTTP response"
                );

                // Record per-endpoint stats
                let failed = status >= 400;
                ep_stats.record_request(response.url().as_str(), elapsed_ms, failed);
                ep_stats.save();

                // Record rate limit headers into the global budget tracker
                crate::rate_budget::registry()
                    .record_from_headers(&endpoint_group, response.headers());

                // Proactive throttle: if remaining quota is below 10% of
                // limit, sleep until the window resets (capped at 30 s) so
                // we do not slam the next request straight into a 429.
                {
                    let rl =
                        crate::rate_limit::RateLimitState::from_headers(response.headers());
                    if let Some(delay) = rl.throttle_delay() {
                        tracing::debug!(
                            remaining = ?rl.remaining,
                            limit = ?rl.limit,
                            delay_secs = delay.as_secs(),
                            "rate limit: {}/{} — throttling before next request",
                            rl.remaining.unwrap_or(0),
                            rl.limit.unwrap_or(0),
                        );
                        sleep(delay).await;
                    } else if let (Some(remaining), Some(limit)) = (rl.remaining, rl.limit) {
                        tracing::debug!(
                            remaining,
                            limit,
                            "rate limit: {}/{}",
                            remaining,
                            limit,
                        );
                    }
                }

                // Record success/failure for circuit breaker
                let failure_type = crate::retry_policy::FailureType::from_status(status);
                if let Some(ft) = failure_type {
                    if ft.triggers_circuit_breaker() {
                        crate::circuit_breaker::registry().record_failure(&endpoint_group);
                    }
                } else if status < 400 {
                    crate::circuit_breaker::registry().record_success(&endpoint_group);
                }

                if is_retryable_status(status) && attempt < config.max_retries {
                    let base_delay = retry_delay_from_response(&response, attempt, config);
                    let multiplier = ep_stats.backoff_multiplier(response.url().as_str());
                    let delay = base_delay.saturating_mul(multiplier);
                    attempt += 1;
                    crate::profiler::record_http_retry();
                    tracing::warn!(
                        http.status = status,
                        attempt,
                        max_retries = config.max_retries,
                        delay_secs = delay.as_secs_f64(),
                        backoff_multiplier = multiplier,
                        "Retryable HTTP status, retrying"
                    );
                    sleep(delay).await;
                    continue;
                }
                crate::profiler::record_http_request(total_network_time);
                crate::api_health::record_latency(total_network_time);
                return Ok(response);
            }
            Err(err) => {
                let elapsed = start.elapsed();
                total_network_time += elapsed;

                // Record per-endpoint stats for network-level failures
                ep_stats.record_request(&request_url, elapsed.as_millis() as u64, true);
                ep_stats.save();

                // Network error → record circuit breaker failure
                crate::circuit_breaker::registry().record_failure(&endpoint_group);

                let retriable = err.is_timeout() || err.is_connect() || err.is_request();
                if !retriable || attempt >= config.max_retries {
                    crate::profiler::record_http_request(total_network_time);
                    crate::api_health::record_failure();
                    tracing::error!(error = %err, attempt, "HTTP request failed");
                    return Err(err).context("HTTP request failed");
                }
                attempt += 1;
                crate::profiler::record_http_retry();
                let base_delay = calculate_delay(attempt, config.base_delay, config.max_wait);
                let multiplier = ep_stats.backoff_multiplier(&request_url);
                let delay = base_delay.saturating_mul(multiplier);
                tracing::warn!(
                    error = %err,
                    attempt,
                    max_retries = config.max_retries,
                    delay_secs = delay.as_secs_f64(),
                    backoff_multiplier = multiplier,
                    "Network error, retrying"
                );
                sleep(delay).await;
            }
        }
    }
}

/// Calculate delay with exponential backoff and jitter
pub fn calculate_delay(attempt: u32, base_delay: u64, max_wait: u64) -> Duration {
    use rand::Rng;

    // Exponential backoff: base_delay * 2^attempt (saturating to avoid overflow)
    let exponential_delay =
        base_delay.saturating_mul(1_u64.checked_shl(attempt).unwrap_or(u64::MAX));

    // Cap at max_wait
    let capped_delay = exponential_delay.min(max_wait);

    // Add jitter (random 0-25% of delay)
    let mut rng = rand::thread_rng();
    let jitter = if capped_delay > 0 {
        rng.gen_range(0..=(capped_delay / 4))
    } else {
        0
    };

    Duration::from_secs(capped_delay.saturating_add(jitter))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_config_default() {
        let config = HttpClientConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_wait, 60);
        assert_eq!(config.base_delay, 1);
        assert_eq!(config.timeout, 120);
        assert_eq!(config.connect_timeout, 30);
    }

    #[test]
    fn test_http_config_create_client() {
        let config = HttpClientConfig::default();
        let client = config.create_client();
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_config_from_cli_flag() {
        let config = HttpClientConfig::from_cli_and_env(Some(60));
        assert_eq!(config.timeout, 60);
        // Other values should be default
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_http_config_from_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "90");
        }
        let config = HttpClientConfig::from_cli_and_env(None);
        assert_eq!(config.timeout, 90);
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_http_config_cli_overrides_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "90");
        }
        let config = HttpClientConfig::from_cli_and_env(Some(45));
        assert_eq!(config.timeout, 45);
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_http_config_invalid_env() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_TIMEOUT", "not_a_number");
        }
        let config = HttpClientConfig::from_cli_and_env(None);
        assert_eq!(config.timeout, 120); // Falls back to default
        unsafe {
            std::env::remove_var("RAPS_TIMEOUT");
        }
    }

    #[test]
    fn test_http_config_full_cli_flags() {
        let config = HttpClientConfig::from_cli_and_env_full(Some(30), Some(5), Some(2), Some(120));
        assert_eq!(config.timeout, 30);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.base_delay, 2);
        assert_eq!(config.max_wait, 120);
    }

    #[test]
    fn test_http_config_full_env_vars() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_MAX_RETRIES", "7");
            std::env::set_var("RAPS_BASE_DELAY", "3");
            std::env::set_var("RAPS_MAX_WAIT", "90");
        }
        let config = HttpClientConfig::from_cli_and_env_full(None, None, None, None);
        assert_eq!(config.max_retries, 7);
        assert_eq!(config.base_delay, 3);
        assert_eq!(config.max_wait, 90);
        unsafe {
            std::env::remove_var("RAPS_MAX_RETRIES");
            std::env::remove_var("RAPS_BASE_DELAY");
            std::env::remove_var("RAPS_MAX_WAIT");
        }
    }

    #[test]
    fn test_http_config_full_cli_overrides_env() {
        unsafe {
            std::env::set_var("RAPS_MAX_RETRIES", "10");
        }
        let config = HttpClientConfig::from_cli_and_env_full(None, Some(2), None, None);
        assert_eq!(config.max_retries, 2); // CLI wins
        unsafe {
            std::env::remove_var("RAPS_MAX_RETRIES");
        }
    }

    #[test]
    fn test_calculate_delay_exponential() {
        // First retry: base_delay * 2^1 = 1 * 2 = 2 seconds
        let delay1 = calculate_delay(1, 1, 60);
        assert!(delay1.as_secs() >= 2);
        assert!(delay1.as_secs() <= 3); // 2 + up to 25% jitter

        // Second retry: base_delay * 2^2 = 1 * 4 = 4 seconds
        let delay2 = calculate_delay(2, 1, 60);
        assert!(delay2.as_secs() >= 4);
        assert!(delay2.as_secs() <= 5);
    }

    #[test]
    fn test_calculate_delay_max_wait() {
        // Very high attempt should be capped at max_wait
        let delay = calculate_delay(10, 1, 60);
        assert!(delay.as_secs() <= 75); // 60 + up to 25% jitter
    }

    #[test]
    fn test_calculate_delay_custom_base() {
        // With base_delay of 2: 2 * 2^1 = 4 seconds
        let delay = calculate_delay(1, 2, 60);
        assert!(delay.as_secs() >= 4);
        assert!(delay.as_secs() <= 5);
    }

    #[test]
    fn test_is_allowed_url_developer_api() {
        assert!(is_allowed_url(
            "https://developer.api.autodesk.com/oss/v2/buckets"
        ));
    }

    #[test]
    fn test_is_allowed_url_userprofile() {
        assert!(is_allowed_url(
            "https://api.userprofile.autodesk.com/userinfo"
        ));
    }

    #[test]
    fn test_is_allowed_url_acc() {
        assert!(is_allowed_url("https://acc.autodesk.com/api/projects"));
    }

    #[test]
    fn test_is_allowed_url_with_path_and_query() {
        assert!(is_allowed_url(
            "https://developer.api.autodesk.com/oss/v2/buckets?limit=10&region=US"
        ));
    }

    #[test]
    fn test_is_allowed_url_external_rejected() {
        assert!(!is_allowed_url("https://evil.com/steal-token"));
    }

    #[test]
    fn test_is_allowed_url_localhost_rejected() {
        assert!(!is_allowed_url("http://localhost:8080/api"));
    }

    #[test]
    fn test_is_allowed_url_internal_ip_rejected() {
        assert!(!is_allowed_url("http://192.168.1.1/api"));
    }

    #[test]
    fn test_is_allowed_url_similar_domain_rejected() {
        // Should not allow fake domains that look similar
        assert!(!is_allowed_url(
            "https://developer.api.autodesk.com.evil.com/api"
        ));
    }

    #[test]
    fn test_is_allowed_url_invalid_url() {
        assert!(!is_allowed_url("not-a-valid-url"));
    }

    #[test]
    fn test_is_allowed_url_empty() {
        assert!(!is_allowed_url(""));
    }

    #[test]
    fn test_is_allowed_url_subdomain() {
        // Subdomains of allowed domains should be allowed
        assert!(is_allowed_url("https://us.developer.api.autodesk.com/api"));
    }

    #[test]
    fn test_is_retryable_status_429() {
        assert!(is_retryable_status(429));
    }

    #[test]
    fn test_is_retryable_status_408() {
        assert!(is_retryable_status(408));
    }

    #[test]
    fn test_is_retryable_status_5xx() {
        assert!(is_retryable_status(500));
        assert!(is_retryable_status(502));
        assert!(is_retryable_status(503));
        assert!(is_retryable_status(504));
    }

    #[test]
    fn test_is_retryable_status_not_retryable() {
        assert!(!is_retryable_status(200));
        assert!(!is_retryable_status(201));
        assert!(!is_retryable_status(400));
        assert!(!is_retryable_status(401));
        assert!(!is_retryable_status(403));
        assert!(!is_retryable_status(404));
        assert!(!is_retryable_status(409));
        assert!(!is_retryable_status(422));
    }

    /// Helper: bind a TCP listener on a random port and return (addr, listener)
    fn bind_test_server() -> (std::net::SocketAddr, std::net::TcpListener) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        (addr, listener)
    }

    fn accept_and_respond(listener: &std::net::TcpListener, raw_response: &str) {
        use std::io::{Read, Write};
        let (mut stream, _) = listener.accept().unwrap();
        // Drain the request
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);
        stream.write_all(raw_response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }

    #[tokio::test]
    async fn test_retry_delay_from_response_with_retry_after_header() {
        let (addr, listener) = bind_test_server();
        let handle = std::thread::spawn(move || {
            accept_and_respond(
                &listener,
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 5\r\nContent-Length: 0\r\n\r\n",
            );
        });

        let client = reqwest::Client::new();
        let response = client.get(format!("http://{}", addr)).send().await.unwrap();
        let config = HttpClientConfig::default();
        let delay = retry_delay_from_response(&response, 0, &config);
        assert_eq!(delay, Duration::from_secs(5));
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_retry_delay_from_response_retry_after_capped_at_max_wait() {
        let (addr, listener) = bind_test_server();
        let handle = std::thread::spawn(move || {
            accept_and_respond(
                &listener,
                "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 300\r\nContent-Length: 0\r\n\r\n",
            );
        });

        let client = reqwest::Client::new();
        let response = client.get(format!("http://{}", addr)).send().await.unwrap();
        let config = HttpClientConfig {
            max_wait: 60,
            ..Default::default()
        };
        let delay = retry_delay_from_response(&response, 0, &config);
        assert_eq!(delay, Duration::from_secs(60));
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_retry_delay_from_response_fallback_to_exponential() {
        let (addr, listener) = bind_test_server();
        let handle = std::thread::spawn(move || {
            accept_and_respond(
                &listener,
                "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\n\r\n",
            );
        });

        let client = reqwest::Client::new();
        let response = client.get(format!("http://{}", addr)).send().await.unwrap();
        let config = HttpClientConfig::default();
        // attempt=0 -> calculate_delay(1, 1, 60) -> 1*2^1 = 2s + jitter
        let delay = retry_delay_from_response(&response, 0, &config);
        assert!(delay.as_secs() >= 2);
        assert!(delay.as_secs() <= 3);
        handle.join().unwrap();
    }

    #[tokio::test]
    async fn test_send_with_retry_success() {
        let (addr, listener) = bind_test_server();
        let handle = std::thread::spawn(move || {
            accept_and_respond(&listener, "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK");
        });

        let config = HttpClientConfig::default();
        let client = reqwest::Client::new();
        let url = format!("http://{}", addr);

        let response = send_with_retry(&config, || client.get(&url)).await;
        assert!(response.is_ok());
        assert_eq!(response.unwrap().status().as_u16(), 200);
        handle.join().unwrap();
    }
}
