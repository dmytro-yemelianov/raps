// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! ACC Extended API client (Assets, Submittals, Checklists) and Project Admin

mod assets;
mod checklists;
mod project_admin;
mod submittals;
pub mod types;

pub use types::*;

use anyhow::Context;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

/// ACC Extended API client
#[derive(Clone)]
pub struct AccClient {
    pub(crate) config: Config,
    pub(crate) auth: AuthClient,
    pub(crate) http_client: reqwest::Client,
}

impl AccClient {
    /// Create a new ACC client
    pub fn new(config: Config, auth: AuthClient) -> Self {
        Self::new_with_http_config(config, auth, HttpClientConfig::default())
            .expect("default HTTP client configuration must always succeed")
    }

    /// Create a new ACC client with custom HTTP config.
    ///
    /// Returns an error if the HTTP client cannot be built (e.g. invalid proxy URL).
    pub fn new_with_http_config(
        config: Config,
        auth: AuthClient,
        http_config: HttpClientConfig,
    ) -> anyhow::Result<Self> {
        let http_client = http_config
            .create_client()
            .context("Failed to initialise HTTP client for ACC")?;

        Ok(Self {
            config,
            auth,
            http_client,
        })
    }
}
