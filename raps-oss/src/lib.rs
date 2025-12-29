// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS OSS Service - Object Storage Service client
//!
//! This crate provides OSS operations: buckets, objects, uploads, downloads.

pub mod bucket;
pub mod object;
pub mod upload;
pub mod download;
pub mod signed_url;
pub mod types;

pub use bucket::BucketClient;
pub use object::ObjectClient;
pub use upload::UploadClient;
pub use download::DownloadClient;
pub use signed_url::SignedUrlClient;
pub use types::*;
