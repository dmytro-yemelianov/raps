// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Guided first-run setup wizard for RAPS CLI.
//!
//! Walks the user through credential configuration, optional 3-legged login,
//! default hub selection, and project file creation, then runs `raps doctor`.

use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};

use super::{ProfileConfig, load_profiles, save_profiles};

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run_wizard() -> Result<()> {
    println!();
    println!("{}", "RAPS Setup Wizard".bold().cyan());
    println!("{}", "─".repeat(50));
    println!(
        "{}",
        "This wizard configures your Autodesk Platform Services credentials.".dimmed()
    );
    println!();

    // Step 1: check for existing credentials
    let already_configured = credentials_already_exist().await;

    if already_configured {
        println!(
            "{} Credentials are already configured.",
            "!".yellow().bold()
        );
        let reconfigure = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Do you want to reconfigure?")
            .default(false)
            .interact()?;

        if !reconfigure {
            println!("{} Setup cancelled.", "✓".green().bold());
            return Ok(());
        }
        println!();
    }

    // Step 2: choose auth type
    let auth_options = &[
        "2-legged  (Client Credentials — server-to-server)",
        "3-legged  (User Auth — browser / device flow)",
    ];

    let auth_choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which auth type?")
        .items(auth_options)
        .default(0)
        .interact()?;

    println!();

    match auth_choice {
        0 => setup_two_legged().await?,
        _ => setup_three_legged().await?,
    }

    // Step 3: optional default hub
    println!();
    select_default_hub().await?;

    // Step 4: run doctor
    println!();
    println!("{}", "Running diagnostics…".dimmed());
    println!("{}", "─".repeat(50));
    crate::commands::doctor::execute(crate::output::OutputFormat::Table).await?;

    println!();
    println!("{} Setup complete!", "✓".green().bold());
    println!(
        "{}",
        "Run 'raps auth status' at any time to check your authentication state.".dimmed()
    );

    Ok(())
}

// ── Credential detection ──────────────────────────────────────────────────────

async fn credentials_already_exist() -> bool {
    // Check keyring for a stored token
    if keyring::Entry::new("raps", "aps_token")
        .ok()
        .and_then(|e| e.get_password().ok())
        .is_some()
    {
        return true;
    }

    // Check profiles file
    if let Ok(profiles) = load_profiles().await {
        if !profiles.profiles.is_empty() {
            return true;
        }
    }

    // Check env vars
    if std::env::var("APS_CLIENT_ID").is_ok() || std::env::var("APS_CLIENT_SECRET").is_ok() {
        return true;
    }

    false
}

// ── 2-legged setup ────────────────────────────────────────────────────────────

async fn setup_two_legged() -> Result<()> {
    println!("{}", "2-Legged Client Credentials Setup".bold());
    println!("{}", "─".repeat(40));

    let client_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client ID")
        .interact_text()?;

    let client_secret: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client Secret")
        .interact_text()?;

    if client_id.trim().is_empty() || client_secret.trim().is_empty() {
        anyhow::bail!("Client ID and Client Secret must not be empty");
    }

    // Validate by attempting a token fetch
    println!();
    println!("{}", "Validating credentials…".dimmed());

    let config = raps_kernel::config::Config {
        client_id: client_id.trim().to_string(),
        client_secret: client_secret.trim().to_string(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: String::new(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };

    let auth = raps_kernel::auth::AuthClient::new(config);
    match auth.test_auth().await {
        Ok(()) => {
            println!("{} Credentials valid!", "✓".green().bold());
        }
        Err(e) => {
            println!("{} Credential validation failed: {}", "✗".red().bold(), e);
            let proceed = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Save credentials anyway?")
                .default(false)
                .interact()?;
            if !proceed {
                anyhow::bail!("Setup aborted due to invalid credentials");
            }
        }
    }

    // Save to default profile
    save_credentials_to_profile(
        "default",
        client_id.trim(),
        client_secret.trim(),
    )
    .await?;

    println!(
        "{} Credentials saved to profile '{}'.",
        "✓".green().bold(),
        "default".cyan()
    );

    Ok(())
}

// ── 3-legged setup ────────────────────────────────────────────────────────────

async fn setup_three_legged() -> Result<()> {
    println!("{}", "3-Legged User Auth Setup".bold());
    println!("{}", "─".repeat(40));

    let client_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Client ID")
        .interact_text()?;

    if client_id.trim().is_empty() {
        anyhow::bail!("Client ID must not be empty");
    }

    // Save the client_id to the default profile first (no secret for 3-legged only)
    // Users may still supply a secret for mixed auth; skip it here for simplicity.
    save_credentials_to_profile("default", client_id.trim(), "").await?;

    println!();
    println!("{}", "Launching device flow login…".dimmed());
    println!(
        "{}",
        "You will receive a URL and a short code to enter in your browser.".dimmed()
    );

    let config = raps_kernel::config::Config {
        client_id: client_id.trim().to_string(),
        client_secret: String::new(),
        base_url: "https://developer.api.autodesk.com".to_string(),
        callback_url: String::new(),
        da_nickname: None,
        http_config: raps_kernel::http::HttpClientConfig::default(),
    };

    let auth = raps_kernel::auth::AuthClient::new(config);

    // Default scopes for a useful 3-legged session
    let scopes = &[
        "data:read",
        "data:write",
        "data:create",
        "data:search",
        "account:read",
        "user:read",
    ];

    let token = auth.login_device(scopes).await?;

    let now = chrono::Utc::now().timestamp();
    let expires_in = (token.expires_at - now).max(0);
    println!(
        "{} Login successful! Token expires in {} seconds.",
        "✓".green().bold(),
        expires_in
    );

    Ok(())
}

// ── Hub selection ─────────────────────────────────────────────────────────────

async fn select_default_hub() -> Result<()> {
    let set_hub = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Set a default hub? (requires 3-legged login)")
        .default(false)
        .interact()?;

    if !set_hub {
        return Ok(());
    }

    // Build a DM client with whatever credentials are currently in env/profile
    let config = match raps_kernel::config::Config::from_env_lenient() {
        Ok(c) => c,
        Err(e) => {
            println!(
                "{} Cannot load config for hub listing: {}",
                "!".yellow().bold(),
                e
            );
            return Ok(());
        }
    };

    let auth = raps_kernel::auth::AuthClient::new(config.clone());
    let dm = raps_dm::DataManagementClient::new(config, auth);

    println!("{}", "Fetching hubs…".dimmed());

    let hubs = match dm.list_hubs().await {
        Ok(h) => h,
        Err(e) => {
            println!(
                "{} Could not list hubs (are you logged in?): {}",
                "!".yellow().bold(),
                e
            );
            return Ok(());
        }
    };

    if hubs.is_empty() {
        println!("{}", "No hubs found.".yellow());
        return Ok(());
    }

    let hub_names: Vec<String> = hubs
        .iter()
        .map(|h| format!("{} ({})", h.attributes.name, h.id))
        .collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select default hub")
        .items(&hub_names)
        .default(0)
        .interact()?;

    let selected_hub = &hubs[selection];

    // Write .raps-project in CWD
    let project_content = serde_json::json!({
        "hub_id": selected_hub.id,
        "hub_name": selected_hub.attributes.name,
    });

    let cwd = std::env::current_dir()?;
    let project_file = cwd.join(".raps-project");
    tokio::fs::write(
        &project_file,
        serde_json::to_string_pretty(&project_content)?,
    )
    .await?;

    println!(
        "{} Default hub set to '{}'. Written to {}",
        "✓".green().bold(),
        selected_hub.attributes.name.cyan(),
        project_file.display()
    );

    // Also persist to active profile context
    let mut profiles_data = load_profiles().await?;
    let profile_name = profiles_data
        .active_profile
        .clone()
        .unwrap_or_else(|| "default".to_string());

    let profile = profiles_data
        .profiles
        .entry(profile_name)
        .or_insert_with(|| ProfileConfig {
            client_id: None,
            client_secret: None,
            base_url: None,
            callback_url: None,
            da_nickname: None,
            use_keychain: None,
            context_hub_id: None,
            context_project_id: None,
            context_account_id: None,
        });

    profile.context_hub_id = Some(selected_hub.id.clone());
    save_profiles(&profiles_data).await?;

    Ok(())
}

// ── Profile helpers ───────────────────────────────────────────────────────────

async fn save_credentials_to_profile(
    name: &str,
    client_id: &str,
    client_secret: &str,
) -> Result<()> {
    let mut profiles_data = load_profiles().await?;

    let profile = profiles_data
        .profiles
        .entry(name.to_string())
        .or_insert_with(|| ProfileConfig {
            client_id: None,
            client_secret: None,
            base_url: None,
            callback_url: None,
            da_nickname: None,
            use_keychain: None,
            context_hub_id: None,
            context_project_id: None,
            context_account_id: None,
        });

    profile.client_id = Some(client_id.to_string());
    if !client_secret.is_empty() {
        profile.client_secret = Some(client_secret.to_string());
    }

    if profiles_data.active_profile.is_none() {
        profiles_data.active_profile = Some(name.to_string());
    }

    save_profiles(&profiles_data).await
}
