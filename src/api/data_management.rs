// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Data Management API module
//!
//! This module is now an adapter that wraps raps-dm service crate
//! to maintain backward compatibility with existing commands.

// API response structs may contain fields we don't use - this is expected for external API contracts
#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::AuthClient;
use crate::config::Config;

// Re-export types from raps-dm for backward compatibility
pub use raps_dm::types::*;

/// Data Management API client (adapter wrapping raps-dm service crate with cached clients)
#[derive(Clone)]
pub struct DataManagementClient {
    config: Config,
    auth: AuthClient,
    http_client: reqwest::Client,
    // Cached kernel clients (lazy-initialized)
    kernel_config: Arc<Mutex<Option<raps_kernel::Config>>>,
    kernel_http: Arc<Mutex<Option<raps_kernel::HttpClient>>>,
    kernel_auth: Arc<Mutex<Option<raps_kernel::AuthClient>>>,
}

impl DataManagementClient {
    /// Create a new Data Management client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, crate::http::HttpClientConfig::default())
    }

    /// Create a new Data Management client with custom HTTP config
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: crate::http::HttpClientConfig,
    ) -> Self {
        let http_client = http_config
            .create_client()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            auth,
            http_client,
            kernel_config: Arc::new(Mutex::new(None)),
            kernel_http: Arc::new(Mutex::new(None)),
            kernel_auth: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or create kernel config (cached)
    async fn get_kernel_config(&self) -> Result<raps_kernel::Config> {
        let mut config = self.kernel_config.lock().await;
        if config.is_none() {
            *config = Some(raps_kernel::Config {
                client_id: self.config.client_id.clone(),
                client_secret: self.config.client_secret.clone(),
                base_url: self.config.base_url.clone(),
                callback_url: self.config.callback_url.clone(),
                da_nickname: self.config.da_nickname.clone(),
            });
        }
        Ok(config.as_ref().unwrap().clone())
    }

    /// Get or create kernel HTTP client (cached)
    async fn get_kernel_http(&self) -> Result<raps_kernel::HttpClient> {
        let mut http = self.kernel_http.lock().await;
        if http.is_none() {
            let config = raps_kernel::HttpClientConfig {
                timeout: std::time::Duration::from_secs(120),
                connect_timeout: std::time::Duration::from_secs(30),
                max_retries: 3,
                retry_base_delay: std::time::Duration::from_secs(1),
                retry_max_delay: std::time::Duration::from_secs(60),
                retry_jitter: true,
            };
            *http = Some(
                raps_kernel::HttpClient::new(config)
                    .map_err(|e| anyhow::anyhow!("Failed to create kernel HTTP client: {}", e))?,
            );
        }
        Ok(http.as_ref().unwrap().clone())
    }

    /// Get or create kernel auth client (cached)
    async fn get_kernel_auth(&self) -> Result<raps_kernel::AuthClient> {
        let mut auth = self.kernel_auth.lock().await;
        if auth.is_none() {
            let kernel_config = self.get_kernel_config().await?;
            *auth = Some(
                raps_kernel::AuthClient::new(kernel_config)
                    .map_err(|e| anyhow::anyhow!("Failed to create kernel auth client: {}", e))?,
            );
        }
        Ok(auth.as_ref().unwrap().clone())
    }

    /// List all accessible hubs
    pub async fn list_hubs(&self) -> Result<Vec<Hub>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();

        let hub_client =
            raps_dm::HubClient::new(kernel_http, kernel_auth, kernel_config, project_url);

        hub_client
            .list_hubs()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get hub details
    pub async fn get_hub(&self, hub_id: &str) -> Result<Hub> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();

        let hub_client =
            raps_dm::HubClient::new(kernel_http, kernel_auth, kernel_config, project_url);

        hub_client
            .get_hub(hub_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List projects in a hub
    pub async fn list_projects(&self, hub_id: &str) -> Result<Vec<Project>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();

        let project_client =
            raps_dm::ProjectClient::new(kernel_http, kernel_auth, kernel_config, project_url);

        project_client
            .list_projects(hub_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get project details
    pub async fn get_project(&self, hub_id: &str, project_id: &str) -> Result<Project> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();

        let project_client =
            raps_dm::ProjectClient::new(kernel_http, kernel_auth, kernel_config, project_url);

        project_client
            .get_project(hub_id, project_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get top-level folders in a project
    pub async fn get_top_folders(&self, hub_id: &str, project_id: &str) -> Result<Vec<Folder>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();
        let data_url = kernel_config.data_url();

        let folder_client = raps_dm::FolderClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            project_url,
            data_url,
        );

        folder_client
            .get_top_folders(hub_id, project_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// List contents of a folder (returns raw JSON values)
    pub async fn list_folder_contents(
        &self,
        project_id: &str,
        folder_id: &str,
    ) -> Result<Vec<serde_json::Value>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();
        let data_url = kernel_config.data_url();

        let folder_client = raps_dm::FolderClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            project_url,
            data_url,
        );

        folder_client
            .list_folder_contents(project_id, folder_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Create a new folder
    pub async fn create_folder(
        &self,
        project_id: &str,
        parent_folder_id: &str,
        folder_name: &str,
    ) -> Result<Folder> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let project_url = kernel_config.project_url();
        let data_url = kernel_config.data_url();

        let folder_client = raps_dm::FolderClient::new(
            kernel_http,
            kernel_auth,
            kernel_config,
            project_url,
            data_url,
        );

        folder_client
            .create_folder(project_id, parent_folder_id, folder_name)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get item details
    pub async fn get_item(&self, project_id: &str, item_id: &str) -> Result<Item> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let data_url = kernel_config.data_url();

        let item_client =
            raps_dm::ItemClient::new(kernel_http, kernel_auth, kernel_config, data_url);

        item_client
            .get_item(project_id, item_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Get versions of an item
    pub async fn get_item_versions(&self, project_id: &str, item_id: &str) -> Result<Vec<Version>> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let data_url = kernel_config.data_url();

        let item_client =
            raps_dm::ItemClient::new(kernel_http, kernel_auth, kernel_config, data_url);

        item_client
            .get_item_versions(project_id, item_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }

    /// Create an item from storage (upload a file)
    pub async fn create_item_from_storage(
        &self,
        project_id: &str,
        folder_id: &str,
        filename: &str,
        storage_id: &str,
    ) -> Result<Item> {
        let kernel_config = self.get_kernel_config().await?;
        let kernel_http = self.get_kernel_http().await?;
        let kernel_auth = self.get_kernel_auth().await?;
        let data_url = kernel_config.data_url();

        let item_client =
            raps_dm::ItemClient::new(kernel_http, kernel_auth, kernel_config, data_url);

        item_client
            .create_item_from_storage(project_id, folder_id, filename, storage_id)
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}
