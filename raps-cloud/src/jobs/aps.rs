// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Build APS clients from tenant-scoped encrypted credentials.

use anyhow::{Context, Result};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppState, crypto};

#[derive(Deserialize)]
struct StoredCredential {
    client_id: String,
    client_secret: String,
}

/// Resolved set of APS clients ready to make API calls.
pub struct ApsClients {
    pub admin: raps_acc::admin::AccountAdminClient,
    pub users: raps_acc::users::ProjectUsersClient,
}

/// Decrypt a tenant's stored credential and build APS clients.
pub async fn build_clients(state: &AppState, credential_id: Uuid) -> Result<ApsClients> {
    // Fetch the encrypted credential
    let cred = crate::db::credentials::get_by_id(&state.db, credential_id)
        .await?
        .context("Credential not found")?;

    // Decrypt
    let master_key = crypto::MasterKey::from_hex(&state.config.master_encryption_key)?;
    let plaintext = crypto::decrypt(&master_key, &cred.encrypted_data, &cred.nonce)?;
    let stored: StoredCredential =
        serde_json::from_slice(&plaintext).context("Invalid credential JSON")?;

    // Build raps-kernel Config
    let config = raps_kernel::config::Config {
        client_id: stored.client_id,
        client_secret: stored.client_secret,
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: String::new(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };

    let auth = raps_kernel::auth::AuthClient::new(config.clone());
    let admin = raps_acc::admin::AccountAdminClient::new(config.clone(), auth.clone());
    let users = raps_acc::users::ProjectUsersClient::new(config, auth);

    Ok(ApsClients { admin, users })
}
