// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! MCP Tool definitions and utilities
//!
//! This module contains additional tool implementations and helper utilities
//! for the RAPS MCP Server.

// Tool definitions are in server.rs.
// This module is reserved for additional utilities and extended tool implementations.

/// Available MCP tools in the RAPS server
#[allow(dead_code)]
pub const TOOLS: &[&str] = &[
    "auth_test",
    "auth_status",
    "bucket_list",
    "bucket_create",
    "bucket_get",
    "bucket_delete",
    "object_list",
    "object_delete",
    "object_signed_url",
    "object_urn",
    "translate_start",
    "translate_status",
    "hub_list",
    "project_list",
];

