// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Benchmarks for upload operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use raps_oss::upload::{MultipartUploadState, UploadConfig};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn bench_upload_config_default(c: &mut Criterion) {
    c.bench_function("upload_config_default", |b| {
        b.iter(|| {
            black_box(UploadConfig::default());
        });
    });
}

fn bench_upload_config_custom(c: &mut Criterion) {
    c.bench_function("upload_config_custom", |b| {
        b.iter(|| {
            black_box(UploadConfig {
                concurrency: 10,
                chunk_size: 10 * 1024 * 1024,
                resume: true,
            });
        });
    });
}

fn bench_multipart_state_remaining_parts(c: &mut Criterion) {
    let mut group = c.benchmark_group("multipart_state_remaining_parts");

    for total_parts in [5, 10, 20, 50, 100].iter() {
        let mut state = MultipartUploadState {
            bucket_key: "test-bucket".to_string(),
            object_key: "test-object".to_string(),
            file_path: "/tmp/test.bin".to_string(),
            file_size: *total_parts as u64 * 5 * 1024 * 1024,
            chunk_size: 5 * 1024 * 1024,
            total_parts: *total_parts,
            completed_parts: (1..=*total_parts / 2).collect(), // Half completed
            part_etags: HashMap::new(),
            upload_key: "test-upload-key".to_string(),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            file_mtime: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        group.bench_with_input(
            BenchmarkId::from_parameter(total_parts),
            total_parts,
            |b, _| {
                b.iter(|| {
                    black_box(state.remaining_parts());
                });
            },
        );
    }

    group.finish();
}

fn bench_multipart_state_serialization(c: &mut Criterion) {
    let state = MultipartUploadState {
        bucket_key: "test-bucket".to_string(),
        object_key: "test-object".to_string(),
        file_path: "/tmp/test.bin".to_string(),
        file_size: 100 * 1024 * 1024,
        chunk_size: 5 * 1024 * 1024,
        total_parts: 20,
        completed_parts: (1..=10).collect(),
        part_etags: {
            let mut map = HashMap::new();
            for i in 1..=10 {
                map.insert(i, format!("etag-{}", i));
            }
            map
        },
        upload_key: "test-upload-key".to_string(),
        started_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        file_mtime: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    };

    c.bench_function("multipart_state_serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&state).unwrap());
        });
    });

    let json = serde_json::to_string(&state).unwrap();
    c.bench_function("multipart_state_deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<MultipartUploadState>(&json).unwrap());
        });
    });
}

fn bench_chunk_size_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_size_calculation");

    for file_size_mb in [10, 50, 100, 500, 1000].iter() {
        let file_size = *file_size_mb * 1024 * 1024;
        let chunk_size = MultipartUploadState::DEFAULT_CHUNK_SIZE;

        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}MB", file_size_mb)),
            &file_size,
            |b, size| {
                b.iter(|| {
                    let total_parts = (*size as f64 / chunk_size as f64).ceil() as u32;
                    black_box(total_parts);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_upload_config_default,
    bench_upload_config_custom,
    bench_multipart_state_remaining_parts,
    bench_multipart_state_serialization,
    bench_chunk_size_calculation
);
criterion_main!(benches);
