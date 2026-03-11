// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{auth, error::ApiError, AppState};

/// Authenticated user info attached to request extensions.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
}

pub async fn require_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    let claims = auth::decode_jwt(&state.config.jwt_secret, token)
        .map_err(|_| ApiError::Unauthorized("Invalid or expired token".to_string()))?;

    let auth_user = AuthUser {
        user_id: claims.sub,
        tenant_id: claims.tenant_id,
        role: claims.role,
    };

    request.extensions_mut().insert(auth_user);
    Ok(next.run(request).await)
}
