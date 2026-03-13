// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

mod auth;
mod config;
mod crypto;
mod db;
mod error;
mod jobs;
mod middleware;
mod response;
mod routes;
pub mod ws;

use std::time::Duration;

use axum::{
    Json, Router, middleware as axum_mw,
    routing::{get, post},
};
use config::CloudConfig;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: CloudConfig,
    pub db: sqlx::PgPool,
    pub progress_tx: ws::ProgressTx,
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = CloudConfig::from_env()?;
    let db = sqlx::PgPool::connect(&config.database_url).await?;
    sqlx::migrate!().run(&db).await?;

    let (progress_tx, _rx) = ws::new_progress_channel();

    let state = AppState {
        config: config.clone(),
        db,
        progress_tx,
    };

    // Shutdown channel: send `true` to signal all background tasks to stop.
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Public routes (no auth required; /ws does its own auth via query param)
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws::ws_handler))
        .route("/api/v1/auth/signup", post(routes::auth_routes::signup))
        .route("/api/v1/auth/login", post(routes::auth_routes::login));

    // Protected routes (JWT auth required)
    let protected_routes = Router::new()
        .route(
            "/api/v1/jobs",
            get(routes::jobs::list_jobs).post(routes::jobs::create_job),
        )
        .route("/api/v1/jobs/{id}", get(routes::jobs::get_job))
        .route("/api/v1/jobs/{id}/cancel", post(routes::jobs::cancel_job))
        .route("/api/v1/jobs/{id}/retry", post(routes::jobs::retry_job))
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::auth_mw::require_auth,
        ));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Spawn background job runner with shutdown signal
    let job_state = state.clone();
    let runner_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        jobs::runner::run_loop(job_state, runner_shutdown).await;
    });

    // Spawn timeout reaper with shutdown signal
    let reaper_state = state.clone();
    let reaper_shutdown = shutdown_rx.clone();
    tokio::spawn(async move {
        jobs::runner::run_timeout_reaper(reaper_state, reaper_shutdown).await;
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("raps-cloud listening on port {}", config.port);

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            // Wait for SIGTERM or SIGINT (Ctrl-C)
            #[cfg(unix)]
            {
                use tokio::signal::unix::{SignalKind, signal};
                let mut sigterm =
                    signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {}
                    _ = sigterm.recv() => {}
                }
            }
            #[cfg(not(unix))]
            {
                tokio::signal::ctrl_c().await.ok();
            }

            tracing::info!("Shutdown signal received, draining in-flight jobs...");
            let _ = shutdown_tx.send(true);
            // Give workers time to finish their current jobs
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await?;

    Ok(())
}
