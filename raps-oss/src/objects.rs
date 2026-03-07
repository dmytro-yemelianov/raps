// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object operations for the OSS API.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use raps_kernel::error::RapsError;
use sha1::{Digest, Sha1};
use std::path::{Component, Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};

use raps_kernel::progress;

use crate::OssClient;
use crate::types::*;

impl OssClient {
    fn normalize_lexical_path(path: &Path) -> PathBuf {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    normalized.pop();
                }
                other => normalized.push(other.as_os_str()),
            }
        }
        normalized
    }

    /// Upload a file to a bucket using S3 signed URLs
    /// Automatically uses multipart upload for files larger than 5MB
    pub async fn upload_object(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
    ) -> Result<ObjectInfo> {
        self.upload_object_with_options(bucket_key, object_key, file_path, false)
            .await
    }

    /// Upload a file with resume option
    /// If resume is true, will attempt to resume an interrupted upload
    pub async fn upload_object_with_options(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
        resume: bool,
    ) -> Result<ObjectInfo> {
        let metadata = tokio::fs::metadata(file_path)
            .await
            .context("Failed to get file metadata")?;
        let file_size = metadata.len();
        tracing::debug!(
            bucket = bucket_key,
            key = object_key,
            size = file_size,
            "upload_object"
        );

        // Use multipart upload for files larger than threshold
        if file_size > MultipartUploadState::MULTIPART_THRESHOLD {
            self.upload_multipart(bucket_key, object_key, file_path, resume)
                .await
        } else {
            self.upload_single_part(bucket_key, object_key, file_path)
                .await
        }
    }

    /// Upload a small file using single-part upload
    pub(crate) async fn upload_single_part(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
    ) -> Result<ObjectInfo> {
        // Read file
        let mut file = File::open(file_path)
            .await
            .context("Failed to open file for upload")?;

        let metadata = file
            .metadata()
            .await
            .context("Failed to get file metadata")?;
        let file_size = metadata.len();

        // Create progress bar (hidden in non-interactive mode)
        let pb = progress::file_progress(file_size, &format!("Uploading {}", object_key));

        // Step 1: Get signed S3 upload URL
        pb.set_message(format!("Getting upload URL for {}", object_key));
        let signed = self
            .get_signed_upload_url(bucket_key, object_key, None, None)
            .await?;

        if signed.urls.is_empty() {
            anyhow::bail!("No upload URLs returned from signed upload request");
        }

        // Step 2: Stream upload directly to S3 instead of loading into memory
        pb.set_message(format!("Uploading {} to S3", object_key));
        let s3_url = &signed.urls[0];

        // Create a streaming body that reads the file in chunks
        use futures_util::stream::TryStreamExt;
        use tokio_util::codec::{BytesCodec, FramedRead};

        // Reset file position to start
        file.seek(std::io::SeekFrom::Start(0)).await?;

        // Create a stream that reads the file in chunks
        let file_stream = FramedRead::new(file, BytesCodec::new())
            .map_ok(|bytes| bytes.freeze())
            .map_err(std::io::Error::other);

        let body = reqwest::Body::wrap_stream(file_stream);

        let _upload_start = std::time::Instant::now();
        let response = self
            .http_client
            .put(s3_url)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Length", file_size.to_string())
            .body(body)
            .send()
                    .await
                    .context("Failed to upload to S3")?;
                raps_kernel::profiler::record_http_request(_upload_start.elapsed());
            
                tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(s3_url), "HTTP response");
            
                if !response.status().is_success() {
                    return Err(RapsError::from_response(response).await.into());
                }
            
                pb.set_position(file_size);
                    // Step 3: Complete the upload
        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &signed.upload_key)
            .await?;

        pb.finish_with_message(format!("Uploaded {}", object_key));

        Ok(object_info)
    }

    /// Download an object from a bucket using S3 signed URLs (new API)
    pub async fn download_object(
        &self,
        bucket_key: &str,
        object_key: &str,
        output_path: &Path,
    ) -> Result<()> {
        tracing::debug!(bucket = bucket_key, key = object_key, "download_object");
        // Step 1: Get signed S3 download URL
        let signed = self
            .get_signed_download_url(bucket_key, object_key, None)
            .await?;

        let download_url = signed
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL returned"))?;

        // Step 2: Download from S3 with retry logic
        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&download_url)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&download_url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let total_size = signed
            .size
            .unwrap_or(response.content_length().unwrap_or(0));

        // Create progress bar (hidden in non-interactive mode)
        let pb = progress::file_progress(total_size, &format!("Downloading {}", object_key));

        // Validate output path stays within current directory, even if the file does not exist yet.
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let canon_cwd = cwd.canonicalize().unwrap_or_else(|_| cwd.clone());
        let abs_output = if output_path.is_absolute() {
            output_path.to_path_buf()
        } else {
            cwd.join(output_path)
        };
        if let Some(parent) = abs_output.parent() {
            let checked_parent = parent
                .canonicalize()
                .unwrap_or_else(|_| Self::normalize_lexical_path(parent));
            if !checked_parent.starts_with(&canon_cwd) {
                anyhow::bail!(
                    "Path '{}' escapes working directory '{}'",
                    output_path.display(),
                    cwd.display()
                );
            }
        }

        // Stream download
        let mut file = File::create(output_path)
            .await
            .context("Failed to create output file")?;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error while downloading")?;
            file.write_all(&chunk)
                .await
                .context("Failed to write to file")?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        pb.finish_with_message(format!("Downloaded {}", object_key));
        Ok(())
    }

    /// Download an object and stream it to any async writer (stdout, file, etc.)
    pub async fn download_object_to_writer(
        &self,
        bucket_key: &str,
        object_key: &str,
        writer: &mut (impl tokio::io::AsyncWrite + Unpin),
    ) -> Result<()> {
        tracing::debug!(bucket = bucket_key, key = object_key, "download_object_to_writer");
        let signed = self
            .get_signed_download_url(bucket_key, object_key, None)
            .await?;

        let download_url = signed
            .url
            .ok_or_else(|| anyhow::anyhow!("No download URL returned"))?;

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&download_url)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&download_url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let total_size = signed
            .size
            .unwrap_or(response.content_length().unwrap_or(0));

        let pb = progress::file_progress(total_size, &format!("Downloading {}", object_key));

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.context("Error while downloading")?;
            writer
                .write_all(&chunk)
                .await
                .context("Failed to write output")?;
            downloaded += chunk.len() as u64;
            pb.set_position(downloaded);
        }

        writer.flush().await?;
        pb.finish_with_message(format!("Downloaded {}", object_key));
        Ok(())
    }

    /// List objects in a bucket
    pub async fn list_objects(&self, bucket_key: &str) -> Result<Vec<ObjectItem>> {
        self.list_objects_with_limit(bucket_key, None).await
    }

    /// List objects in a bucket, returning at most `limit` items if specified.
    pub async fn list_objects_with_limit(
        &self,
        bucket_key: &str,
        limit: Option<usize>,
    ) -> Result<Vec<ObjectItem>> {
        const MAX_PAGES: usize = 100;
        let token = self.auth.get_token().await?;
        let mut all_objects = Vec::new();
        let mut start_at: Option<String> = None;
        let mut page = 0;

        loop {
            page += 1;
            if page > MAX_PAGES {
                tracing::warn!(
                    pages = MAX_PAGES,
                    objects = all_objects.len(),
                    "Reached maximum page limit for object listing"
                );
                break;
            }
            let mut url = format!("{}/buckets/{}/objects", self.config.oss_url(), bucket_key);
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
                self.http_client.get(&url).bearer_auth(&token)
            })
            .await?;

            tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

            if !response.status().is_success() {
                return Err(RapsError::from_response(response).await.into());
            }

            let response_text = response
                .text()
                .await
                .context("Failed to read objects response")?;

            let objects_response: ObjectsResponse = serde_json::from_str(&response_text)
                .with_context(|| format!("Failed to parse objects response: {}", response_text))?;

            all_objects.extend(objects_response.items);

            // Stop early if we've reached the requested limit
            if let Some(max) = limit
                && all_objects.len() >= max
            {
                all_objects.truncate(max);
                break;
            }

            if objects_response.next.is_none() {
                break;
            }
            start_at = objects_response.next;
        }

        Ok(all_objects)
    }

    /// Delete an object from a bucket
    pub async fn delete_object(&self, bucket_key: &str, object_key: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.config.oss_url(),
            bucket_key,
            object_key
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        Ok(())
    }

    /// Get the SHA-1 hash of an existing remote object.
    ///
    /// Returns `Ok(Some(sha1))` if the object exists, or `Ok(None)` if the
    /// object does not exist (HTTP 404).  Any other HTTP error is propagated.
    pub async fn get_object_details_sha1(
        &self,
        bucket_key: &str,
        object_key: &str,
    ) -> Result<Option<String>> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}/details",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let details: crate::types::ObjectDetails = response
            .json()
            .await
            .context("Failed to parse object details response")?;

        Ok(Some(details.sha1))
    }

    /// Compute the SHA-1 of a local file and compare it to the remote object.
    ///
    /// Returns `Ok(Some(object_info))` when an identical object already exists
    /// in the bucket, or `Ok(None)` when no duplicate is found.
    pub async fn check_duplicate(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &std::path::Path,
    ) -> Result<Option<ObjectInfo>> {
        // Compute local SHA-1
        let file_bytes = tokio::fs::read(file_path)
            .await
            .context("Failed to read file for SHA-1 computation")?;
        let mut hasher = Sha1::new();
        hasher.update(&file_bytes);
        let local_sha1 = hex::encode(hasher.finalize());

        // Fetch remote SHA-1
        let remote_sha1 = self
            .get_object_details_sha1(bucket_key, object_key)
            .await?;

        match remote_sha1 {
            Some(remote) if remote.to_lowercase() == local_sha1.to_lowercase() => {
                tracing::info!(
                    bucket = bucket_key,
                    key = object_key,
                    sha1 = %local_sha1,
                    "cache_hit: identical object already exists, skipping upload"
                );
                // Duplicate found — return full ObjectInfo converted from ObjectDetails
                let details = self.get_object_details(bucket_key, object_key).await?;
                Ok(Some(ObjectInfo {
                    bucket_key: details.bucket_key,
                    object_key: details.object_key,
                    object_id: details.object_id,
                    sha1: Some(details.sha1),
                    size: details.size,
                    location: details.location,
                    content_type: Some(details.content_type),
                }))
            }
            _ => Ok(None),
        }
    }

    /// Get detailed metadata for an object without downloading it
    ///
    /// Returns extended information including size, SHA1 hash, content type,
    /// and timestamps.
    pub async fn get_object_details(
        &self,
        bucket_key: &str,
        object_key: &str,
    ) -> Result<ObjectDetails> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}/details",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let details: ObjectDetails = response
            .json()
            .await
            .context("Failed to parse object details response")?;

        Ok(details)
    }

    /// Get a signed S3 URL for direct download (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    pub async fn get_signed_download_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3DownloadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3download",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        if let Some(mins) = minutes_expiration {
            url = format!("{}?minutesExpiration={}", url, mins);
        }

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let signed: SignedS3DownloadResponse = response
            .json()
            .await
            .context("Failed to parse signed URL response")?;

        Ok(signed)
    }

    /// Get a signed S3 URL for direct upload (bypasses OSS servers)
    ///
    /// The signed URL expires in 2 minutes by default.
    /// Returns an upload key that must be used to complete the upload.
    pub async fn get_signed_upload_url(
        &self,
        bucket_key: &str,
        object_key: &str,
        parts: Option<u32>,
        minutes_expiration: Option<u32>,
    ) -> Result<SignedS3UploadResponse> {
        let token = self.auth.get_token().await?;
        let mut url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        let mut params = Vec::new();
        if let Some(p) = parts {
            params.push(format!("parts={}", p));
        }
        if let Some(mins) = minutes_expiration {
            params.push(format!("minutesExpiration={}", mins));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let signed: SignedS3UploadResponse = response
            .json()
            .await
            .context("Failed to parse signed URL response")?;

        Ok(signed)
    }

    /// Complete an S3 signed upload
    pub async fn complete_signed_upload(
        &self,
        bucket_key: &str,
        object_key: &str,
        upload_key: &str,
    ) -> Result<ObjectInfo> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}/signeds3upload",
            self.config.oss_url(),
            bucket_key,
            urlencoding::encode(object_key)
        );

        let body = serde_json::json!({
            "uploadKey": upload_key
        });

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("Content-Type", "application/json")
                .json(&body)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        // Get response text for debugging
        let response_text = response
            .text()
            .await
            .context("Failed to read upload completion response")?;

        // Try to parse as ObjectInfo
        let object_info: ObjectInfo = serde_json::from_str(&response_text).with_context(|| {
            format!(
                "Failed to parse upload completion response: {}",
                response_text
            )
        })?;

        Ok(object_info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use raps_kernel::auth::AuthClient;
    use raps_kernel::config::Config;
    use raps_kernel::http::HttpClientConfig;

    fn create_test_oss_client() -> OssClient {
        let config = Config {
            client_id: "test".to_string(),
            client_secret: "secret".to_string(),
            base_url: "https://developer.api.autodesk.com".to_string(),
            callback_url: "http://localhost:8080/callback".to_string(),
            da_nickname: None,
            http_config: HttpClientConfig::default(),
        };
        let auth = AuthClient::new(config.clone());
        OssClient::new(config, auth)
    }

    #[test]
    fn test_get_urn() {
        let client = create_test_oss_client();
        let urn = client.get_urn("my-bucket", "my-object.dwg");

        assert!(!urn.contains("urn:adsk.objects:os.object:"));
        assert!(!urn.contains("my-bucket"));
        assert!(!urn.contains("my-object.dwg"));
        assert!(!urn.contains("+"));
        assert!(!urn.contains("/"));
        assert!(!urn.contains("="));
    }

    #[test]
    fn test_oss_client_url_generation() {
        let client = create_test_oss_client();
        let urn = client.get_urn("bucket", "object.dwg");
        // URN should be base64 encoded
        assert!(
            urn.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn test_get_urn_special_characters() {
        let client = create_test_oss_client();
        let urn = client.get_urn("bucket-with-dash", "object with spaces.dwg");
        // URN should handle special characters
        assert!(!urn.is_empty());
        assert!(
            urn.chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn test_get_urn_unicode() {
        let client = create_test_oss_client();
        let urn = client.get_urn("test-bucket", "\u{0444}\u{0430}\u{0439}\u{043b}.dwg"); // Cyrillic filename
        assert!(!urn.is_empty());
    }
}
