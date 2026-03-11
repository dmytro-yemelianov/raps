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

use axum::{middleware as axum_mw, routing::{get, post}, Json, Router};
use config::CloudConfig;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: CloudConfig,
    pub db: sqlx::PgPool,
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

    let state = AppState {
        config: config.clone(),
        db,
    };

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/api/v1/auth/signup", post(routes::auth_routes::signup))
        .route("/api/v1/auth/login", post(routes::auth_routes::login));

    // Protected routes (JWT auth required)
    let protected_routes = Router::new()
        .route("/api/v1/jobs", get(routes::jobs::list_jobs).post(routes::jobs::create_job))
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

    // Spawn background job runner
    let job_state = state.clone();
    tokio::spawn(async move {
        jobs::runner::run_loop(job_state).await;
    });

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await?;
    tracing::info!("raps-cloud listening on port {}", config.port);
    axum::serve(listener, app).await?;
    Ok(())
}
