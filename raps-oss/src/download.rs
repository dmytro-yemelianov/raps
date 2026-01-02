// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Download operations

use raps_kernel::{HttpClient, RapsError, Result};
use std::path::Path;

/// Download client for OSS operations
pub struct DownloadClient {
    http: HttpClient,
    base_url: String,
}

impl DownloadClient {
    /// Create new download client
    pub fn new(http: HttpClient, base_url: String) -> Self {
        Self { http, base_url }
    }

    /// Download an object to a file
    pub async fn download_object(
        &self,
        _bucket_key: &str,
        _object_key: &str,
        _output_path: &Path,
    ) -> Result<()> {
        // Stub - full implementation pending
        Err(RapsError::Internal {
            message: "Not yet implemented".to_string(),
        })
    }
}
