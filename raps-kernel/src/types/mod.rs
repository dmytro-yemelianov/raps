// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Domain types for RAPS Kernel

pub mod urn;
pub mod bucket;
pub mod object;

pub use urn::Urn;
pub use bucket::BucketKey;
pub use object::ObjectKey;
