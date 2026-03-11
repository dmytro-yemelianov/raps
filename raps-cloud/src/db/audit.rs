// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use sqlx::PgPool;
use std::net::IpAddr;
use uuid::Uuid;

pub async fn log(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Option<Uuid>,
    action: &str,
    resource_type: Option<&str>,
    resource_id: Option<&str>,
    ip_address: Option<IpAddr>,
    metadata: Option<serde_json::Value>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (tenant_id, user_id, action, resource_type, resource_id, ip_address, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(tenant_id)
    .bind(user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(ip_address.map(|ip| ip.to_string()))
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}
