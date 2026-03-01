// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Custom API call commands
//!
//! Execute arbitrary HTTP requests to APS API endpoints using the current authentication.
//! Supports GET, POST, PUT, PATCH, DELETE methods with query parameters, request bodies,
//! custom headers, and multiple output formats.

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use colored::Colorize;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue};
use reqwest::{Client, Method, Response, StatusCode};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::str::FromStr;

use crate::output::OutputFormat;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::{HttpClientConfig, is_allowed_url};

/// HTTP methods supported by the custom API command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    /// Convert to reqwest Method
    fn as_reqwest_method(self) -> Method {
        match self {
            HttpMethod::Get => Method::GET,
            HttpMethod::Post => Method::POST,
            HttpMethod::Put => Method::PUT,
            HttpMethod::Patch => Method::PATCH,
            HttpMethod::Delete => Method::DELETE,
        }
    }

    /// Check if this method supports a request body
    fn supports_body(self) -> bool {
        matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch)
    }
}

/// Error response structure
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    pub status_code: u16,
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Custom API call commands
#[derive(Debug, Subcommand)]
pub enum ApiCommands {
    /// Execute HTTP GET request
    Get {
        /// API endpoint path (e.g., /oss/v2/buckets)
        endpoint: String,

        /// Query parameter (KEY=VALUE, repeatable)
        #[arg(long = "query", value_parser = parse_key_value)]
        query: Vec<(String, String)>,

        /// Custom header (KEY:VALUE, repeatable)
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        header: Vec<(String, String)>,

        /// Save response to file
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Show response headers and status
        #[arg(short, long)]
        verbose: bool,
    },

    /// Execute HTTP POST request
    Post {
        /// API endpoint path (e.g., /oss/v2/buckets)
        endpoint: String,

        /// Inline JSON request body
        #[arg(short, long, conflicts_with = "data_file")]
        data: Option<String>,

        /// Read request body from file (use `-` for stdin)
        #[arg(short = 'f', long = "data-file", conflicts_with = "data")]
        data_file: Option<PathBuf>,

        /// Query parameter (KEY=VALUE, repeatable)
        #[arg(long = "query", value_parser = parse_key_value)]
        query: Vec<(String, String)>,

        /// Custom header (KEY:VALUE, repeatable)
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        header: Vec<(String, String)>,

        /// Save response to file (use `-` for stdout)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Show response headers and status
        #[arg(short, long)]
        verbose: bool,
    },

    /// Execute HTTP PUT request
    Put {
        /// API endpoint path
        endpoint: String,

        /// Inline JSON request body
        #[arg(short, long, conflicts_with = "data_file")]
        data: Option<String>,

        /// Read request body from file (use `-` for stdin)
        #[arg(short = 'f', long = "data-file", conflicts_with = "data")]
        data_file: Option<PathBuf>,

        /// Query parameter (KEY=VALUE, repeatable)
        #[arg(long = "query", value_parser = parse_key_value)]
        query: Vec<(String, String)>,

        /// Custom header (KEY:VALUE, repeatable)
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        header: Vec<(String, String)>,

        /// Save response to file (use `-` for stdout)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Show response headers and status
        #[arg(short, long)]
        verbose: bool,
    },

    /// Execute HTTP PATCH request
    Patch {
        /// API endpoint path
        endpoint: String,

        /// Inline JSON request body
        #[arg(short, long, conflicts_with = "data_file")]
        data: Option<String>,

        /// Read request body from file (use `-` for stdin)
        #[arg(short = 'f', long = "data-file", conflicts_with = "data")]
        data_file: Option<PathBuf>,

        /// Query parameter (KEY=VALUE, repeatable)
        #[arg(long = "query", value_parser = parse_key_value)]
        query: Vec<(String, String)>,

        /// Custom header (KEY:VALUE, repeatable)
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        header: Vec<(String, String)>,

        /// Save response to file (use `-` for stdout)
        #[arg(long = "out-file")]
        out_file: Option<PathBuf>,

        /// Show response headers and status
        #[arg(short, long)]
        verbose: bool,
    },

    /// Execute HTTP DELETE request
    Delete {
        /// API endpoint path
        endpoint: String,

        /// Query parameter (KEY=VALUE, repeatable)
        #[arg(long = "query", value_parser = parse_key_value)]
        query: Vec<(String, String)>,

        /// Custom header (KEY:VALUE, repeatable)
        #[arg(short = 'H', long = "header", value_parser = parse_header)]
        header: Vec<(String, String)>,

        /// Show response headers and status
        #[arg(short, long)]
        verbose: bool,
    },
}

impl ApiCommands {
    /// Execute the API command
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        http_config: &HttpClientConfig,
        output_format: OutputFormat,
    ) -> Result<()> {
        // Extract common parameters based on variant
        let (method, endpoint, query, headers, data, data_file, output_file, verbose) = match self {
            ApiCommands::Get {
                endpoint,
                query,
                header,
                out_file,
                verbose,
            } => (
                HttpMethod::Get,
                endpoint,
                query,
                header,
                None,
                None,
                out_file,
                verbose,
            ),

            ApiCommands::Post {
                endpoint,
                data,
                data_file,
                query,
                header,
                out_file,
                verbose,
            } => (
                HttpMethod::Post,
                endpoint,
                query,
                header,
                data,
                data_file,
                out_file,
                verbose,
            ),

            ApiCommands::Put {
                endpoint,
                data,
                data_file,
                query,
                header,
                out_file,
                verbose,
            } => (
                HttpMethod::Put,
                endpoint,
                query,
                header,
                data,
                data_file,
                out_file,
                verbose,
            ),

            ApiCommands::Patch {
                endpoint,
                data,
                data_file,
                query,
                header,
                out_file,
                verbose,
            } => (
                HttpMethod::Patch,
                endpoint,
                query,
                header,
                data,
                data_file,
                out_file,
                verbose,
            ),

            ApiCommands::Delete {
                endpoint,
                query,
                header,
                verbose,
            } => (
                HttpMethod::Delete,
                endpoint,
                query,
                header,
                None,
                None,
                None,
                verbose,
            ),
        };

        // Build full URL from endpoint
        let full_url = build_url(&config.base_url, &endpoint, &query)?;

        // Validate URL is allowed
        if !is_allowed_url(&full_url) {
            bail!(
                "Only APS API endpoints are allowed. Use a path like /oss/v2/buckets\n\
                 Hint: External URLs are not permitted for security reasons."
            );
        }

        // Parse request body if provided
        let body = parse_body(method, data, data_file)?;

        // Get auth token
        let token = get_auth_token(auth_client).await?;

        // Build and execute request
        let client = http_config.create_client()?;
        let response =
            execute_request(&client, method, &full_url, &token, &headers, body.as_ref()).await?;

        // Handle response
        handle_response(response, output_format, output_file, verbose).await
    }
}

/// Parse KEY=VALUE format for query parameters
fn parse_key_value(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid format '{}'. Expected KEY=VALUE", s));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

/// Parse KEY:VALUE format for headers
fn parse_header(s: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!(
            "Invalid header format '{}'. Expected KEY:VALUE (e.g., Content-Type:application/json)",
            s
        ));
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

/// Build full URL from base URL, endpoint, and query parameters.
/// IMPORTANT: Callers must validate the result with `is_allowed_url` before use.
fn build_url(base_url: &str, endpoint: &str, query_params: &[(String, String)]) -> Result<String> {
    // Handle relative vs absolute endpoints
    let mut url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
        endpoint.to_string()
    } else {
        // Ensure endpoint starts with /
        let endpoint = if endpoint.starts_with('/') {
            endpoint.to_string()
        } else {
            format!("/{}", endpoint)
        };
        format!("{}{}", base_url.trim_end_matches('/'), endpoint)
    };

    // Append query parameters
    if !query_params.is_empty() {
        let query_string: String = query_params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        if url.contains('?') {
            url = format!("{}&{}", url, query_string);
        } else {
            url = format!("{}?{}", url, query_string);
        }
    }

    Ok(url)
}

/// Parse and validate request body from --data or --data-file
fn parse_body(
    method: HttpMethod,
    data: Option<String>,
    data_file: Option<PathBuf>,
) -> Result<Option<Value>> {
    // Check if body is allowed for this method
    if !method.supports_body() {
        if data.is_some() || data_file.is_some() {
            bail!(
                "Request body is not allowed for {} requests",
                match method {
                    HttpMethod::Get => "GET",
                    HttpMethod::Delete => "DELETE",
                    _ => unreachable!(),
                }
            );
        }
        return Ok(None);
    }

    // Read body from data, file, or stdin
    let body_str =
        if let Some(data) = data {
            Some(data)
        } else if let Some(path) = data_file {
            if path.as_os_str() == "-" {
                use std::io::Read;
                let mut buf = String::new();
                std::io::stdin()
                    .lock()
                    .read_to_string(&mut buf)
                    .context("Failed to read request body from stdin")?;
                Some(buf)
            } else {
                Some(std::fs::read_to_string(&path).with_context(|| {
                    format!("Failed to read body from file: {}", path.display())
                })?)
            }
        } else {
            None
        };

    // Parse and validate JSON
    if let Some(body_str) = body_str {
        let value: Value =
            serde_json::from_str(&body_str).with_context(|| "Invalid JSON in request body")?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// Get authentication token from auth client
async fn get_auth_token(auth_client: &AuthClient) -> Result<String> {
    // Try 3-legged token first, fall back to 2-legged
    match auth_client.get_3leg_token().await {
        Ok(token) => Ok(token),
        Err(_) => {
            // Try 2-legged token
            auth_client.get_token().await.with_context(|| {
                "Not authenticated. Run 'raps auth login' first.\n\
                 Hint: Use 'raps auth login' for 3-legged auth or configure client credentials for 2-legged auth."
            })
        }
    }
}

/// Execute HTTP request with authentication
async fn execute_request(
    client: &Client,
    method: HttpMethod,
    url: &str,
    token: &str,
    custom_headers: &[(String, String)],
    body: Option<&Value>,
) -> Result<Response> {
    tracing::info!(
        method = %method.as_reqwest_method(),
        url = %raps_kernel::logging::redact_secrets(url),
        "HTTP request"
    );

    let mut request = client.request(method.as_reqwest_method(), url);

    // Add authorization header
    request = request.header(AUTHORIZATION, format!("Bearer {}", token));

    // Add custom headers (but prevent overriding Authorization)
    for (key, value) in custom_headers {
        if key.to_lowercase() == "authorization" {
            tracing::warn!("Ignoring attempt to override Authorization header");
            continue;
        }
        match (HeaderName::from_str(key), HeaderValue::from_str(value)) {
            (Ok(name), Ok(val)) => {
                request = request.header(name, val);
            }
            (Err(e), _) => tracing::warn!("Skipping invalid header name '{}': {}", key, e),
            (_, Err(e)) => tracing::warn!("Skipping invalid header value for '{}': {}", key, e),
        }
    }

    // Add body if present
    if let Some(body) = body {
        request = request.header(CONTENT_TYPE, "application/json").json(body);
    }

    let response = request.send().await.context("Failed to send request")?;

    Ok(response)
}

/// Handle HTTP response and format output
async fn handle_response(
    response: Response,
    output_format: OutputFormat,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    let status = response.status();
    let status_code = status.as_u16();

    // Collect headers for verbose output
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    // Print verbose output if requested
    if verbose {
        println!(
            "{}",
            format!(
                "HTTP/1.1 {} {}",
                status_code,
                status.canonical_reason().unwrap_or("")
            )
            .cyan()
        );
        for (key, value) in &headers {
            println!("{}: {}", key.dimmed(), value);
        }
        println!();
    }

    // Handle response based on content type and status
    if content_type.contains("application/json") {
        let body_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        // Try to parse as JSON
        let json: Result<Value, _> = serde_json::from_str(&body_text);

        match json {
            Ok(value) => {
                if status.is_success() {
                    // Save to file if requested
                    if let Some(ref path) = output_file {
                        let pretty = serde_json::to_string_pretty(&value)?;
                        if path.as_os_str() == "-" {
                            print!("{}", pretty);
                        } else {
                            std::fs::write(path, &pretty)?;
                            eprintln!("{} {}", "Saved to:".green(), path.display());
                        }
                    } else {
                        // Output using configured format
                        output_format.write(&value)?;
                    }
                    Ok(())
                } else {
                    // Error response
                    let error = ApiError {
                        status_code,
                        error_type: categorize_error(status_code),
                        message: extract_error_message(&value, status),
                        details: Some(value),
                    };
                    output_format.write(&error)?;
                    bail!(
                        "API error ({}): {}",
                        status_code,
                        extract_error_message(
                            error.details.as_ref().unwrap_or(&Value::Null),
                            status
                        )
                    );
                }
            }
            Err(_) => {
                // JSON parse failed, treat as text
                if status.is_success() {
                    if let Some(ref path) = output_file {
                        if path.as_os_str() == "-" {
                            print!("{}", body_text);
                        } else {
                            std::fs::write(path, &body_text)?;
                            eprintln!("{} {}", "Saved to:".green(), path.display());
                        }
                    } else {
                        println!("{}", body_text);
                    }
                    Ok(())
                } else {
                    bail!("API error ({}): {}", status_code, body_text);
                }
            }
        }
    } else if content_type.starts_with("text/") || content_type.contains("xml") {
        // Text response
        let body_text = response
            .text()
            .await
            .context("Failed to read response body")?;

        if status.is_success() {
            if let Some(path) = output_file {
                if path.as_os_str() == "-" {
                    print!("{}", body_text);
                } else {
                    std::fs::write(&path, &body_text)?;
                    eprintln!("{} {}", "Saved to:".green(), path.display());
                }
            } else {
                println!("{}", body_text);
            }
            Ok(())
        } else {
            bail!("API error ({}): {}", status_code, body_text);
        }
    } else {
        // Binary response
        let bytes = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        if status.is_success() {
            if let Some(path) = output_file {
                if path.as_os_str() == "-" {
                    use std::io::Write;
                    std::io::stdout()
                        .lock()
                        .write_all(&bytes)
                        .context("Failed to write to stdout")?;
                } else {
                    std::fs::write(&path, &bytes)?;
                    eprintln!(
                        "{} {} ({} bytes)",
                        "Saved to:".green(),
                        path.display(),
                        bytes.len()
                    );
                }
                Ok(())
            } else if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
                // stdout is piped — write binary directly
                use std::io::Write;
                std::io::stdout()
                    .lock()
                    .write_all(&bytes)
                    .context("Failed to write to stdout")?;
                Ok(())
            } else {
                bail!(
                    "Binary response received. Use --out-file to save to a file (or pipe stdout).\n\
                     Content-Type: {}, Size: {} bytes",
                    content_type,
                    bytes.len()
                );
            }
        } else {
            bail!(
                "API error ({}): binary error response ({} bytes)",
                status_code,
                bytes.len()
            );
        }
    }
}

/// Categorize error based on status code
fn categorize_error(status_code: u16) -> String {
    match status_code {
        401 | 403 => "authentication".to_string(),
        400 | 422 => "validation".to_string(),
        404 => "not_found".to_string(),
        429 => "rate_limited".to_string(),
        500..=599 => "server_error".to_string(),
        _ => "error".to_string(),
    }
}

/// Extract error message from JSON response
fn extract_error_message(value: &Value, status: StatusCode) -> String {
    // Try common error message fields
    if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        return msg.to_string();
    }
    if let Some(msg) = value.get("error").and_then(|v| v.as_str()) {
        return msg.to_string();
    }
    if let Some(msg) = value.get("reason").and_then(|v| v.as_str()) {
        return msg.to_string();
    }
    if let Some(msg) = value.get("developerMessage").and_then(|v| v.as_str()) {
        return msg.to_string();
    }

    // Fallback to status text
    status
        .canonical_reason()
        .unwrap_or("Request failed")
        .to_string()
}
