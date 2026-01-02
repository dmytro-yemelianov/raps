# Data Model: RAPS Ecosystem Improvements

**Feature**: 001-raps-ecosystem-improvements  
**Date**: 2025-12-29  
**Updated**: 2025-12-29  
**Status**: Implemented (types now live in `raps-kernel`)

## Overview

This document defines the key data structures and types for the RAPS ecosystem improvements. These types are now implemented in the `raps-kernel` crate and used across all service crates.

---

## Core Types (raps-kernel) ✅ IMPLEMENTED

### Authentication

```rust
/// OAuth token with expiry tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// The access token value
    pub token: String,
    /// Token type (typically "Bearer")
    pub token_type: String,
    /// Expiry time in seconds from issuance
    pub expires_in: u64,
    /// When the token was issued (Unix timestamp)
    pub issued_at: u64,
    /// Refresh token for 3-legged auth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Granted OAuth scopes
    pub scopes: Vec<String>,
}

impl AccessToken {
    /// Check if token is expired (with 30s buffer)
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now >= self.issued_at + self.expires_in - 30
    }
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub callback_url: Option<String>,
    pub scopes: Vec<String>,
}
```

### OSS Types

```rust
/// Base64-URL encoded URN for translation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Urn(pub String);

impl Urn {
    /// Create URN from bucket/object path
    pub fn from_path(bucket: &str, object: &str) -> Self {
        let path = format!("urn:adsk.objects:os.object:{}/{}", bucket, object);
        let encoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&path);
        Self(encoded)
    }
    
    /// Decode URN to original path
    pub fn decode(&self) -> Result<String> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&self.0)?;
        Ok(String::from_utf8(bytes)?)
    }
}

/// Bucket key with validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BucketKey(String);

impl BucketKey {
    /// Create validated bucket key (lowercase alphanumeric + hyphens, 3-128 chars)
    pub fn new(key: impl Into<String>) -> Result<Self> {
        let key = key.into();
        if key.len() < 3 || key.len() > 128 {
            return Err(anyhow!("Bucket key must be 3-128 characters"));
        }
        if !key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
            return Err(anyhow!("Bucket key must be lowercase alphanumeric with hyphens"));
        }
        Ok(Self(key))
    }
}

/// Bucket information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BucketInfo {
    /// Unique bucket key
    pub bucket_key: String,
    /// Creation timestamp (ISO 8601)
    pub created_date: String,
    /// Retention policy: transient, temporary, persistent
    pub policy_key: String,
    /// Owner ID
    pub owner_id: Option<String>,
    /// Permission level for current user
    pub permissions: Option<Vec<Permission>>,
}

/// Object in a bucket
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ObjectInfo {
    /// Object key (filename)
    pub object_key: String,
    /// Parent bucket key
    pub bucket_key: String,
    /// Object ID
    pub object_id: String,
    /// SHA-1 hash of content
    pub sha1: String,
    /// Size in bytes
    pub size: u64,
    /// MIME content type
    pub content_type: String,
    /// Location URL
    pub location: String,
}
```

### Upload Session Types

```rust
/// Resumable upload session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadSession {
    /// Unique session ID
    pub session_id: String,
    /// Target bucket
    pub bucket_key: String,
    /// Target object key
    pub object_key: String,
    /// Total file size in bytes
    pub total_size: u64,
    /// Chunk size used
    pub chunk_size: u64,
    /// Status of each chunk (indexed by part number)
    pub chunks: Vec<ChunkStatus>,
    /// Upload ID from OSS (for multipart)
    pub upload_id: Option<String>,
    /// Session creation time
    pub created_at: u64,
    /// Last activity time
    pub updated_at: u64,
}

/// Status of an individual chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkStatus {
    Pending,
    InProgress,
    Completed { etag: String },
    Failed { error: String, attempts: u32 },
}

/// Parallel upload progress
#[derive(Debug, Clone)]
pub struct UploadProgress {
    pub total_chunks: usize,
    pub completed_chunks: usize,
    pub bytes_uploaded: u64,
    pub bytes_total: u64,
    pub current_speed: f64, // bytes per second
}
```

### Translation Types

```rust
/// Translation job status
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TranslationStatus {
    /// Source URN
    pub urn: String,
    /// Current status
    pub status: TranslationState,
    /// Progress percentage (0-100)
    pub progress: String,
    /// Region (US, EMEA)
    pub region: String,
    /// Output formats requested
    pub output_formats: Vec<String>,
    /// Error messages if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<TranslationMessage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TranslationState {
    Pending,
    Inprogress,
    Success,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TranslationMessage {
    pub r#type: String,
    pub code: String,
    pub message: String,
}
```

---

## Configuration Types

### HTTP Client Configuration

```rust
/// HTTP client configuration
#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    /// Request timeout
    pub timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Maximum retries for transient errors
    pub max_retries: u32,
    /// Base delay for exponential backoff
    pub retry_base_delay: Duration,
    /// Maximum delay between retries
    pub retry_max_delay: Duration,
    /// Whether to add jitter to retry delays
    pub retry_jitter: bool,
    /// Maximum concurrent requests for parallel operations
    pub concurrency: usize,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_base_delay: Duration::from_secs(1),
            retry_max_delay: Duration::from_secs(30),
            retry_jitter: true,
            concurrency: 5,
        }
    }
}
```

### APS Endpoints Configuration

```rust
/// APS API endpoint URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApsEndpoints {
    pub auth: String,
    pub oss: String,
    pub derivative: String,
    pub data_management: String,
    pub webhooks: String,
    pub design_automation: String,
    pub issues: String,
    pub rfi: String,
    pub reality_capture: String,
}

impl Default for ApsEndpoints {
    fn default() -> Self {
        Self {
            auth: "https://developer.api.autodesk.com/authentication/v2".into(),
            oss: "https://developer.api.autodesk.com/oss/v2".into(),
            derivative: "https://developer.api.autodesk.com/modelderivative/v2".into(),
            data_management: "https://developer.api.autodesk.com/data/v1".into(),
            webhooks: "https://developer.api.autodesk.com/webhooks/v1".into(),
            design_automation: "https://developer.api.autodesk.com/da/us-east/v3".into(),
            issues: "https://developer.api.autodesk.com/construction/issues/v1".into(),
            rfi: "https://developer.api.autodesk.com/construction/rfis/v1".into(),
            reality_capture: "https://developer.api.autodesk.com/photo-to-3d/v1".into(),
        }
    }
}
```

---

## Output Schema Types

### Command Output Envelopes

```rust
/// Standard success response envelope
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct SuccessResponse<T> {
    /// Response data
    pub data: T,
    /// Metadata about the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
}

/// Response metadata
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ResponseMeta {
    /// Total count (for paginated responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,
    /// Next page marker
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_marker: Option<String>,
    /// Request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// Standard error response envelope
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ErrorResponse {
    /// Error code (matches exit code)
    pub code: i32,
    /// Human-readable error message
    pub message: String,
    /// Error cause chain
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
    /// Suggested remediation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}
```

### Specific Output Types

```rust
/// Bucket list command output
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BucketListOutput {
    pub buckets: Vec<BucketInfo>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_marker: Option<String>,
}

/// Object list command output
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ObjectListOutput {
    pub objects: Vec<ObjectInfo>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_marker: Option<String>,
}

/// Upload command output
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UploadOutput {
    pub object_key: String,
    pub bucket_key: String,
    pub urn: String,
    pub size: u64,
    pub sha1: String,
    pub duration_ms: u64,
}

/// Translation start output
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct TranslationStartOutput {
    pub urn: String,
    pub output_format: String,
    pub status: String,
    pub message: String,
}
```

---

## MCP Types

### Tool Definitions

```rust
/// MCP tool call result
#[derive(Debug, Serialize)]
pub struct McpToolResult<T> {
    pub content: Vec<McpContent<T>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct McpContent<T> {
    pub r#type: String,
    pub text: Option<String>,
    pub data: Option<T>,
}

/// Tool parameter schemas
#[derive(Debug, Deserialize, JsonSchema)]
pub struct BucketListParams {
    /// Filter by region: US or EMEA
    #[serde(default)]
    pub region: Option<String>,
    /// Maximum results to return
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize { 100 }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BucketCreateParams {
    /// Bucket key (lowercase, 3-128 chars)
    pub key: String,
    /// Retention policy: transient, temporary, persistent
    #[serde(default = "default_policy")]
    pub policy: String,
    /// Region: US or EMEA
    #[serde(default = "default_region")]
    pub region: String,
}

fn default_policy() -> String { "transient".into() }
fn default_region() -> String { "US".into() }
```

---

## Error Types

```rust
/// RAPS error with context
#[derive(Debug, thiserror::Error)]
pub enum RapsError {
    #[error("Authentication failed: {message}")]
    AuthError { message: String, code: i32 },
    
    #[error("Resource not found: {resource}")]
    NotFound { resource: String },
    
    #[error("API error: {message}")]
    ApiError { 
        message: String, 
        status: u16,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    #[error("Network error: {message}")]
    NetworkError { 
        message: String,
        #[source]
        source: Option<reqwest::Error>,
    },
    
    #[error("Upload error: {message}")]
    UploadError { 
        message: String,
        chunk: Option<usize>,
    },
    
    #[error("Internal error: {message}")]
    Internal { message: String },
}

impl RapsError {
    /// Get exit code for this error type
    pub fn exit_code(&self) -> i32 {
        match self {
            RapsError::AuthError { code, .. } => *code,
            RapsError::NotFound { .. } => 4,
            RapsError::ApiError { .. } => 5,
            RapsError::ConfigError { .. } => 2,
            RapsError::NetworkError { .. } => 5,
            RapsError::UploadError { .. } => 5,
            RapsError::Internal { .. } => 6,
        }
    }
}
```

---

## Type Relationships

```
┌──────────────────────────────────────────────────────────────────┐
│                        raps-core                                  │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────┐       │
│  │ AuthConfig  │───▶│ AccessToken │───▶│ ApsClient       │       │
│  └─────────────┘    └─────────────┘    └────────┬────────┘       │
│                                                  │                │
│  ┌─────────────────┐    ┌───────────────────────┼───────────┐    │
│  │ HttpClientConfig│───▶│     API Clients       │           │    │
│  └─────────────────┘    │  ┌────────────────────┼───────┐   │    │
│                         │  │ OssClient         ─┘       │   │    │
│  ┌─────────────────┐    │  │ DerivativeClient           │   │    │
│  │ ApsEndpoints    │───▶│  │ DataManagementClient       │   │    │
│  └─────────────────┘    │  │ DesignAutomationClient     │   │    │
│                         │  │ IssuesClient               │   │    │
│                         │  └────────────────────────────┘   │    │
│                         └───────────────────────────────────┘    │
│                                                                   │
│  ┌────────────┐  ┌────────────┐  ┌───────────────┐               │
│  │ BucketInfo │  │ ObjectInfo │  │UploadSession  │               │
│  └────────────┘  └────────────┘  └───────────────┘               │
│         │               │               │                         │
│         ▼               ▼               ▼                         │
│  ┌─────────────────────────────────────────────────┐             │
│  │              Output Schema Types                 │             │
│  │  BucketListOutput, ObjectListOutput, etc.       │             │
│  └─────────────────────────────────────────────────┘             │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
         │                    │                    │
         ▼                    ▼                    ▼
   ┌──────────┐        ┌──────────┐        ┌──────────┐
   │   raps   │        │  aps-tui │        │   MCP    │
   │  (CLI)   │        │  (TUI)   │        │ (Server) │
   └──────────┘        └──────────┘        └──────────┘
```

---

## Schema Generation

To generate JSON Schema documentation:

```rust
use schemars::schema_for;

fn main() {
    // Generate schemas for all output types
    let bucket_list_schema = schema_for!(BucketListOutput);
    let object_list_schema = schema_for!(ObjectListOutput);
    let upload_schema = schema_for!(UploadOutput);
    
    // Write to docs/schemas/
    std::fs::write(
        "docs/schemas/bucket-list.json",
        serde_json::to_string_pretty(&bucket_list_schema).unwrap()
    ).unwrap();
}
```

Or via CLI:

```bash
raps schema bucket-list > docs/schemas/bucket-list.json
raps schema object-list > docs/schemas/object-list.json
raps schema --all > docs/schemas/all-schemas.json
```
