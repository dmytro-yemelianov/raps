// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Copy and batch operations for the OSS API.

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::OssClient;
use crate::types::*;

impl OssClient {
    // ============== COPY & BATCH OPERATIONS ==============

    /// Server-side copy of an object using `x-ads-copy-from` header.
    /// Much more efficient than download+re-upload for same-region copies.
    pub async fn copy_object(
        &self,
        src_bucket: &str,
        object_key: &str,
        dest_bucket: &str,
        dest_key: Option<&str>,
    ) -> Result<ObjectDetails> {
        let token = self.auth.get_token().await?;
        let destination_key = dest_key.unwrap_or(object_key);
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.config.oss_url(),
            dest_bucket,
            urlencoding::encode(destination_key)
        );
        let copy_from = format!("{}/objects/{}", src_bucket, urlencoding::encode(object_key));

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .put(&url)
                .bearer_auth(&token)
                .header("x-ads-copy-from", &copy_from)
                .header("Content-Length", "0")
        })
        .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Failed to copy '{}/{}' to '{}/{}' ({status}): {error_text}",
                src_bucket,
                object_key,
                dest_bucket,
                destination_key
            );
        }

        response
            .json()
            .await
            .context("Failed to parse copy response")
    }

    /// Batch copy objects from one bucket to another with concurrent execution.
    /// Uses a semaphore to limit concurrency to 10 concurrent requests.
    pub async fn batch_copy_objects(
        &self,
        src_bucket: &str,
        dest_bucket: &str,
        object_keys: &[String],
    ) -> Result<BatchResult<ObjectDetails>> {
        let semaphore = Arc::new(Semaphore::new(10));
        let mut join_set = tokio::task::JoinSet::new();

        for key in object_keys {
            let client = self.clone();
            let src = src_bucket.to_string();
            let dest = dest_bucket.to_string();
            let key = key.clone();
            let sem = semaphore.clone();

            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");
                let result = client.copy_object(&src, &key, &dest, None).await;
                (key, result)
            });
        }

        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok((key, Ok(details))) => {
                    succeeded += 1;
                    results.push(BatchItemResult {
                        key,
                        result: Ok(details),
                    });
                }
                Ok((key, Err(e))) => {
                    failed += 1;
                    results.push(BatchItemResult {
                        key,
                        result: Err(e.to_string()),
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(BatchItemResult {
                        key: "<unknown>".to_string(),
                        result: Err(format!("Task join error: {}", e)),
                    });
                }
            }
        }

        Ok(BatchResult {
            total: object_keys.len(),
            succeeded,
            failed,
            results,
        })
    }

    /// Batch rename objects within a bucket (copy to new key, then delete old key).
    /// Uses a semaphore to limit concurrency to 10 concurrent requests.
    pub async fn batch_rename_objects(
        &self,
        bucket: &str,
        renames: &[(String, String)],
    ) -> Result<BatchResult<ObjectDetails>> {
        let semaphore = Arc::new(Semaphore::new(10));
        let mut join_set = tokio::task::JoinSet::new();

        for (old_key, new_key) in renames {
            let client = self.clone();
            let bucket = bucket.to_string();
            let old = old_key.clone();
            let new = new_key.clone();
            let sem = semaphore.clone();

            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed unexpectedly");
                // Copy to new key
                let copy_result = client.copy_object(&bucket, &old, &bucket, Some(&new)).await;
                match copy_result {
                    Ok(details) => {
                        // Delete old key on successful copy
                        if let Err(e) = client.delete_object(&bucket, &old).await {
                            // Copy succeeded but delete failed — partial success
                            return (
                                old,
                                Err(anyhow::anyhow!(
                                    "Copied to '{}' but failed to delete original: {}",
                                    new,
                                    e
                                )),
                            );
                        }
                        (old, Ok(details))
                    }
                    Err(e) => (old, Err(e)),
                }
            });
        }

        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok((key, Ok(details))) => {
                    succeeded += 1;
                    results.push(BatchItemResult {
                        key,
                        result: Ok(details),
                    });
                }
                Ok((key, Err(e))) => {
                    failed += 1;
                    results.push(BatchItemResult {
                        key,
                        result: Err(e.to_string()),
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(BatchItemResult {
                        key: "<unknown>".to_string(),
                        result: Err(format!("Task join error: {}", e)),
                    });
                }
            }
        }

        Ok(BatchResult {
            total: renames.len(),
            succeeded,
            failed,
            results,
        })
    }
}
