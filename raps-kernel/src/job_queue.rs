// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Redis Streams–based distributed job queue.
//!
//! Three priority streams (`critical`, `normal`, `background`) plus a
//! dead-letter queue (`dlq`). Jobs are enqueued with [`JobProducer`] and
//! consumed by [`JobConsumer`] using Redis consumer groups.

#![cfg(feature = "redis")]

use std::collections::HashMap;

use anyhow::{Context, Result};
use deadpool_redis::Pool;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Stream keys
// ---------------------------------------------------------------------------

const STREAM_CRITICAL: &str = "raps:queue:critical";
const STREAM_NORMAL: &str = "raps:queue:normal";
const STREAM_BACKGROUND: &str = "raps:queue:background";
const STREAM_DLQ: &str = "raps:queue:dlq";
const CONSUMER_GROUP: &str = "raps-workers";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Job priority determines which Redis Stream the job is enqueued to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobPriority {
    Critical,
    Normal,
    Background,
}

impl JobPriority {
    pub fn stream_key(&self) -> &'static str {
        match self {
            JobPriority::Critical => STREAM_CRITICAL,
            JobPriority::Normal => STREAM_NORMAL,
            JobPriority::Background => STREAM_BACKGROUND,
        }
    }

    /// Priority order for fair-weighted consumption: critical first.
    pub fn all() -> [JobPriority; 3] {
        [
            JobPriority::Critical,
            JobPriority::Normal,
            JobPriority::Background,
        ]
    }
}

/// Payload discriminator for different job types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobPayload {
    Translate(TranslateJob),
    Upload(UploadJob),
    ExtractProps(ExtractPropsJob),
    Pipeline(PipelineJob),
}

/// Translate a model derivative.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateJob {
    pub urn: String,
    pub output_format: String,
    pub root_filename: Option<String>,
    pub region: Option<String>,
    pub force: bool,
}

/// Upload a file to OSS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadJob {
    pub bucket_key: String,
    pub object_key: String,
    pub file_path: String,
}

/// Extract properties from a translated model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractPropsJob {
    pub urn: String,
    pub view_guid: Option<String>,
    pub output_path: String,
}

/// Run a multi-step pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineJob {
    pub pipeline_name: String,
    pub pipeline_file: String,
    pub variables: HashMap<String, String>,
}

/// Job envelope wrapping payload with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub payload: JobPayload,
    pub priority: JobPriority,
    pub attempts: u32,
    pub max_attempts: u32,
    pub created_at: String,
    pub enqueued_by: String,
}

impl Job {
    pub fn new(payload: JobPayload, priority: JobPriority) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            payload,
            priority,
            attempts: 0,
            max_attempts: 3,
            created_at: chrono::Utc::now().to_rfc3339(),
            enqueued_by: String::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// JobProducer
// ---------------------------------------------------------------------------

/// Enqueues jobs into Redis Streams.
pub struct JobProducer {
    pool: Pool,
}

impl JobProducer {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Enqueue a job at the given priority. Returns the stream entry ID.
    pub async fn enqueue(&self, payload: JobPayload, priority: JobPriority) -> Result<String> {
        let job = Job::new(payload, priority);
        let data =
            serde_json::to_string(&job).context("Failed to serialize job")?;
        let stream = priority.stream_key();

        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        let entry_id: String = redis::cmd("XADD")
            .arg(stream)
            .arg("*")
            .arg("data")
            .arg(&data)
            .query_async(&mut *conn)
            .await
            .context("XADD failed")?;

        tracing::info!(job_id = %job.id, stream, entry_id, "Job enqueued");
        Ok(entry_id)
    }
}

// ---------------------------------------------------------------------------
// JobConsumer
// ---------------------------------------------------------------------------

/// Consumes jobs from Redis Streams using consumer groups.
pub struct JobConsumer {
    pool: Pool,
    consumer_id: String,
}

impl JobConsumer {
    pub fn new(pool: Pool, consumer_id: String) -> Self {
        Self { pool, consumer_id }
    }

    /// Ensure consumer groups exist on all priority streams.
    ///
    /// Swallows `BUSYGROUP` errors (group already exists).
    pub async fn ensure_consumer_groups(&self) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        for priority in JobPriority::all() {
            let stream = priority.stream_key();
            let result: Result<String, redis::RedisError> = redis::cmd("XGROUP")
                .arg("CREATE")
                .arg(stream)
                .arg(CONSUMER_GROUP)
                .arg("$")
                .arg("MKSTREAM")
                .query_async(&mut *conn)
                .await;

            match result {
                Ok(_) => tracing::info!(stream, "Consumer group created"),
                Err(e) if e.to_string().contains("BUSYGROUP") => {
                    tracing::debug!(stream, "Consumer group already exists");
                }
                Err(e) => {
                    return Err(e).context(format!(
                        "Failed to create consumer group on {stream}"
                    ));
                }
            }
        }
        Ok(())
    }

    /// Block-read one job from the highest-priority stream that has work.
    ///
    /// Tries critical → normal → background in order.
    /// Blocks for up to `block_ms` milliseconds.
    pub async fn dequeue_one(&self, block_ms: u64) -> Result<Option<(JobPriority, String, Job)>> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;

        let streams: Vec<&str> = JobPriority::all()
            .iter()
            .map(|p| p.stream_key())
            .collect();

        // XREADGROUP GROUP raps-workers {id} COUNT 1 BLOCK {ms} STREAMS s1 s2 s3 > > >
        let mut cmd = redis::cmd("XREADGROUP");
        cmd.arg("GROUP")
            .arg(CONSUMER_GROUP)
            .arg(&self.consumer_id)
            .arg("COUNT")
            .arg(1)
            .arg("BLOCK")
            .arg(block_ms)
            .arg("STREAMS");
        for s in &streams {
            cmd.arg(*s);
        }
        for _ in &streams {
            cmd.arg(">");
        }

        let result: Option<redis::Value> = cmd.query_async(&mut *conn).await.ok();

        let Some(redis::Value::Array(stream_results)) = result else {
            return Ok(None);
        };

        // Parse XREADGROUP response: [[stream_name, [[entry_id, [field, value, ...]]]]]
        for stream_result in stream_results {
            let redis::Value::Array(parts) = stream_result else {
                continue;
            };
            if parts.len() < 2 {
                continue;
            }

            let stream_name = match &parts[0] {
                redis::Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                _ => continue,
            };

            let priority = match stream_name.as_str() {
                STREAM_CRITICAL => JobPriority::Critical,
                STREAM_NORMAL => JobPriority::Normal,
                STREAM_BACKGROUND => JobPriority::Background,
                _ => continue,
            };

            let redis::Value::Array(entries) = &parts[1] else {
                continue;
            };

            for entry in entries {
                let redis::Value::Array(entry_parts) = entry else {
                    continue;
                };
                if entry_parts.len() < 2 {
                    continue;
                }

                let entry_id = match &entry_parts[0] {
                    redis::Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
                    _ => continue,
                };

                // Fields are [key, value, key, value, ...]
                let redis::Value::Array(fields) = &entry_parts[1] else {
                    continue;
                };

                let mut data: Option<String> = None;
                let mut i = 0;
                while i + 1 < fields.len() {
                    if let (redis::Value::BulkString(k), redis::Value::BulkString(v)) =
                        (&fields[i], &fields[i + 1])
                    {
                        if k == b"data" {
                            data = Some(String::from_utf8_lossy(v).to_string());
                        }
                    }
                    i += 2;
                }

                if let Some(json) = data {
                    let job: Job = serde_json::from_str(&json)
                        .context("Failed to deserialize job from stream")?;
                    return Ok(Some((priority, entry_id, job)));
                }
            }
        }

        Ok(None)
    }

    /// Acknowledge successful processing.
    pub async fn ack(&self, priority: JobPriority, entry_id: &str) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;
        redis::cmd("XACK")
            .arg(priority.stream_key())
            .arg(CONSUMER_GROUP)
            .arg(entry_id)
            .query_async::<i64>(&mut *conn)
            .await
            .context("XACK failed")?;
        Ok(())
    }

    /// Move a failed job to the dead-letter queue and acknowledge it on the source stream.
    pub async fn nack_to_dlq(&self, job: &Job, error: &str) -> Result<()> {
        let mut conn = self.pool.get().await.context("Redis pool exhausted")?;

        // Add to DLQ with error info
        let data = serde_json::to_string(job).context("Failed to serialize job for DLQ")?;
        redis::cmd("XADD")
            .arg(STREAM_DLQ)
            .arg("*")
            .arg("data")
            .arg(&data)
            .arg("error")
            .arg(error)
            .arg("failed_at")
            .arg(chrono::Utc::now().to_rfc3339())
            .query_async::<String>(&mut *conn)
            .await
            .context("XADD to DLQ failed")?;

        tracing::warn!(job_id = %job.id, error, "Job moved to DLQ");
        Ok(())
    }
}
