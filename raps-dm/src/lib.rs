// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! RAPS Data Management Service - Hubs, projects, folders, items
//!
//! This crate provides Data Management operations: hubs, projects, folders, items.

pub mod hub;
pub mod project;
pub mod folder;
pub mod item;
pub mod types;
pub mod acc;

pub use hub::HubClient;
pub use project::ProjectClient;
pub use folder::FolderClient;
pub use item::ItemClient;
pub use types::*;
pub use acc::{AccClient, Asset, Checklist, Issue, Rfi, Submittal};
