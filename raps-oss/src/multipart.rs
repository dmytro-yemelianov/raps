// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Multipart upload operations for the OSS API.

use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};
use tokio::sync::Semaphore;

use raps_kernel::progress;

use crate::OssClient;
use crate::types::*;

impl OssClient {
    /// Create a fresh multipart upload state with signed URLs
    #[allow(clippy::too_many_arguments)]
    async fn start_fresh_upload(
        &self,
        bucket_key: &str,
        object_key: &str,
        file_path: &Path,
        total_parts: u32,
        file_size: u64,
        chunk_size: u64,
        file_mtime: i64,
    ) -> Result<(MultipartUploadState, Option<Vec<String>>)> {
        let signed = self
            .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
            .await?;
        if signed.urls.len() != total_parts as usize {
            anyhow::bail!(
                "Expected {} URLs but got {}",
                total_parts,
                signed.urls.len()
            );
        }
        let new_state = MultipartUploadState {
            bucket_key: bucket_key.to_string(),
            object_key: object_key.to_string(),
            file_path: file_path.to_string_lossy().to_string(),
            file_size,
            chunk_size,
            total_parts,
            completed_parts: Vec::new(),
            part_etags: std::collections::HashMap::new(),
            upload_key: signed.upload_key,
            started_at: chrono::Utc::now().timestamp(),
            file_mtime,
        };
        new_state.save()?;
        Ok((new_state, Some(signed.urls)))
    }

    /// Upload a large file using multipart upload with resume capability
    pub async fn upload_multipart(
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
        let file_mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let chunk_size = MultipartUploadState::DEFAULT_CHUNK_SIZE;
        let total_parts = file_size.div_ceil(chunk_size) as u32;

        let (state, initial_urls) = if resume {
            if let Some(existing_state) = MultipartUploadState::load(bucket_key, object_key)? {
                if existing_state.can_resume(file_path) {
                    tracing::info!(
                        "Resuming upload: {}/{} completed parts",
                        existing_state.completed_parts.len(),
                        existing_state.total_parts
                    );
                    (existing_state, None)
                } else {
                    tracing::info!("File changed since last upload, starting fresh");
                    MultipartUploadState::delete(bucket_key, object_key)?;
                    self.start_fresh_upload(
                        bucket_key,
                        object_key,
                        file_path,
                        total_parts,
                        file_size,
                        chunk_size,
                        file_mtime,
                    )
                    .await?
                }
            } else {
                self.start_fresh_upload(
                    bucket_key,
                    object_key,
                    file_path,
                    total_parts,
                    file_size,
                    chunk_size,
                    file_mtime,
                )
                .await?
            }
        } else {
            MultipartUploadState::delete(bucket_key, object_key)?;
            self.start_fresh_upload(
                bucket_key,
                object_key,
                file_path,
                total_parts,
                file_size,
                chunk_size,
                file_mtime,
            )
            .await?
        };

        // Create progress bar (hidden in non-interactive mode)
        let pb = progress::file_progress(file_size, &format!("Uploading {}", object_key));

        // Update progress if resuming
        if !state.completed_parts.is_empty() {
            let completed_bytes: u64 = state
                .completed_parts
                .iter()
                .map(|&part| {
                    let start = (part as u64 - 1) * state.chunk_size;
                    let end = std::cmp::min(start + state.chunk_size, state.file_size);
                    end - start
                })
                .sum();
            pb.set_position(completed_bytes);
            pb.set_message(format!(
                "Resuming {} ({} parts done)",
                object_key,
                state.completed_parts.len()
            ));
        } else {
            pb.set_message(format!("Starting multipart upload for {}", object_key));
        }

        // Get remaining parts to upload
        let remaining_parts = state.remaining_parts();

        if remaining_parts.is_empty() {
            pb.set_message(format!("All parts uploaded, completing {}", object_key));
        } else {
            pb.set_message(format!(
                "Uploading {} ({} parts remaining)",
                object_key,
                remaining_parts.len()
            ));
        }

        let urls = if let Some(u) = initial_urls {
            u
        } else {
            let signed = self
                .get_signed_upload_url(bucket_key, object_key, Some(total_parts), None)
                .await?;
            signed.urls
        };

        // Upload remaining parts in parallel with bounded concurrency
        use futures_util::stream::FuturesUnordered;
        use tokio::sync::Mutex;

        const MAX_CONCURRENT_UPLOADS: usize = 5;
        /// Save state to disk every N completed parts instead of every part,
        /// reducing I/O overhead for large files with many chunks.
        const STATE_FLUSH_INTERVAL: usize = 5;

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOADS));
        let upload_key = state.upload_key.clone();
        let parts_since_flush = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let state_mutex = Arc::new(Mutex::new(state));
        let pb_arc = Arc::new(Mutex::new(pb));
        let file_path_clone = file_path.to_path_buf();

        // Buffer pool: pre-allocate buffers to avoid per-chunk allocations.
        // Each concurrent upload task takes a buffer from the pool and returns it when done.
        let (buf_tx, buf_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(MAX_CONCURRENT_UPLOADS);
        for _ in 0..MAX_CONCURRENT_UPLOADS {
            buf_tx
                .send(vec![0u8; chunk_size as usize])
                .await
                .expect("buffer pool channel closed unexpectedly");
        }
        let buf_rx = Arc::new(Mutex::new(buf_rx));

        // Create upload tasks
        let upload_tasks: FuturesUnordered<_> = remaining_parts
            .into_iter()
            .map(|part_num| {
                let part_index = (part_num - 1) as usize;
                let start = (part_num as u64 - 1) * chunk_size;
                let end = std::cmp::min(start + chunk_size, file_size);
                let part_size = end - start;
                let s3_url = urls[part_index].clone();
                let client = self.http_client.clone();
                let semaphore = semaphore.clone();
                let state_mutex = state_mutex.clone();
                let parts_since_flush = parts_since_flush.clone();
                let pb_arc = pb_arc.clone();
                let buf_rx = buf_rx.clone();
                let buf_tx = buf_tx.clone();
                let object_key = object_key.to_string();
                let file_path = file_path_clone.clone();

                async move {
                    // Acquire semaphore permit to limit concurrency
                    let _permit = semaphore
                        .acquire()
                        .await
                        .map_err(|_| anyhow::anyhow!("Upload cancelled"))?;

                    // Take a buffer from the pool (reused across chunks)
                    let mut buffer = {
                        let mut rx = buf_rx.lock().await;
                        rx.recv()
                            .await
                            .ok_or_else(|| anyhow::anyhow!("Buffer pool exhausted"))?
                    };

                    // Read file chunk into pooled buffer
                    {
                        let mut file =
                            tokio::fs::File::open(&file_path).await.with_context(|| {
                                format!("Failed to open file for part {}", part_num)
                            })?;
                        file.seek(SeekFrom::Start(start)).await?;
                        buffer.resize(part_size as usize, 0);
                        file.read_exact(&mut buffer).await?;
                    }

                    // Upload part with retry logic
                    let mut attempts = 0;
                    const MAX_RETRIES: usize = 3;
                    let mut total_part_network_time = std::time::Duration::ZERO;

                    loop {
                        attempts += 1;

                        let _part_start = std::time::Instant::now();
                        let response = client
                            .put(&s3_url)
                            .header("Content-Type", "application/octet-stream")
                            .header("Content-Length", part_size.to_string())
                            .body(buffer.clone())
                            .send()
                            .await;
                        total_part_network_time += _part_start.elapsed();

                        match response {
                            Ok(resp) if resp.status().is_success() => {
                                // Get ETag from response
                                let etag = resp
                                    .headers()
                                    .get("etag")
                                    .and_then(|v| v.to_str().ok())
                                    .map(|s| s.trim_matches('"').to_string())
                                    .unwrap_or_default();

                                // Update state atomically, flush to disk periodically
                                {
                                    let mut state_guard = state_mutex.lock().await;
                                    state_guard.completed_parts.push(part_num);
                                    state_guard.part_etags.insert(part_num, etag);
                                    let count = parts_since_flush
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                        + 1;
                                    if count >= STATE_FLUSH_INTERVAL {
                                        parts_since_flush
                                            .store(0, std::sync::atomic::Ordering::Relaxed);
                                        if let Err(e) = state_guard.save() {
                                            tracing::warn!(error = %e, "Failed to save upload state");
                                        }
                                    }
                                }

                                // Update progress bar
                                {
                                    let pb_guard = pb_arc.lock().await;
                                    pb_guard.set_position(end);
                                    pb_guard.set_message(format!(
                                        "Uploading {} ({} parts completed)",
                                        object_key, part_num
                                    ));
                                }

                                raps_kernel::profiler::record_http_request(total_part_network_time);
                                // Return buffer to pool for reuse
                                let _ = buf_tx.send(buffer).await;
                                return Ok::<_, anyhow::Error>(part_num);
                            }
                            Ok(resp) => {
                                let status = resp.status();
                                let error_text = resp.text().await.unwrap_or_default();
                                if attempts >= MAX_RETRIES {
                                    raps_kernel::profiler::record_http_request(
                                        total_part_network_time,
                                    );
                                    anyhow::bail!(
                                        "Failed to upload part {} after {} attempts ({}): {}",
                                        part_num,
                                        attempts,
                                        status,
                                        error_text
                                    );
                                }
                                raps_kernel::profiler::record_http_retry();
                                // Wait before retry with exponential backoff
                                let delay =
                                    std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                                tokio::time::sleep(delay).await;
                            }
                            Err(e) => {
                                if attempts >= MAX_RETRIES {
                                    raps_kernel::profiler::record_http_request(
                                        total_part_network_time,
                                    );
                                    anyhow::bail!(
                                        "Failed to upload part {} after {} attempts: {}",
                                        part_num,
                                        attempts,
                                        e
                                    );
                                }
                                raps_kernel::profiler::record_http_retry();
                                // Wait before retry
                                let delay =
                                    std::time::Duration::from_millis(100 * (1 << (attempts - 1)));
                                tokio::time::sleep(delay).await;
                            }
                        }
                    }
                }
            })
            .collect();

        // Execute all upload tasks concurrently
        let mut upload_results = Vec::new();
        let mut upload_stream = upload_tasks;

        while let Some(result) = upload_stream.next().await {
            match result {
                Ok(part_num) => {
                    upload_results.push(part_num);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        // Final state flush — ensure all completed parts are persisted before completion
        {
            let state_guard = state_mutex.lock().await;
            if let Err(e) = state_guard.save() {
                tracing::warn!(error = %e, "Failed to save final upload state");
            }
        }

        // Get the progress bar back from the Arc<Mutex<>>
        let pb = match Arc::try_unwrap(pb_arc) {
            Ok(mutex) => mutex.into_inner(),
            Err(arc) => arc.lock().await.clone(),
        };

        // Complete the upload
        pb.set_message(format!("Completing upload for {}", object_key));
        let object_info = self
            .complete_signed_upload(bucket_key, object_key, &upload_key)
            .await?;

        // Clean up state file
        MultipartUploadState::delete(bucket_key, object_key)?;

        pb.finish_with_message(format!("Uploaded {} (multipart)", object_key));

        Ok(object_info)
    }
}
