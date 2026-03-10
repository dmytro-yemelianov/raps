// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP (Model Context Protocol) Server module
//!
//! Provides AI assistants with direct access to Autodesk Platform Services
//! through the Model Context Protocol. Enables natural language interaction
//! with APS APIs for buckets, objects, translation, projects, and more.

pub mod auth_guidance;
pub mod definitions;
mod dispatch;
pub mod server;
pub mod tools;
mod tools_acc;
mod tools_admin;
mod tools_compound;
mod tools_dm;
mod tools_misc;
mod tools_oss;
mod tools_skill;
