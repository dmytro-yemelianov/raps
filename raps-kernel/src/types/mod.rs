// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Domain types for RAPS Kernel

pub mod bucket;
pub mod object;
pub mod urn;

pub use bucket::BucketKey;
pub use object::ObjectKey;
pub use urn::Urn;
