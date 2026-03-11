// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Job {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub credential_id: Option<Uuid>,
    pub kind: String,
    pub status: String,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub created_at: DateTime<Utc>,
}

pub async fn create(
    pool: &PgPool,
    tenant_id: Uuid,
    credential_id: Option<Uuid>,
    kind: &str,
    input: serde_json::Value,
) -> anyhow::Result<Job> {
    let job = sqlx::query_as::<_, Job>(
        "INSERT INTO jobs (tenant_id, credential_id, kind, input)
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(tenant_id)
    .bind(credential_id)
    .bind(kind)
    .bind(input)
    .fetch_one(pool)
    .await?;
    Ok(job)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Job>> {
    let job = sqlx::query_as::<_, Job>("SELECT * FROM jobs WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(job)
}

pub async fn list_by_tenant(
    pool: &PgPool,
    tenant_id: Uuid,
    limit: i64,
    cursor: Option<DateTime<Utc>>,
) -> anyhow::Result<Vec<Job>> {
    let jobs = if let Some(cursor) = cursor {
        sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE tenant_id = $1 AND created_at < $2
             ORDER BY created_at DESC LIMIT $3",
        )
        .bind(tenant_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, Job>(
            "SELECT * FROM jobs WHERE tenant_id = $1
             ORDER BY created_at DESC LIMIT $2",
        )
        .bind(tenant_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(jobs)
}

pub async fn update_status(
    pool: &PgPool,
    id: Uuid,
    status: &str,
    output: Option<serde_json::Value>,
    error: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE jobs SET status = $1, output = $2, error = $3,
         completed_at = CASE WHEN $1 IN ('completed', 'failed', 'cancelled') THEN $4 ELSE completed_at END,
         started_at = CASE WHEN $1 = 'running' AND started_at IS NULL THEN $4 ELSE started_at END,
         duration_ms = CASE WHEN $1 IN ('completed', 'failed') THEN
           EXTRACT(EPOCH FROM ($4 - started_at))::bigint * 1000 ELSE duration_ms END
         WHERE id = $5",
    )
    .bind(status)
    .bind(output)
    .bind(error)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn claim_next(pool: &PgPool) -> anyhow::Result<Option<Job>> {
    let job = sqlx::query_as::<_, Job>(
        "UPDATE jobs SET status = 'running', started_at = now()
         WHERE id = (SELECT id FROM jobs WHERE status = 'queued' ORDER BY created_at LIMIT 1 FOR UPDATE SKIP LOCKED)
         RETURNING *",
    )
    .fetch_optional(pool)
    .await?;
    Ok(job)
}
