// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Performance benchmarks for monolith implementation
//!
//! This benchmark suite mirrors the kernel benchmarks to enable comparison.

#![allow(unsafe_code)] // Benchmarks need unsafe for env var manipulation

#![allow(unsafe_code)] // Benchmarks need unsafe for env var manipulation

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use raps::{Config, HttpClientConfig, AuthClient};
use std::time::Duration;

/// Benchmark: Config loading from environment
fn bench_config_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_loading");
    group.measurement_time(Duration::from_secs(5));
    
    // Save original env vars
    let orig_client_id = std::env::var("APS_CLIENT_ID").ok();
    let orig_client_secret = std::env::var("APS_CLIENT_SECRET").ok();
    
    // Set test env vars
    unsafe {
        std::env::set_var("APS_CLIENT_ID", "test_client_id");
        std::env::set_var("APS_CLIENT_SECRET", "test_secret");
    }
    
    group.bench_function("monolith_config_from_env", |b| {
        b.iter(|| {
            black_box(Config::from_env()).ok()
        });
    });
    
    // Restore original env vars
    unsafe {
        if let Some(val) = orig_client_id {
            std::env::set_var("APS_CLIENT_ID", val);
        } else {
            std::env::remove_var("APS_CLIENT_ID");
        }
        if let Some(val) = orig_client_secret {
            std::env::set_var("APS_CLIENT_SECRET", val);
        } else {
            std::env::remove_var("APS_CLIENT_SECRET");
        }
    }
    
    group.finish();
}

/// Benchmark: HTTP client creation
fn bench_http_client_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_client");
    
    group.bench_function("monolith_http_client_create", |b| {
        b.iter(|| {
            let config = HttpClientConfig::default();
            black_box(config.create_client()).ok()
        });
    });
    
    group.finish();
}

/// Benchmark: Error creation and conversion
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    group.bench_function("monolith_anyhow_error", |b| {
        b.iter(|| {
            let err = anyhow::anyhow!("Test error message");
            black_box(err.to_string())
        });
    });
    
    group.finish();
}

/// Benchmark: Auth client creation (without actual token fetch)
fn bench_auth_client_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth_client");
    
    // Set test env vars
    let orig_client_id = std::env::var("APS_CLIENT_ID").ok();
    let orig_client_secret = std::env::var("APS_CLIENT_SECRET").ok();
    
    unsafe {
        std::env::set_var("APS_CLIENT_ID", "test_client_id");
        std::env::set_var("APS_CLIENT_SECRET", "test_secret");
    }
    
    group.bench_function("monolith_auth_client_new", |b| {
        b.iter(|| {
            let config = Config::from_env().unwrap();
            let auth = AuthClient::new(black_box(config));
            black_box(auth)
        });
    });
    
    // Restore
    unsafe {
        if let Some(val) = orig_client_id {
            std::env::set_var("APS_CLIENT_ID", val);
        } else {
            std::env::remove_var("APS_CLIENT_ID");
        }
        if let Some(val) = orig_client_secret {
            std::env::set_var("APS_CLIENT_SECRET", val);
        } else {
            std::env::remove_var("APS_CLIENT_SECRET");
        }
    }
    
    group.finish();
}

/// Benchmark: URL construction (manual string formatting)
fn bench_url_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_helpers");
    
    let base_url = "https://developer.api.autodesk.com";
    
    group.bench_function("monolith_auth_url", |b| {
        b.iter(|| {
            let url = format!("{}/authentication/v2/token", base_url);
            black_box(url)
        });
    });
    
    group.bench_function("monolith_oss_url", |b| {
        b.iter(|| {
            let url = format!("{}/oss/v2", base_url);
            black_box(url)
        });
    });
    
    group.bench_function("monolith_derivative_url", |b| {
        b.iter(|| {
            let url = format!("{}/modelderivative/v2", base_url);
            black_box(url)
        });
    });
    
    group.bench_function("monolith_project_url", |b| {
        b.iter(|| {
            let url = format!("{}/project/v1", base_url);
            black_box(url)
        });
    });
    
    group.bench_function("monolith_data_url", |b| {
        b.iter(|| {
            let url = format!("{}/data/v1", base_url);
            black_box(url)
        });
    });
    
    group.finish();
}

/// Benchmark: Memory footprint comparison
fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_footprint");
    
    group.bench_function("monolith_http_client_size", |b| {
        b.iter(|| {
            let config = HttpClientConfig::default();
            let client = config.create_client().unwrap();
            black_box(std::mem::size_of_val(&client))
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_config_loading,
    bench_http_client_creation,
    bench_error_handling,
    bench_auth_client_creation,
    bench_url_construction,
    bench_memory_footprint
);
criterion_main!(benches);
