// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Performance benchmarks comparing kernel vs monolith implementations
//!
//! This benchmark suite compares the performance of kernel operations
//! against the monolith implementation to validate the microkernel architecture.

#![allow(unsafe_code)] // Benchmarks need unsafe for env var manipulation

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use raps_kernel::{
    Config, HttpClient, HttpClientConfig, AuthClient, 
    types::{BucketKey, ObjectKey, Urn},
    error::RapsError,
};
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
    
    group.bench_function("kernel_config_from_env", |b| {
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

/// Benchmark: Type validation (BucketKey, ObjectKey, Urn)
fn bench_type_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_validation");
    
    let valid_bucket = "test-bucket-123";
    let valid_object = "test/object/file.txt";
    let valid_urn = "dXJuOmFkc2sud2lwcHJvZDpmcy5maWxlOnZmLk1vZGlmeUZpbGUuZHdmP3ZlcnNpb249MQ";
    
    group.bench_function("bucket_key_parse", |b| {
        b.iter(|| {
            black_box(BucketKey::new(black_box(valid_bucket))).ok()
        });
    });
    
    group.bench_function("object_key_parse", |b| {
        b.iter(|| {
            black_box(ObjectKey::new(black_box(valid_object)))
        });
    });
    
    group.bench_function("urn_from_str", |b| {
        b.iter(|| {
            black_box(Urn::from(black_box(valid_urn)))
        });
    });
    
    group.finish();
}

/// Benchmark: HTTP client creation
fn bench_http_client_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_client");
    
    group.bench_function("kernel_http_client_new", |b| {
        b.iter(|| {
            let config = HttpClientConfig::default();
            black_box(HttpClient::new(black_box(config))).ok()
        });
    });
    
    group.finish();
}

/// Benchmark: Error creation and conversion
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    group.bench_function("kernel_error_creation", |b| {
        b.iter(|| {
            let err = RapsError::Internal {
                message: black_box("Test error message".to_string()),
            };
            black_box(err.exit_code());
        });
    });
    
    group.bench_function("kernel_error_from_io", |b| {
        b.iter(|| {
            let io_err = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "File not found"
            );
            let err: RapsError = black_box(io_err.into());
            black_box(err.exit_code());
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
    
    group.bench_function("kernel_auth_client_new", |b| {
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

/// Benchmark: URL helper methods
fn bench_url_helpers(c: &mut Criterion) {
    let mut group = c.benchmark_group("url_helpers");
    
    let config = Config {
        client_id: "test".to_string(),
        client_secret: "test".to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: "http://localhost:8080/callback".to_string(),
        da_nickname: None,
    };
    
    group.bench_function("kernel_auth_url", |b| {
        b.iter(|| {
            black_box(config.auth_url())
        });
    });
    
    group.bench_function("kernel_oss_url", |b| {
        b.iter(|| {
            black_box(config.oss_url())
        });
    });
    
    group.bench_function("kernel_derivative_url", |b| {
        b.iter(|| {
            black_box(config.derivative_url())
        });
    });
    
    group.bench_function("kernel_project_url", |b| {
        b.iter(|| {
            black_box(config.project_url())
        });
    });
    
    group.bench_function("kernel_data_url", |b| {
        b.iter(|| {
            black_box(config.data_url())
        });
    });
    
    group.finish();
}

/// Benchmark: Memory footprint comparison
fn bench_memory_footprint(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_footprint");
    
    // Measure size of key types
    group.bench_function("bucket_key_size", |b| {
        b.iter(|| {
            let key = BucketKey::new("test-bucket").unwrap();
            black_box(std::mem::size_of_val(&key))
        });
    });
    
    group.bench_function("object_key_size", |b| {
        b.iter(|| {
            let key = ObjectKey::new("test/object");
            black_box(std::mem::size_of_val(&key))
        });
    });
    
    group.bench_function("http_client_size", |b| {
        b.iter(|| {
            let config = HttpClientConfig::default();
            let client = HttpClient::new(config).unwrap();
            black_box(std::mem::size_of_val(&client))
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_config_loading,
    bench_type_validation,
    bench_http_client_creation,
    bench_error_handling,
    bench_auth_client_creation,
    bench_url_helpers,
    bench_memory_footprint
);
criterion_main!(benches);
