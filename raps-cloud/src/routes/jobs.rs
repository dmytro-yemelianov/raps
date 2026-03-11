// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Jobs API routes

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{db, error::ApiError, middleware::auth_mw::AuthUser, response::ApiResponse, AppState};

#[derive(Deserialize)]
pub struct CreateJobRequest {
    pub kind: String,
    pub credential_id: Option<Uuid>,
    pub input: serde_json::Value,
}

#[derive(Deserialize)]
pub struct ListJobsQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
    pub status: Option<String>,
}

pub async fn create_job(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(StatusCode, Json<ApiResponse<db::jobs::Job>>), ApiError> {
    // Validate job kind
    let valid_kinds = [
        "bulk_user_add", "bulk_user_remove", "export_permissions",
        "clone_permissions", "archive_project", "bulk_translate", "pipeline_run",
    ];
    if !valid_kinds.contains(&req.kind.as_str()) {
        return Err(ApiError::BadRequest(format!("Invalid job kind: {}", req.kind)));
    }

    let job = db::jobs::create(
        &state.db,
        auth_user.tenant_id,
        req.credential_id,
        &req.kind,
        req.input,
    )
    .await
    .map_err(|e| ApiError::Internal(e))?;

    tracing::info!(job_id = %job.id, kind = %job.kind, "Job created");

    Ok((StatusCode::ACCEPTED, ApiResponse::ok(job)))
}

pub async fn list_jobs(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<ApiResponse<Vec<db::jobs::Job>>>, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let cursor = query
        .cursor
        .as_deref()
        .and_then(|c| chrono::DateTime::parse_from_rfc3339(c).ok())
        .map(|c| c.with_timezone(&chrono::Utc));

    let jobs = db::jobs::list_by_tenant(&state.db, auth_user.tenant_id, limit + 1, cursor)
        .await
        .map_err(|e| ApiError::Internal(e))?;

    let has_more = jobs.len() as i64 > limit;
    let jobs: Vec<_> = jobs.into_iter().take(limit as usize).collect();
    let next_cursor = jobs.last().map(|j| j.created_at.to_rfc3339());

    Ok(ApiResponse::paginated(jobs, next_cursor, has_more))
}

pub async fn get_job(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<db::jobs::Job>>, ApiError> {
    let job = db::jobs::get_by_id(&state.db, id)
        .await
        .map_err(|e| ApiError::Internal(e))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    if job.tenant_id != auth_user.tenant_id {
        return Err(ApiError::NotFound("Job not found".to_string()));
    }

    Ok(ApiResponse::ok(job))
}

pub async fn cancel_job(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<db::jobs::Job>>, ApiError> {
    let job = db::jobs::get_by_id(&state.db, id)
        .await
        .map_err(|e| ApiError::Internal(e))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    if job.tenant_id != auth_user.tenant_id {
        return Err(ApiError::NotFound("Job not found".to_string()));
    }

    if job.status != "queued" && job.status != "running" {
        return Err(ApiError::BadRequest("Job is not cancellable".to_string()));
    }

    db::jobs::update_status(&state.db, id, "cancelled", None, None)
        .await
        .map_err(|e| ApiError::Internal(e))?;

    let updated = db::jobs::get_by_id(&state.db, id)
        .await
        .map_err(|e| ApiError::Internal(e))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    Ok(ApiResponse::ok(updated))
}

pub async fn retry_job(
    State(state): State<AppState>,
    Extension(auth_user): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<(StatusCode, Json<ApiResponse<db::jobs::Job>>), ApiError> {
    let job = db::jobs::get_by_id(&state.db, id)
        .await
        .map_err(|e| ApiError::Internal(e))?
        .ok_or_else(|| ApiError::NotFound("Job not found".to_string()))?;

    if job.tenant_id != auth_user.tenant_id {
        return Err(ApiError::NotFound("Job not found".to_string()));
    }

    if job.status != "failed" && job.status != "cancelled" {
        return Err(ApiError::BadRequest("Only failed or cancelled jobs can be retried".to_string()));
    }

    // Create a new job with the same parameters
    let new_job = db::jobs::create(
        &state.db,
        auth_user.tenant_id,
        job.credential_id,
        &job.kind,
        job.input,
    )
    .await
    .map_err(|e| ApiError::Internal(e))?;

    tracing::info!(job_id = %new_job.id, original_id = %id, "Job retried");

    Ok((StatusCode::ACCEPTED, ApiResponse::ok(new_job)))
}
