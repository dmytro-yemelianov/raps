// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management types

use serde::{Deserialize, Serialize};

/// Hub information
#[derive(Debug, Clone, Deserialize)]
pub struct Hub {
    #[serde(rename = "type")]
    pub hub_type: String,
    pub id: String,
    pub attributes: HubAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubAttributes {
    pub name: String,
    pub region: Option<String>,
    #[serde(rename = "extension")]
    pub extension: Option<HubExtension>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubExtension {
    #[serde(rename = "type")]
    pub extension_type: Option<String>,
}

/// Project information
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    #[serde(rename = "type")]
    pub project_type: String,
    pub id: String,
    pub attributes: ProjectAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectAttributes {
    pub name: String,
    #[serde(rename = "scopes")]
    pub scopes: Option<Vec<String>>,
}

/// Folder information
#[derive(Debug, Clone, Deserialize)]
pub struct Folder {
    #[serde(rename = "type")]
    pub folder_type: String,
    pub id: String,
    pub attributes: FolderAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FolderAttributes {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "lastModifiedTime")]
    pub last_modified_time: Option<String>,
}

/// Item (file) information
#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    #[serde(rename = "type")]
    pub item_type: String,
    pub id: String,
    pub attributes: ItemAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemAttributes {
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "lastModifiedTime")]
    pub last_modified_time: Option<String>,
    #[serde(rename = "extension")]
    pub extension: Option<ItemExtension>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemExtension {
    #[serde(rename = "type")]
    pub extension_type: Option<String>,
    pub version: Option<String>,
}

/// Version information for an item
#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    #[serde(rename = "type")]
    pub version_type: String,
    pub id: String,
    pub attributes: VersionAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VersionAttributes {
    pub name: String,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    #[serde(rename = "versionNumber")]
    pub version_number: Option<i32>,
    #[serde(rename = "createTime")]
    pub create_time: Option<String>,
    #[serde(rename = "storageSize")]
    pub storage_size: Option<i64>,
}

/// Folder contents (can be folders or items)
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum FolderContent {
    Folder(Folder),
    Item(Item),
}

/// JSON:API response wrapper
#[derive(Debug, Deserialize)]
pub struct JsonApiResponse<T> {
    pub data: T,
    #[serde(default)]
    pub included: Vec<serde_json::Value>,
    pub links: Option<JsonApiLinks>,
}

#[derive(Debug, Deserialize)]
pub struct JsonApiLinks {
    #[serde(rename = "self")]
    pub self_link: Option<JsonApiLink>,
    pub next: Option<JsonApiLink>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum JsonApiLink {
    Simple(String),
    Complex { href: String },
}

/// Request to create a folder
#[derive(Debug, Serialize)]
pub struct CreateFolderRequest {
    pub jsonapi: JsonApiVersion,
    pub data: CreateFolderData,
}

#[derive(Debug, Serialize)]
pub struct JsonApiVersion {
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub attributes: CreateFolderAttributes,
    pub relationships: CreateFolderRelationships,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderAttributes {
    pub name: String,
    pub extension: CreateFolderExtension,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderExtension {
    #[serde(rename = "type")]
    pub ext_type: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderRelationships {
    pub parent: CreateFolderParent,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderParent {
    pub data: CreateFolderParentData,
}

#[derive(Debug, Serialize)]
pub struct CreateFolderParentData {
    #[serde(rename = "type")]
    pub data_type: String,
    pub id: String,
}
