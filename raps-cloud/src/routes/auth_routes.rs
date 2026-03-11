// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Auth routes (signup + login)

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;

use crate::{auth, db, error::ApiError, response::ApiResponse, AppState};

#[derive(Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub password: String,
    pub org_name: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(serde::Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
}

pub async fn signup(
    State(state): State<AppState>,
    Json(req): Json<SignupRequest>,
) -> Result<(StatusCode, Json<ApiResponse<AuthResponse>>), ApiError> {
    // Create slug from org name
    let slug = req.org_name.to_lowercase().replace(' ', "-");

    // Create tenant
    let tenant = db::tenants::create(&state.db, &req.org_name, &slug)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") || e.to_string().contains("23505") {
                ApiError::Conflict("Organization already exists".to_string())
            } else {
                ApiError::Internal(e)
            }
        })?;

    // Hash password and create user
    let password_hash = auth::hash_password(&req.password)
        .map_err(|e| ApiError::Internal(e))?;
    let user = db::users::create(&state.db, tenant.id, &req.email, &password_hash, None, "owner")
        .await
        .map_err(|e| ApiError::Internal(e))?;

    // Generate JWT
    let claims = auth::Claims {
        sub: user.id,
        tenant_id: tenant.id,
        role: user.role.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_expiry_seconds as i64)).timestamp() as usize,
    };
    let token = auth::encode_jwt(&state.config.jwt_secret, &claims)
        .map_err(|e| ApiError::Internal(e))?;

    Ok((StatusCode::CREATED, ApiResponse::ok(AuthResponse {
        token,
        user_id: user.id,
        tenant_id: tenant.id,
    })))
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<AuthResponse>>, ApiError> {
    let user = db::users::get_by_email(&state.db, &req.email)
        .await
        .map_err(|e| ApiError::Internal(e))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    let valid = auth::verify_password(&req.password, &user.password_hash)
        .map_err(|e| ApiError::Internal(e))?;
    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let claims = auth::Claims {
        sub: user.id,
        tenant_id: user.tenant_id,
        role: user.role.clone(),
        exp: (chrono::Utc::now() + chrono::Duration::seconds(state.config.jwt_expiry_seconds as i64)).timestamp() as usize,
    };
    let token = auth::encode_jwt(&state.config.jwt_secret, &claims)
        .map_err(|e| ApiError::Internal(e))?;

    Ok(ApiResponse::ok(AuthResponse {
        token,
        user_id: user.id,
        tenant_id: user.tenant_id,
    }))
}
