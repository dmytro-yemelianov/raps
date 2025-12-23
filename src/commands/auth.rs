//! Authentication commands
//! 
//! Commands for testing authentication, logging in with 3-legged OAuth, and logging out.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::MultiSelect;

use crate::api::AuthClient;

/// Available OAuth scopes for 3-legged authentication
const AVAILABLE_SCOPES: &[(&str, &str)] = &[
    ("data:read", "Read data (hubs, projects, folders, items)"),
    ("data:write", "Write data (create/update items)"),
    ("data:create", "Create new data"),
    ("data:search", "Search for data"),
    ("account:read", "Read account information"),
    ("account:write", "Write account information"),
    ("user:read", "Read user profile"),
    ("user:write", "Write user profile"),
    ("viewables:read", "Read viewable content"),
];

/// Default scopes for login
const DEFAULT_SCOPES: &[&str] = &[
    "data:read",
    "data:write",
    "data:create",
    "account:read",
    "user:read",
    "viewables:read",
];

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Test 2-legged (client credentials) authentication
    Test,
    
    /// Login with 3-legged OAuth (opens browser)
    Login {
        /// Use default scopes without prompting
        #[arg(short, long)]
        default: bool,
    },
    
    /// Logout and clear stored tokens
    Logout,
    
    /// Show current authentication status
    Status,
    
    /// Show logged-in user profile (requires 3-legged auth)
    Whoami,
}

impl AuthCommands {
    pub async fn execute(self, auth_client: &AuthClient) -> Result<()> {
        match self {
            AuthCommands::Test => test_auth(auth_client).await,
            AuthCommands::Login { default } => login(auth_client, default).await,
            AuthCommands::Logout => logout(auth_client).await,
            AuthCommands::Status => status(auth_client).await,
            AuthCommands::Whoami => whoami(auth_client).await,
        }
    }
}

async fn test_auth(auth_client: &AuthClient) -> Result<()> {
    println!("{}", "Testing 2-legged authentication...".dimmed());
    auth_client.test_auth().await?;
    println!("{} 2-legged authentication successful!", "✓".green().bold());
    println!("  {} {}", "Client ID:".bold(), mask_string(&auth_client.config().client_id));
    println!("  {} {}", "Base URL:".bold(), auth_client.config().base_url);
    Ok(())
}

async fn login(auth_client: &AuthClient, use_defaults: bool) -> Result<()> {
    // Check if already logged in
    if auth_client.is_logged_in().await {
        println!("{}", "Already logged in. Use 'raps auth logout' to logout first.".yellow());
        return Ok(());
    }

    // Select scopes
    let scopes: Vec<&str> = if use_defaults {
        DEFAULT_SCOPES.to_vec()
    } else {
        let scope_labels: Vec<String> = AVAILABLE_SCOPES
            .iter()
            .map(|(scope, desc)| format!("{} - {}", scope, desc))
            .collect();

        // Find default selections
        let defaults: Vec<bool> = AVAILABLE_SCOPES
            .iter()
            .map(|(scope, _)| DEFAULT_SCOPES.contains(scope))
            .collect();

        let selections = MultiSelect::new()
            .with_prompt("Select OAuth scopes")
            .items(&scope_labels)
            .defaults(&defaults)
            .interact()?;

        if selections.is_empty() {
            anyhow::bail!("At least one scope must be selected");
        }

        selections.iter().map(|&i| AVAILABLE_SCOPES[i].0).collect()
    };

    println!("{}", "Starting 3-legged OAuth login...".dimmed());
    println!("  {} {:?}", "Scopes:".bold(), scopes);

    let token = auth_client.login(&scopes).await?;

    println!("\n{} Login successful!", "✓".green().bold());
    println!("  {} {}", "Access Token:".bold(), mask_string(&token.access_token));
    if token.refresh_token.is_some() {
        println!("  {} {}", "Refresh Token:".bold(), "stored".green());
    }
    println!("  {} {:?}", "Scopes:".bold(), token.scopes);

    Ok(())
}

async fn logout(auth_client: &AuthClient) -> Result<()> {
    if !auth_client.is_logged_in().await {
        println!("{}", "Not currently logged in.".yellow());
        return Ok(());
    }

    auth_client.logout().await?;
    println!("{} Logged out successfully. Stored tokens cleared.", "✓".green().bold());
    Ok(())
}

async fn status(auth_client: &AuthClient) -> Result<()> {
    println!("{}", "Authentication Status".bold());
    println!("{}", "─".repeat(40));

    // 2-legged status
    print!("  {} ", "2-legged (Client Credentials):".bold());
    match auth_client.test_auth().await {
        Ok(_) => println!("{}", "✓ Available".green()),
        Err(_) => println!("{}", "✗ Not configured".red()),
    }

    // 3-legged status
    print!("  {} ", "3-legged (User Login):".bold());
    if auth_client.is_logged_in().await {
        println!("{}", "✓ Logged in".green());
        // Try to get token info
        if let Ok(token) = auth_client.get_3leg_token().await {
            println!("    {} {}", "Token:".dimmed(), mask_string(&token));
        }
    } else {
        println!("{}", "✗ Not logged in".yellow());
        println!("    {}", "Run 'raps auth login' to authenticate".dimmed());
    }

    println!("{}", "─".repeat(40));
    Ok(())
}

async fn whoami(auth_client: &AuthClient) -> Result<()> {
    if !auth_client.is_logged_in().await {
        println!("{}", "Not logged in. Please run 'raps auth login' first.".yellow());
        return Ok(());
    }

    println!("{}", "Fetching user profile...".dimmed());
    
    let user = auth_client.get_user_info().await?;

    println!("\n{}", "User Profile".bold());
    println!("{}", "─".repeat(50));
    
    if let Some(name) = &user.name {
        println!("  {} {}", "Name:".bold(), name.cyan());
    }
    
    if let Some(email) = &user.email {
        let verified = if user.email_verified.unwrap_or(false) {
            " ✓".green().to_string()
        } else {
            "".to_string()
        };
        println!("  {} {}{}", "Email:".bold(), email, verified);
    }
    
    if let Some(username) = &user.preferred_username {
        println!("  {} {}", "Username:".bold(), username);
    }
    
    println!("  {} {}", "APS ID:".bold(), user.sub.dimmed());
    
    if let Some(profile) = &user.profile {
        println!("  {} {}", "Profile URL:".bold(), profile.dimmed());
    }

    println!("{}", "─".repeat(50));
    Ok(())
}

/// Mask a string for display (show first 4 and last 4 characters)
fn mask_string(s: &str) -> String {
    if s.len() <= 8 {
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..4], &s[s.len()-4..])
    }
}

