// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! OSS and translation tool implementations for the MCP server.

use std::str::FromStr;

use raps_derivative::OutputFormat;
use raps_oss::{Region, RetentionPolicy};

use super::server::{RapsServer, format_size, validate_file_path};

impl RapsServer {
    pub(crate) async fn auth_test(&self) -> String {
        let auth = self.get_auth_client().await;
        match auth.get_token().await {
            Ok(_) => "Authentication successful! 2-legged OAuth credentials are valid.".to_string(),
            Err(e) => format!("Authentication failed: {}", e),
        }
    }

    pub(crate) async fn auth_status(&self) -> String {
        let auth = self.get_auth_client().await;
        let config = self.config();
        let mut status = String::new();

        // Check 2-legged
        let two_legged_valid = match auth.get_token().await {
            Ok(_) => {
                status.push_str("2-legged OAuth: Valid\n");
                true
            }
            Err(_) => {
                status.push_str("2-legged OAuth: Not configured or invalid\n");
                false
            }
        };

        // Check 3-legged
        let (three_legged_valid, three_legged_expired) = match auth.get_3leg_token().await {
            Ok(_) => {
                status.push_str("3-legged OAuth: Valid (user logged in)\n");
                (true, false)
            }
            Err(e) => {
                status
                    .push_str("3-legged OAuth: Not logged in (run 'raps auth login' to log in)\n");
                let err_msg = e.to_string().to_lowercase();
                (
                    false,
                    err_msg.contains("expired") || err_msg.contains("refresh"),
                )
            }
        };

        // Append tool availability summary
        let auth_state = super::auth_guidance::AuthState {
            has_client_id: !config.client_id.is_empty(),
            has_client_secret: !config.client_secret.is_empty(),
            two_legged_valid,
            three_legged_valid,
            three_legged_expired,
        };
        status.push_str(&super::auth_guidance::get_tool_availability_summary(
            &auth_state,
        ));

        status
    }

    pub(crate) async fn auth_login(&self) -> String {
        // MCP cannot perform interactive OAuth login directly
        // Return guidance for the user
        "3-legged OAuth login requires browser interaction.\n\n\
        To authenticate, run one of the following commands in your terminal:\n\n\
        1. Browser-based login (recommended):\n\
           raps auth login\n\n\
        2. Device code flow (for headless environments):\n\
           raps auth login --device\n\n\
        After completing authentication, your MCP session will automatically use the stored tokens."
            .to_string()
    }

    pub(crate) async fn auth_logout(&self) -> String {
        let auth = self.get_auth_client().await;

        match auth.logout().await {
            Ok(()) => {
                "Successfully logged out. 3-legged OAuth tokens have been cleared.".to_string()
            }
            Err(e) => format!("Logout failed: {}", e),
        }
    }

    pub(crate) async fn bucket_list(&self, region: Option<String>, limit: Option<usize>) -> String {
        let client = self.get_oss_client().await;
        let limit = Self::clamp_limit(limit, 100, 500);

        match client.list_buckets().await {
            Ok(all_buckets) => {
                // Filter by region if specified
                let filtered: Vec<_> = all_buckets
                    .into_iter()
                    .filter(|b| {
                        if let Some(ref r) = region {
                            b.region
                                .as_ref()
                                .map(|br| br.eq_ignore_ascii_case(r))
                                .unwrap_or(true)
                        } else {
                            true
                        }
                    })
                    .collect();

                let total = filtered.len();
                let buckets: Vec<_> = filtered.into_iter().take(limit).collect();
                let shown = buckets.len();

                let mut output = if shown < total {
                    format!("Showing {} of {} bucket(s):\n\n", shown, total)
                } else {
                    format!("Found {} bucket(s):\n\n", total)
                };
                for b in &buckets {
                    output.push_str(&format!(
                        "* {} (policy: {}, region: {})\n",
                        b.bucket_key,
                        b.policy_key,
                        b.region.as_deref().unwrap_or("unknown")
                    ));
                }
                output
            }
            Err(e) => format!("Error listing buckets: {}", e),
        }
    }

    pub(crate) async fn bucket_create(
        &self,
        bucket_key: String,
        policy: String,
        region: String,
    ) -> String {
        let client = self.get_oss_client().await;

        let retention = match policy.to_lowercase().as_str() {
            "transient" => RetentionPolicy::Transient,
            "temporary" => RetentionPolicy::Temporary,
            "persistent" => RetentionPolicy::Persistent,
            _ => {
                return "Invalid policy. Use transient, temporary, or persistent.".to_string();
            }
        };

        let reg = match region.to_uppercase().as_str() {
            "EMEA" => Region::EMEA,
            "US" => Region::US,
            _ => return "Invalid region. Use US or EMEA.".to_string(),
        };

        match client.create_bucket(&bucket_key, retention, reg).await {
            Ok(bucket) => format!(
                "Bucket created successfully:\n* Key: {}\n* Owner: {}\n* Policy: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key
            ),
            Err(e) => format!("Failed to create bucket: {}", e),
        }
    }

    pub(crate) async fn bucket_get(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.get_bucket_details(&bucket_key).await {
            Ok(bucket) => format!(
                "Bucket: {}\n* Owner: {}\n* Policy: {}\n* Created: {}",
                bucket.bucket_key, bucket.bucket_owner, bucket.policy_key, bucket.created_date
            ),
            Err(e) => format!("Bucket not found or error: {e}"),
        }
    }

    pub(crate) async fn bucket_delete(&self, bucket_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.delete_bucket(&bucket_key).await {
            Ok(()) => format!("Bucket '{}' deleted successfully", bucket_key),
            Err(e) => format!("Failed to delete bucket: {}", e),
        }
    }

    pub(crate) async fn object_list(&self, bucket_key: String, limit: Option<usize>) -> String {
        let client = self.get_oss_client().await;
        let limit = Self::clamp_limit(limit, 100, 1000);

        match client.list_objects(&bucket_key).await {
            Ok(all_objects) => {
                let total = all_objects.len();
                let objects: Vec<_> = all_objects.into_iter().take(limit).collect();
                let shown = objects.len();
                let mut output = if shown < total {
                    format!(
                        "Showing {} of {} object(s) in '{}':\n\n",
                        shown, total, bucket_key
                    )
                } else {
                    format!("Found {} object(s) in '{}':\n\n", total, bucket_key)
                };
                for obj in &objects {
                    output.push_str(&format!("* {} ({} bytes)\n", obj.object_key, obj.size));
                }
                output
            }
            Err(e) => format!("Error listing objects: {}", e),
        }
    }

    pub(crate) async fn object_delete(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.delete_object(&bucket_key, &object_key).await {
            Ok(()) => format!(
                "Object '{}' deleted from bucket '{}'",
                object_key, bucket_key
            ),
            Err(e) => format!("Failed to delete object: {}", e),
        }
    }

    pub(crate) async fn object_signed_url(
        &self,
        bucket_key: String,
        object_key: String,
        minutes: u32,
    ) -> String {
        let client = self.get_oss_client().await;
        let minutes = minutes.clamp(2, 60);

        match client
            .get_signed_download_url(&bucket_key, &object_key, Some(minutes))
            .await
        {
            Ok(response) => {
                if let Some(url) = response.url {
                    format!(
                        "Pre-signed download URL (expires in {} minutes):\n{}",
                        minutes, url
                    )
                } else {
                    "No URL returned. The object may have been uploaded in chunks.".to_string()
                }
            }
            Err(e) => format!("Failed to generate signed URL: {}", e),
        }
    }

    pub(crate) async fn object_urn(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;
        let urn = client.get_urn(&bucket_key, &object_key);
        format!("URN for {}/{}:\n{}", bucket_key, object_key, urn)
    }

    pub(crate) async fn translate_start(&self, urn: String, format: String) -> String {
        let client = self.get_derivative_client().await;

        let output_format = match OutputFormat::from_str(&format) {
            Ok(format) => format,
            Err(_) => {
                return "Invalid output format. Supported: svf2, svf, thumbnail, obj, stl, step, iges, ifc.".to_string();
            }
        };

        match client
            .translate(
                &urn,
                output_format,
                None,
                raps_derivative::MdRegion::default(),
                false,
            )
            .await
        {
            Ok(result) => format!(
                "Translation job started:\n* Result: {}\n* URN: {}",
                result.result, result.urn
            ),
            Err(e) => format!("Translation failed: {}", e),
        }
    }

    pub(crate) async fn translate_status(&self, urn: String) -> String {
        let client = self.get_derivative_client().await;

        match client.get_manifest(&urn).await {
            Ok(manifest) => {
                let mut output = format!(
                    "Translation status: {} ({})\n* URN: {}\n* Region: {}\n* Has thumbnail: {}",
                    manifest.status,
                    manifest.progress,
                    manifest.urn,
                    manifest.region,
                    manifest.has_thumbnail
                );
                if !manifest.derivatives.is_empty() {
                    output.push_str(&format!(
                        "\n* Derivatives: {} output(s)",
                        manifest.derivatives.len()
                    ));
                    for d in &manifest.derivatives {
                        let name = d.name.as_deref().unwrap_or("-");
                        let prog = d.progress.as_deref().unwrap_or("-");
                        output.push_str(&format!(
                            "\n  - {} [{}] status: {} ({})",
                            name, d.output_type, d.status, prog
                        ));
                    }
                }
                output
            }
            Err(e) => format!("Could not get translation status: {}", e),
        }
    }

    pub(crate) async fn object_upload(
        &self,
        bucket_key: String,
        file_path: String,
        object_key: Option<String>,
    ) -> String {
        use std::path::Path;

        let path = Path::new(&file_path);

        // Security: reject paths to sensitive locations
        if let Err(msg) = validate_file_path(path) {
            return msg;
        }

        // Validate file exists
        if !path.exists() {
            return format!("Error: File not found: {}", file_path);
        }

        if !path.is_file() {
            return format!("Error: Path is not a file: {}", file_path);
        }

        // Determine object key from filename if not provided
        let obj_key = object_key.unwrap_or_else(|| {
            path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "unnamed".to_string())
        });

        let client = self.get_oss_client().await;

        match client.upload_object(&bucket_key, &obj_key, path).await {
            Ok(info) => {
                let urn = client.get_urn(&bucket_key, &obj_key);
                format!(
                    "Uploaded '{}' to '{}'\n* Object Key: {}\n* Size: {} bytes\n* SHA1: {}\n* URN: {}",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    bucket_key,
                    info.object_key,
                    info.size,
                    info.sha1.unwrap_or_else(|| "-".to_string()),
                    urn
                )
            }
            Err(e) => format!("Failed to upload file: {}", e),
        }
    }

    pub(crate) async fn object_upload_batch(
        &self,
        bucket_key: String,
        file_paths: Vec<String>,
    ) -> String {
        use std::path::Path;
        use std::sync::Arc;
        use tokio::sync::Semaphore;

        if file_paths.is_empty() {
            return "Error: No files specified for upload.".to_string();
        }

        let client = self.get_oss_client().await;
        let semaphore = Arc::new(Semaphore::new(4)); // 4-way concurrency

        let mut handles = Vec::new();

        for file_path in file_paths.clone() {
            let client = client.clone();
            let bucket_key = bucket_key.clone();
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .expect("semaphore closed unexpectedly");

            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit until done
                let path = std::path::PathBuf::from(&file_path);

                if !path.exists() {
                    return (
                        file_path,
                        false,
                        None::<u64>,
                        Some("File not found".to_string()),
                    );
                }

                if !path.is_file() {
                    return (
                        file_path,
                        false,
                        None::<u64>,
                        Some("Not a file".to_string()),
                    );
                }

                let obj_key = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unnamed".to_string());

                match client.upload_object(&bucket_key, &obj_key, &path).await {
                    Ok(info) => (file_path, true, Some(info.size), None::<String>),
                    Err(e) => (file_path, false, None::<u64>, Some(e.to_string())),
                }
            });

            handles.push(handle);
        }

        // Collect results
        let mut successful = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for handle in handles {
            match handle.await {
                Ok((path, success, size, error)) => {
                    let path_display = Path::new(&path)
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or(path);

                    if success {
                        successful += 1;
                        let size_display = size.map(format_size).unwrap_or_default();
                        results.push(format!("\u{2713} {} ({})", path_display, size_display));
                    } else {
                        failed += 1;
                        let err_msg = error.unwrap_or_else(|| "Unknown error".to_string());
                        results.push(format!("\u{2717} {} ({})", path_display, err_msg));
                    }
                }
                Err(_join_err) => {
                    failed += 1;
                    results.push("\u{2717} (internal task error)".to_string());
                }
            }
        }

        format!(
            "Batch upload complete: {} succeeded, {} failed\n\nResults:\n{}",
            successful,
            failed,
            results.join("\n")
        )
    }

    pub(crate) async fn object_download(
        &self,
        bucket_key: String,
        object_key: String,
        output_path: String,
    ) -> String {
        use std::path::Path;

        let client = self.get_oss_client().await;

        // Security: reject paths to sensitive locations
        let path = Path::new(&output_path);
        if let Err(msg) = validate_file_path(path) {
            return msg;
        }

        // Check if parent directory exists
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            return format!("Error: Directory does not exist: {}", parent.display());
        }

        match client.download_object(&bucket_key, &object_key, path).await {
            Ok(()) => {
                // Get file size from downloaded file
                let size = std::fs::metadata(&output_path)
                    .map(|m| m.len())
                    .unwrap_or(0);
                format!(
                    "Downloaded '{}' to '{}'\n* Size: {} bytes",
                    object_key, output_path, size
                )
            }
            Err(e) => format!("Failed to download object: {}", e),
        }
    }

    pub(crate) async fn object_info(&self, bucket_key: String, object_key: String) -> String {
        let client = self.get_oss_client().await;

        match client.get_object_details(&bucket_key, &object_key).await {
            Ok(details) => {
                let size_display = format_size(details.size);
                let urn = client.get_urn(&bucket_key, &object_key);

                format!(
                    "Object: {} in {}\n\n\
                     * Size: {} bytes ({})\n\
                     * Content-Type: {}\n\
                     * SHA1: {}\n\
                     * Created: {}\n\
                     * Modified: {}\n\
                     * URN: {}",
                    details.object_key,
                    details.bucket_key,
                    details.size,
                    size_display,
                    details.content_type,
                    details.sha1,
                    details.created_date.unwrap_or_else(|| "-".to_string()),
                    details
                        .last_modified_date
                        .unwrap_or_else(|| "-".to_string()),
                    urn
                )
            }
            Err(e) => format!("Failed to get object details: {}", e),
        }
    }

    pub(crate) async fn object_copy(
        &self,
        source_bucket: String,
        source_key: String,
        dest_bucket: String,
        dest_key: Option<String>,
    ) -> String {
        let client = self.get_oss_client().await;
        let destination_key = dest_key.unwrap_or_else(|| source_key.clone());

        // Check if destination exists first (non-destructive)
        match client
            .get_object_details(&dest_bucket, &destination_key)
            .await
        {
            Ok(existing) => {
                return format!(
                    "Warning: Object '{}' already exists in '{}' (skipped)\n\
                     * Existing size: {} bytes\n\
                     * Delete it first if you want to overwrite.",
                    destination_key, dest_bucket, existing.size
                );
            }
            Err(_) => {
                // Object doesn't exist, proceed with copy
            }
        }

        // OSS doesn't have a direct copy API, so we download and re-upload
        // For now, use a temporary file approach
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("raps_copy_{}", uuid::Uuid::new_v4()));

        // Download to temp -- always clean up on any exit path
        let result = match client
            .download_object(&source_bucket, &source_key, &temp_path)
            .await
        {
            Ok(_) => {
                // Upload to destination
                match client
                    .upload_object(&dest_bucket, &destination_key, &temp_path)
                    .await
                {
                    Ok(info) => {
                        let urn = client.get_urn(&dest_bucket, &destination_key);
                        format!(
                            "Copied '{}' from '{}' to '{}'\n* Size: {} bytes\n* New URN: {}",
                            source_key, source_bucket, dest_bucket, info.size, urn
                        )
                    }
                    Err(e) => format!("Failed to copy to destination: {}", e),
                }
            }
            Err(e) => format!("Failed to read source object: {}", e),
        };

        // Clean up temp file -- always runs regardless of success/failure
        let _ = std::fs::remove_file(&temp_path);

        result
    }

    pub(crate) async fn object_delete_batch(
        &self,
        bucket_key: String,
        object_keys: Vec<String>,
    ) -> String {
        if object_keys.is_empty() {
            return "Error: No objects specified for deletion.".to_string();
        }

        let client = self.get_oss_client().await;
        let mut deleted = 0;
        let mut skipped = 0;
        let mut failed = 0;
        let mut results = Vec::new();

        for object_key in &object_keys {
            match client.delete_object(&bucket_key, object_key).await {
                Ok(()) => {
                    deleted += 1;
                    results.push(format!("\u{2713} {} (deleted)", object_key));
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("404") || err_str.contains("not found") {
                        skipped += 1;
                        results.push(format!("\u{25CB} {} (not found, skipped)", object_key));
                    } else {
                        failed += 1;
                        results.push(format!("\u{2717} {} ({})", object_key, err_str));
                    }
                }
            }
        }

        format!(
            "Batch delete complete: {} deleted, {} skipped, {} failed\n\nResults:\n{}",
            deleted,
            skipped,
            failed,
            results.join("\n")
        )
    }
}
