// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub plan_tier: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create(pool: &PgPool, name: &str, slug: &str) -> anyhow::Result<Tenant> {
    let tenant =
        sqlx::query_as::<_, Tenant>("INSERT INTO tenants (name, slug) VALUES ($1, $2) RETURNING *")
            .bind(name)
            .bind(slug)
            .fetch_one(pool)
            .await?;
    Ok(tenant)
}

pub async fn get_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<Tenant>> {
    let tenant = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(tenant)
}

pub async fn get_by_slug(pool: &PgPool, slug: &str) -> anyhow::Result<Option<Tenant>> {
    let tenant = sqlx::query_as::<_, Tenant>("SELECT * FROM tenants WHERE slug = $1")
        .bind(slug)
        .fetch_optional(pool)
        .await?;
    Ok(tenant)
}
