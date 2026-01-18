//! Python bindings for RAPS - Rust Autodesk Platform Services
//!
//! This module provides native Python bindings using PyO3, exposing RAPS
//! functionality as a Python library for programmatic use.

use pyo3::exceptions::{PyConnectionError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// Exceptions
// ============================================================================

pyo3::create_exception!(raps, RapsError, pyo3::exceptions::PyException);
pyo3::create_exception!(raps, AuthenticationError, RapsError);
pyo3::create_exception!(raps, NotFoundError, RapsError);
pyo3::create_exception!(raps, RateLimitError, RapsError);
pyo3::create_exception!(raps, ValidationError, RapsError);

/// Convert anyhow::Error to Python exception
fn to_py_err(err: anyhow::Error) -> PyErr {
    let msg = err.to_string();
    if msg.contains("401") || msg.contains("authentication") || msg.contains("Unauthorized") {
        AuthenticationError::new_err(msg)
    } else if msg.contains("404") || msg.contains("not found") || msg.contains("Not Found") {
        NotFoundError::new_err(msg)
    } else if msg.contains("429") || msg.contains("rate limit") {
        RateLimitError::new_err(msg)
    } else if msg.contains("400") || msg.contains("invalid") || msg.contains("validation") {
        ValidationError::new_err(msg)
    } else {
        RapsError::new_err(msg)
    }
}

// ============================================================================
// Bucket class
// ============================================================================

/// Represents an OSS bucket
#[pyclass]
#[derive(Clone)]
pub struct Bucket {
    #[pyo3(get)]
    pub key: String,
    #[pyo3(get)]
    pub owner: String,
    #[pyo3(get)]
    pub created_date: u64,
    #[pyo3(get)]
    pub policy: String,
    #[pyo3(get)]
    pub region: Option<String>,
}

#[pymethods]
impl Bucket {
    fn __repr__(&self) -> String {
        format!("Bucket(key='{}', policy='{}')", self.key, self.policy)
    }

    fn __str__(&self) -> String {
        self.key.clone()
    }
}

// ============================================================================
// Object class
// ============================================================================

/// Represents an object in an OSS bucket
#[pyclass]
#[derive(Clone)]
pub struct Object {
    #[pyo3(get)]
    pub bucket_key: String,
    #[pyo3(get)]
    pub object_key: String,
    #[pyo3(get)]
    pub object_id: String,
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    pub sha1: Option<String>,
}

#[pymethods]
impl Object {
    /// Get the base64-encoded URN for this object
    #[getter]
    fn urn(&self) -> String {
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&self.object_id)
    }

    fn __repr__(&self) -> String {
        format!(
            "Object(bucket='{}', key='{}', size={})",
            self.bucket_key, self.object_key, self.size
        )
    }

    fn __str__(&self) -> String {
        self.object_key.clone()
    }
}

// ============================================================================
// TranslationJob class
// ============================================================================

/// Represents a Model Derivative translation job
#[pyclass]
#[derive(Clone)]
pub struct TranslationJob {
    #[pyo3(get)]
    pub urn: String,
    #[pyo3(get)]
    pub status: String,
    #[pyo3(get)]
    pub progress: String,
    // Internal fields for polling
    client_id: String,
    client_secret: String,
    base_url: String,
}

#[pymethods]
impl TranslationJob {
    fn __repr__(&self) -> String {
        format!(
            "TranslationJob(urn='{}', status='{}', progress='{}')",
            self.urn, self.status, self.progress
        )
    }

    /// Wait for the translation to complete
    ///
    /// Args:
    ///     timeout: Maximum seconds to wait (default: 600)
    ///     poll_interval: Seconds between status checks (default: 5)
    ///
    /// Returns:
    ///     Updated TranslationJob with final status
    fn wait(
        &self,
        py: Python<'_>,
        timeout: Option<u64>,
        poll_interval: Option<u64>,
    ) -> PyResult<TranslationJob> {
        let timeout = timeout.unwrap_or(600);
        let poll_interval = poll_interval.unwrap_or(5);
        let urn = self.urn.clone();
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),
                    base_url: base_url.clone(),
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let derivative = raps_derivative::DerivativeClient::new(config, auth);

                let start = std::time::Instant::now();
                loop {
                    let manifest = derivative
                        .get_manifest(&urn)
                        .await
                        .map_err(to_py_err)?;

                    let status = manifest.status.clone();
                    let progress = manifest.progress.clone();

                    if status == "success" || status == "failed" || status == "timeout" {
                        return Ok(TranslationJob {
                            urn: urn.clone(),
                            status,
                            progress,
                            client_id: client_id.clone(),
                            client_secret: client_secret.clone(),
                            base_url: base_url.clone(),
                        });
                    }

                    if start.elapsed().as_secs() > timeout {
                        return Err(PyRuntimeError::new_err(format!(
                            "Translation timed out after {} seconds",
                            timeout
                        )));
                    }

                    tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
                }
            })
        })
    }
}

// ============================================================================
// Hub class
// ============================================================================

/// Represents a Data Management hub
#[pyclass]
#[derive(Clone)]
pub struct Hub {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub hub_type: String,
    #[pyo3(get)]
    pub region: Option<String>,
}

#[pymethods]
impl Hub {
    fn __repr__(&self) -> String {
        format!("Hub(id='{}', name='{}')", self.id, self.name)
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }
}

// ============================================================================
// Project class
// ============================================================================

/// Represents a project in a hub
#[pyclass]
#[derive(Clone)]
pub struct Project {
    #[pyo3(get)]
    pub id: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub project_type: String,
}

#[pymethods]
impl Project {
    fn __repr__(&self) -> String {
        format!("Project(id='{}', name='{}')", self.id, self.name)
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }
}

// ============================================================================
// BucketsManager class
// ============================================================================

/// Manager for bucket operations
#[pyclass]
pub struct BucketsManager {
    client_id: String,
    client_secret: String,
    base_url: String,
}

#[pymethods]
impl BucketsManager {
    /// List all buckets
    ///
    /// Args:
    ///     region: Optional region filter ("US" or "EMEA")
    ///     limit: Maximum number of buckets to return
    ///
    /// Returns:
    ///     List of Bucket objects
    fn list(&self, py: Python<'_>, region: Option<String>, limit: Option<usize>) -> PyResult<Vec<Bucket>> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let buckets = oss.list_buckets().await.map_err(to_py_err)?;

                let mut result: Vec<Bucket> = buckets
                    .into_iter()
                    .filter(|b| {
                        if let Some(ref r) = region {
                            b.region.as_ref().map(|br| br == r).unwrap_or(false)
                        } else {
                            true
                        }
                    })
                    .map(|b| Bucket {
                        key: b.bucket_key,
                        owner: String::new(),
                        created_date: b.created_date,
                        policy: b.policy_key,
                        region: b.region,
                    })
                    .collect();

                if let Some(lim) = limit {
                    result.truncate(lim);
                }

                Ok(result)
            })
        })
    }

    /// Create a new bucket
    ///
    /// Args:
    ///     key: Bucket key (unique identifier)
    ///     policy: Retention policy ("transient", "temporary", "persistent")
    ///     region: Storage region ("US" or "EMEA")
    ///
    /// Returns:
    ///     Created Bucket object
    fn create(
        &self,
        py: Python<'_>,
        key: String,
        policy: Option<String>,
        region: Option<String>,
    ) -> PyResult<Bucket> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let policy = policy.unwrap_or_else(|| "transient".to_string());
        let region = region.unwrap_or_else(|| "US".to_string());

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let policy_enum = policy
                    .parse::<raps_oss::RetentionPolicy>()
                    .map_err(|e| ValidationError::new_err(e))?;

                let region_enum = match region.to_uppercase().as_str() {
                    "US" => raps_oss::Region::US,
                    "EMEA" => raps_oss::Region::EMEA,
                    _ => return Err(ValidationError::new_err("Invalid region: use 'US' or 'EMEA'")),
                };

                let bucket = oss
                    .create_bucket(&key, policy_enum, region_enum)
                    .await
                    .map_err(to_py_err)?;

                Ok(Bucket {
                    key: bucket.bucket_key,
                    owner: bucket.bucket_owner,
                    created_date: bucket.created_date,
                    policy: bucket.policy_key,
                    region: Some(region),
                })
            })
        })
    }

    /// Get bucket details
    ///
    /// Args:
    ///     key: Bucket key
    ///
    /// Returns:
    ///     Bucket object
    fn get(&self, py: Python<'_>, key: String) -> PyResult<Bucket> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let bucket = oss.get_bucket_details(&key).await.map_err(to_py_err)?;

                Ok(Bucket {
                    key: bucket.bucket_key,
                    owner: bucket.bucket_owner,
                    created_date: bucket.created_date,
                    policy: bucket.policy_key,
                    region: None,
                })
            })
        })
    }

    /// Delete a bucket
    ///
    /// Args:
    ///     key: Bucket key to delete
    fn delete(&self, py: Python<'_>, key: String) -> PyResult<()> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                oss.delete_bucket(&key).await.map_err(to_py_err)?;
                Ok(())
            })
        })
    }
}

// ============================================================================
// ObjectsManager class
// ============================================================================

/// Manager for object operations within a bucket
#[pyclass]
pub struct ObjectsManager {
    bucket_key: String,
    client_id: String,
    client_secret: String,
    base_url: String,
}

#[pymethods]
impl ObjectsManager {
    /// List objects in the bucket
    ///
    /// Args:
    ///     limit: Maximum number of objects to return
    ///
    /// Returns:
    ///     List of Object objects
    fn list(&self, py: Python<'_>, limit: Option<usize>) -> PyResult<Vec<Object>> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let bucket_key = self.bucket_key.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let objects = oss.list_objects(&bucket_key).await.map_err(to_py_err)?;

                let mut result: Vec<Object> = objects
                    .into_iter()
                    .map(|o| Object {
                        bucket_key: o.bucket_key,
                        object_key: o.object_key,
                        object_id: o.object_id,
                        size: o.size,
                        sha1: o.sha1,
                    })
                    .collect();

                if let Some(lim) = limit {
                    result.truncate(lim);
                }

                Ok(result)
            })
        })
    }

    /// Upload a file to the bucket
    ///
    /// Args:
    ///     path: Local file path to upload
    ///     object_key: Optional object key (defaults to filename)
    ///
    /// Returns:
    ///     Uploaded Object
    fn upload(&self, py: Python<'_>, path: String, object_key: Option<String>) -> PyResult<Object> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let bucket_key = self.bucket_key.clone();

        let file_path = PathBuf::from(&path);
        let obj_key = object_key.unwrap_or_else(|| {
            file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let info = oss
                    .upload_object(&bucket_key, &obj_key, &file_path)
                    .await
                    .map_err(to_py_err)?;

                Ok(Object {
                    bucket_key: info.bucket_key,
                    object_key: info.object_key,
                    object_id: info.object_id,
                    size: info.size,
                    sha1: info.sha1,
                })
            })
        })
    }

    /// Download an object from the bucket
    ///
    /// Args:
    ///     object_key: Key of the object to download
    ///     path: Local path to save the file
    ///
    /// Returns:
    ///     Path to downloaded file
    fn download(&self, py: Python<'_>, object_key: String, path: String) -> PyResult<String> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let bucket_key = self.bucket_key.clone();
        let output_path = PathBuf::from(&path);

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                oss.download_object(&bucket_key, &object_key, &output_path)
                    .await
                    .map_err(to_py_err)?;

                Ok(output_path.to_string_lossy().to_string())
            })
        })
    }

    /// Delete an object from the bucket
    ///
    /// Args:
    ///     object_key: Key of the object to delete
    fn delete(&self, py: Python<'_>, object_key: String) -> PyResult<()> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let bucket_key = self.bucket_key.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                oss.delete_object(&bucket_key, &object_key)
                    .await
                    .map_err(to_py_err)?;
                Ok(())
            })
        })
    }

    /// Get a signed download URL for an object
    ///
    /// Args:
    ///     object_key: Key of the object
    ///     minutes: URL expiration in minutes (2-60, default: 2)
    ///
    /// Returns:
    ///     Signed URL string
    fn signed_url(&self, py: Python<'_>, object_key: String, minutes: Option<u32>) -> PyResult<String> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let bucket_key = self.bucket_key.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let oss = raps_oss::OssClient::new(config, auth);

                let signed = oss
                    .get_signed_download_url(&bucket_key, &object_key, minutes)
                    .await
                    .map_err(to_py_err)?;

                signed
                    .url
                    .ok_or_else(|| RapsError::new_err("No URL returned"))
            })
        })
    }
}

// ============================================================================
// HubsManager class
// ============================================================================

/// Manager for hub operations (requires 3-legged auth)
///
/// Note: 3-legged authentication must be set up via the RAPS CLI first
/// using `raps auth login`. The stored token will be used automatically.
#[pyclass]
pub struct HubsManager {
    client_id: String,
    client_secret: String,
    base_url: String,
}

#[pymethods]
impl HubsManager {
    /// List all hubs (requires 3-legged authentication)
    ///
    /// Note: You must first run `raps auth login` from the CLI to authenticate.
    /// The stored 3-legged token will be used automatically.
    ///
    /// Returns:
    ///     List of Hub objects
    ///
    /// Raises:
    ///     AuthenticationError: If no 3-legged token is available
    fn list(&self, py: Python<'_>) -> PyResult<Vec<Hub>> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let dm = raps_dm::DataManagementClient::new(config, auth);

                let hubs = dm
                    .list_hubs()
                    .await
                    .map_err(to_py_err)?;

                Ok(hubs
                    .into_iter()
                    .map(|h| Hub {
                        id: h.id,
                        name: h.attributes.name,
                        hub_type: h.hub_type,
                        region: h.attributes.region,
                    })
                    .collect())
            })
        })
    }
}

// ============================================================================
// Client class (main entry point)
// ============================================================================

/// Main RAPS client for interacting with Autodesk Platform Services
///
/// Create a client with explicit credentials:
///     client = Client(client_id="xxx", client_secret="yyy")
///
/// Or load from environment:
///     client = Client.from_env()
///
/// Example:
///     client = Client.from_env()
///     buckets = client.buckets.list()
///     for bucket in buckets:
///         print(f"{bucket.key}: {bucket.policy}")
#[pyclass]
pub struct Client {
    client_id: String,
    client_secret: String,
    base_url: String,
    access_token: Option<String>,
}

#[pymethods]
impl Client {
    /// Create a new RAPS client with 2-legged authentication
    ///
    /// Args:
    ///     client_id: APS application client ID
    ///     client_secret: APS application client secret
    ///     base_url: Optional API base URL (for testing)
    #[new]
    #[pyo3(signature = (client_id, client_secret, base_url=None))]
    fn new(client_id: String, client_secret: String, base_url: Option<String>) -> Self {
        Client {
            client_id,
            client_secret,
            base_url: base_url.unwrap_or_else(|| "https://developer.api.autodesk.com".to_string()),
            access_token: None,
        }
    }

    /// Create a client from environment variables
    ///
    /// Reads APS_CLIENT_ID and APS_CLIENT_SECRET from environment
    #[staticmethod]
    fn from_env() -> PyResult<Self> {
        let client_id = std::env::var("APS_CLIENT_ID")
            .map_err(|_| ValidationError::new_err("APS_CLIENT_ID environment variable not set"))?;
        let client_secret = std::env::var("APS_CLIENT_SECRET")
            .map_err(|_| ValidationError::new_err("APS_CLIENT_SECRET environment variable not set"))?;
        let base_url = std::env::var("APS_BASE_URL")
            .unwrap_or_else(|_| "https://developer.api.autodesk.com".to_string());

        Ok(Client {
            client_id,
            client_secret,
            base_url,
            access_token: None,
        })
    }

    /// Test 2-legged authentication
    ///
    /// Returns:
    ///     True if authentication succeeds
    fn test_auth(&self, py: Python<'_>) -> PyResult<bool> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id,
                    client_secret,
                    base_url,
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config);

                match auth.get_token().await {
                    Ok(_) => Ok(true),
                    Err(e) => Err(to_py_err(e)),
                }
            })
        })
    }

    /// Get bucket operations manager
    #[getter]
    fn buckets(&self) -> BucketsManager {
        BucketsManager {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            base_url: self.base_url.clone(),
        }
    }

    /// Get hub operations manager (requires login for 3-legged auth)
    #[getter]
    fn hubs(&self) -> HubsManager {
        HubsManager {
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            base_url: self.base_url.clone(),
        }
    }

    /// Get object operations manager for a specific bucket
    ///
    /// Args:
    ///     bucket_key: The bucket to operate on
    ///
    /// Returns:
    ///     ObjectsManager for the bucket
    fn objects(&self, bucket_key: String) -> ObjectsManager {
        ObjectsManager {
            bucket_key,
            client_id: self.client_id.clone(),
            client_secret: self.client_secret.clone(),
            base_url: self.base_url.clone(),
        }
    }

    /// Start a translation job
    ///
    /// Args:
    ///     urn: Base64-encoded URN of the source file
    ///     output_format: Target format (default: "svf2")
    ///     force: Force re-translation even if exists
    ///
    /// Returns:
    ///     TranslationJob object
    fn translate(
        &self,
        py: Python<'_>,
        urn: String,
        output_format: Option<String>,
        force: Option<bool>,
    ) -> PyResult<TranslationJob> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();
        let format = output_format.unwrap_or_else(|| "svf2".to_string());
        let force = force.unwrap_or(false);

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),
                    base_url: base_url.clone(),
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let derivative = raps_derivative::DerivativeClient::new(config, auth);

                // Convert format string to OutputFormat enum
                let output_format = format
                    .parse::<raps_derivative::OutputFormat>()
                    .map_err(|e| ValidationError::new_err(e))?;

                derivative
                    .translate(&urn, output_format, None)
                    .await
                    .map_err(to_py_err)?;

                // Get initial status
                let manifest = derivative.get_manifest(&urn).await.map_err(to_py_err)?;

                Ok(TranslationJob {
                    urn,
                    status: manifest.status,
                    progress: manifest.progress,
                    client_id,
                    client_secret,
                    base_url,
                })
            })
        })
    }

    /// Get translation status
    ///
    /// Args:
    ///     urn: Base64-encoded URN of the source file
    ///
    /// Returns:
    ///     TranslationJob with current status
    fn get_translation_status(&self, py: Python<'_>, urn: String) -> PyResult<TranslationJob> {
        let client_id = self.client_id.clone();
        let client_secret = self.client_secret.clone();
        let base_url = self.base_url.clone();

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            rt.block_on(async {
                let config = raps_kernel::config::Config {
                    client_id: client_id.clone(),
                    client_secret: client_secret.clone(),
                    base_url: base_url.clone(),
                    callback_url: "http://localhost:8080/callback".to_string(),
                    da_nickname: None,
                    http_config: raps_kernel::http::HttpClientConfig::default(),
                };
                let auth = raps_kernel::auth::AuthClient::new(config.clone());
                let derivative = raps_derivative::DerivativeClient::new(config, auth);

                let manifest = derivative.get_manifest(&urn).await.map_err(to_py_err)?;

                Ok(TranslationJob {
                    urn,
                    status: manifest.status,
                    progress: manifest.progress,
                    client_id,
                    client_secret,
                    base_url,
                })
            })
        })
    }

    /// Generate a URN for a bucket/object combination
    ///
    /// Args:
    ///     bucket_key: Bucket key
    ///     object_key: Object key
    ///
    /// Returns:
    ///     Base64-encoded URN string
    fn get_urn(&self, bucket_key: String, object_key: String) -> String {
        use base64::Engine;
        let object_id = format!("urn:adsk.objects:os.object:{}/{}", bucket_key, object_key);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(object_id)
    }

    fn __repr__(&self) -> String {
        format!(
            "Client(client_id='{}...', base_url='{}')",
            &self.client_id[..8.min(self.client_id.len())],
            self.base_url
        )
    }

    /// Context manager entry
    fn __enter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Context manager exit
    fn __exit__(
        &self,
        _exc_type: Option<PyObject>,
        _exc_val: Option<PyObject>,
        _exc_tb: Option<PyObject>,
    ) -> bool {
        false
    }
}

// ============================================================================
// Module definition
// ============================================================================

/// RAPS - Python bindings for Autodesk Platform Services
///
/// Example:
///     from raps import Client
///
///     client = Client.from_env()
///     buckets = client.buckets.list()
///     for bucket in buckets:
///         print(f"{bucket.key}: {bucket.policy}")
#[pymodule]
fn raps(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Classes
    m.add_class::<Client>()?;
    m.add_class::<Bucket>()?;
    m.add_class::<Object>()?;
    m.add_class::<TranslationJob>()?;
    m.add_class::<Hub>()?;
    m.add_class::<Project>()?;
    m.add_class::<BucketsManager>()?;
    m.add_class::<ObjectsManager>()?;
    m.add_class::<HubsManager>()?;

    // Exceptions - use the type object macro for PyO3 0.22+
    m.add("RapsError", m.py().get_type_bound::<RapsError>())?;
    m.add("AuthenticationError", m.py().get_type_bound::<AuthenticationError>())?;
    m.add("NotFoundError", m.py().get_type_bound::<NotFoundError>())?;
    m.add("RateLimitError", m.py().get_type_bound::<RateLimitError>())?;
    m.add("ValidationError", m.py().get_type_bound::<ValidationError>())?;

    // Version
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    Ok(())
}
