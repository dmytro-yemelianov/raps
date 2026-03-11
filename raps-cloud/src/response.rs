// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ResponseMeta>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pagination: Option<Pagination>,
}

#[derive(Serialize)]
pub struct ResponseMeta {
    pub request_id: String,
}

#[derive(Serialize)]
pub struct Pagination {
    pub cursor: Option<String>,
    pub has_more: bool,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Json<Self> {
        Json(Self {
            data,
            meta: None,
            pagination: None,
        })
    }

    pub fn paginated(data: T, cursor: Option<String>, has_more: bool) -> Json<Self> {
        Json(Self {
            data,
            meta: None,
            pagination: Some(Pagination { cursor, has_more }),
        })
    }
}
