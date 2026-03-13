// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Credential {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub label: String,
    pub mode: String,
    pub encrypted_data: Vec<u8>,
    pub nonce: Vec<u8>,
    pub scopes: Vec<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialView {
    pub id: Uuid,
    pub label: String,
    pub mode: String,
    pub scopes: Vec<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Credential> for CredentialView {
    fn from(c: Credential) -> Self {
        Self {
            id: c.id,
            label: c.label,
            mode: c.mode,
            scopes: c.scopes,
            is_default: c.is_default,
            created_at: c.created_at,
        }
    }
}

pub async fn create(
    pool: &PgPool,
    tenant_id: Uuid,
    label: &str,
    mode: &str,
    encrypted_data: &[u8],
    nonce: &[u8],
    scopes: &[String],
    is_default: bool,
) -> anyhow::Result<Credential> {
    if is_default {
        sqlx::query(
            "UPDATE credentials SET is_default = false WHERE tenant_id = $1 AND is_default = true",
        )
        .bind(tenant_id)
        .execute(pool)
        .await?;
    }

    let cred = sqlx::query_as::<_, Credential>(
        "INSERT INTO credentials (tenant_id, label, mode, encrypted_data, nonce, scopes, is_default)
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(tenant_id)
    .bind(label)
    .bind(mode)
    .bind(encrypted_data)
    .bind(nonce)
    .bind(scopes)
    .bind(is_default)
    .fetch_one(pool)
    .await?;
    Ok(cred)
}

pub async fn list_by_tenant(pool: &PgPool, tenant_id: Uuid) -> anyhow::Result<Vec<Credential>> {
    let creds = sqlx::query_as::<_, Credential>(
        "SELECT * FROM credentials WHERE tenant_id = $1 ORDER BY created_at",
    )
    .bind(tenant_id)
    .fetch_all(pool)
    .await?;
    Ok(creds)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Credential>> {
    let cred = sqlx::query_as::<_, Credential>("SELECT * FROM credentials WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(cred)
}

pub async fn get_default(pool: &PgPool, tenant_id: Uuid) -> anyhow::Result<Option<Credential>> {
    let cred = sqlx::query_as::<_, Credential>(
        "SELECT * FROM credentials WHERE tenant_id = $1 AND is_default = true",
    )
    .bind(tenant_id)
    .fetch_optional(pool)
    .await?;
    Ok(cred)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM credentials WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
