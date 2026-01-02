// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Secure credential storage abstraction

pub mod keyring;
pub mod file;
pub mod token;

pub use keyring::StorageBackend;
pub use token::TokenStorage;
