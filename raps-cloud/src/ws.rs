// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! WebSocket handler for real-time job progress streaming.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Query, State, WebSocketUpgrade,
    },
    response::Response,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{auth, AppState};

/// Progress update broadcast to WebSocket clients.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressEvent {
    pub job_id: Uuid,
    pub tenant_id: Uuid,
    pub status: String,
    pub completed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub total: usize,
    pub current_item: Option<String>,
}

/// Query params for WebSocket auth.
#[derive(Deserialize)]
pub struct WsQuery {
    pub token: String,
}

/// Broadcast channel sender — stored in AppState.
pub type ProgressTx = broadcast::Sender<JobProgressEvent>;

pub fn new_progress_channel() -> (ProgressTx, broadcast::Receiver<JobProgressEvent>) {
    broadcast::channel(256)
}

/// WebSocket upgrade handler.
pub async fn ws_handler(
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, crate::error::ApiError> {
    // Authenticate via query param token
    let claims = auth::decode_jwt(&state.config.jwt_secret, &query.token)
        .map_err(|_| crate::error::ApiError::Unauthorized("Invalid or expired token".into()))?;

    let tenant_id = claims.tenant_id;
    let rx = state.progress_tx.subscribe();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, rx, tenant_id)))
}

async fn handle_socket(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<JobProgressEvent>,
    tenant_id: Uuid,
) {
    loop {
        tokio::select! {
            // Forward matching progress events to the client
            result = rx.recv() => {
                match result {
                    Ok(event) if event.tenant_id == tenant_id => {
                        let json = serde_json::to_string(&event).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break; // Client disconnected
                        }
                    }
                    Ok(_) => {} // Different tenant, skip
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged by {n} messages");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            // Handle incoming messages (ping/pong/close)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
