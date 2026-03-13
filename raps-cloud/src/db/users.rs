// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    #[serde(skip)]
    pub password_hash: String,
    pub display_name: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create(
    pool: &PgPool,
    tenant_id: Uuid,
    email: &str,
    password_hash: &str,
    display_name: Option<&str>,
    role: &str,
) -> anyhow::Result<User> {
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (tenant_id, email, password_hash, display_name, role)
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(tenant_id)
    .bind(email)
    .bind(password_hash)
    .bind(display_name)
    .bind(role)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn get_by_email(pool: &PgPool, email: &str) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(user)
}

pub async fn list_by_tenant(pool: &PgPool, tenant_id: Uuid) -> anyhow::Result<Vec<User>> {
    let users =
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE tenant_id = $1 ORDER BY created_at")
            .bind(tenant_id)
            .fetch_all(pool)
            .await?;
    Ok(users)
}
