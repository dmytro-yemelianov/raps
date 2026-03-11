// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Background job runner that polls for queued jobs and executes them.

use std::time::Duration;

use crate::AppState;
use sqlx;

/// Run the background job processing loop.
///
/// Polls for queued jobs every `poll_interval` and executes them.
/// Each job type is dispatched to its handler.
/// Shuts down cleanly when `shutdown` receives `true`.
pub async fn run_loop(state: AppState, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    let poll_interval = Duration::from_secs(2);
    tracing::info!("Job runner started");

    loop {
        // Check shutdown flag before doing any work
        if *shutdown.borrow() {
            tracing::info!("Job runner shutting down");
            break;
        }

        match crate::db::jobs::claim_next(&state.db).await {
            Ok(Some(job)) => {
                let state = state.clone();
                tokio::spawn(async move {
                    tracing::info!(job_id = %job.id, kind = %job.kind, "Processing job");
                    let result = execute_job(&state, &job).await;

                    match result {
                        Ok(output) => {
                            // Only update if job is still running (not cancelled)
                            let _ = sqlx::query(
                                "UPDATE jobs SET status = 'completed', output = $1, completed_at = now(),
                                 duration_ms = EXTRACT(EPOCH FROM (now() - started_at))::bigint * 1000
                                 WHERE id = $2 AND status = 'running'",
                            )
                            .bind(output)
                            .bind(job.id)
                            .execute(&state.db)
                            .await;
                            tracing::info!(job_id = %job.id, "Job completed");
                        }
                        Err(e) => {
                            tracing::error!(job_id = %job.id, "Job failed: {e:#}");
                            let _ = sqlx::query(
                                "UPDATE jobs SET status = 'failed', error = $1, completed_at = now(),
                                 duration_ms = EXTRACT(EPOCH FROM (now() - started_at))::bigint * 1000
                                 WHERE id = $2 AND status = 'running'",
                            )
                            .bind(&e.to_string())
                            .bind(job.id)
                            .execute(&state.db)
                            .await;
                        }
                    }
                });
            }
            Ok(None) => {
                // No jobs to process; wait before polling again, but wake early on shutdown
                tokio::select! {
                    _ = shutdown.changed() => {
                        tracing::info!("Job runner shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(poll_interval) => {}
                }
            }
            Err(e) => {
                tracing::error!("Job runner error: {e:#}");
                tokio::select! {
                    _ = shutdown.changed() => {
                        tracing::info!("Job runner shutting down");
                        break;
                    }
                    _ = tokio::time::sleep(poll_interval) => {}
                }
            }
        }
    }
}

/// Periodically scan for jobs that have exceeded their `timeout_seconds` and mark them failed.
///
/// Shuts down cleanly when `shutdown` receives `true`.
pub async fn run_timeout_reaper(state: AppState, mut shutdown: tokio::sync::watch::Receiver<bool>) {
    let reap_interval = Duration::from_secs(30);
    tracing::info!("Timeout reaper started");

    loop {
        tokio::select! {
            _ = shutdown.changed() => {
                tracing::info!("Timeout reaper shutting down");
                break;
            }
            _ = tokio::time::sleep(reap_interval) => {
                match crate::db::jobs::find_timed_out(&state.db).await {
                    Ok(jobs) if !jobs.is_empty() => {
                        for job in &jobs {
                            tracing::warn!(job_id = %job.id, "Job timed out");
                        }
                    }
                    Err(e) => tracing::error!("Timeout reaper error: {e:#}"),
                    _ => {}
                }
            }
        }
    }
}

/// Execute a job based on its kind.
async fn execute_job(
    state: &AppState,
    job: &crate::db::jobs::Job,
) -> anyhow::Result<serde_json::Value> {
    match job.kind.as_str() {
        "bulk_user_add" => super::bulk_user::execute_add(state, job).await,
        "bulk_user_remove" => super::bulk_user::execute_remove(state, job).await,
        "export_permissions" => {
            tracing::info!(job_id = %job.id, "Executing export_permissions");
            Ok(serde_json::json!({"status": "completed", "message": "Not yet implemented"}))
        }
        "clone_permissions" => {
            tracing::info!(job_id = %job.id, "Executing clone_permissions");
            Ok(serde_json::json!({"status": "completed", "message": "Not yet implemented"}))
        }
        "archive_project" => {
            tracing::info!(job_id = %job.id, "Executing archive_project");
            Ok(serde_json::json!({"status": "completed", "message": "Not yet implemented"}))
        }
        "bulk_translate" => {
            tracing::info!(job_id = %job.id, "Executing bulk_translate");
            Ok(serde_json::json!({"status": "completed", "message": "Not yet implemented"}))
        }
        "pipeline_run" => {
            tracing::info!(job_id = %job.id, "Executing pipeline_run");
            Ok(serde_json::json!({"status": "completed", "message": "Not yet implemented"}))
        }
        other => {
            anyhow::bail!("Unknown job kind: {other}");
        }
    }
}
