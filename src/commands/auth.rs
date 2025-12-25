//! Authentication commands
//!
//! Commands for testing authentication, logging in with 3-legged OAuth, and logging out.

use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use dialoguer::MultiSelect;
use serde::Serialize;

use crate::api::AuthClient;
use crate::output::OutputFormat;

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
        /// Use device code flow instead of browser (headless-friendly)
        #[arg(long)]
        device: bool,
        /// Provide access token directly (for CI/CD - use with caution)
        #[arg(long)]
        token: Option<String>,
        /// Refresh token (optional, used with --token)
        #[arg(long)]
        refresh_token: Option<String>,
        /// Token expiry in seconds (default: 3600, used with --token)
        #[arg(long, default_value = "3600")]
        expires_in: u64,
    },

    /// Logout and clear stored tokens
    Logout,

    /// Show current authentication status
    Status,

    /// Show logged-in user profile (requires 3-legged auth)
    Whoami,

    /// Inspect token details (scopes, expiry) - useful for CI
    Inspect {
        /// Exit with code 1 if token expires within N seconds (for CI)
        #[arg(long)]
        warn_expiry_seconds: Option<u64>,
    },
}

impl AuthCommands {
    pub async fn execute(
        self,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            AuthCommands::Test => test_auth(auth_client, output_format).await,
            AuthCommands::Login {
                default,
                device,
                token,
                refresh_token,
                expires_in,
            } => {
                login(
                    auth_client,
                    default,
                    device,
                    token,
                    refresh_token,
                    expires_in,
                    output_format,
                )
                .await
            }
            AuthCommands::Logout => logout(auth_client, output_format).await,
            AuthCommands::Status => status(auth_client, output_format).await,
            AuthCommands::Whoami => whoami(auth_client, output_format).await,
            AuthCommands::Inspect { warn_expiry_seconds } => {
                inspect_token(auth_client, warn_expiry_seconds, output_format).await
            }
        }
    }
}

#[derive(Serialize)]
struct TestAuthOutput {
    success: bool,
    client_id: String,
    base_url: String,
}

async fn test_auth(auth_client: &AuthClient, output_format: OutputFormat) -> Result<()> {
    if output_format.supports_colors() {
        println!("{}", "Testing 2-legged authentication...".dimmed());
    }
    auth_client.test_auth().await?;

    let output = TestAuthOutput {
        success: true,
        client_id: mask_string(&auth_client.config().client_id),
        base_url: auth_client.config().base_url.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} 2-legged authentication successful!", "✓".green().bold());
            println!("  {} {}", "Client ID:".bold(), output.client_id);
            println!("  {} {}", "Base URL:".bold(), output.base_url);
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct LoginOutput {
    success: bool,
    access_token: String,
    refresh_token_stored: bool,
    scopes: Vec<String>,
}

async fn login(
    auth_client: &AuthClient,
    use_defaults: bool,
    device: bool,
    token: Option<String>,
    refresh_token: Option<String>,
    expires_in: u64,
    output_format: OutputFormat,
) -> Result<()> {
    // Check if already logged in
    if auth_client.is_logged_in().await {
        let msg = "Already logged in. Use 'raps auth logout' to logout first.";
        match output_format {
            OutputFormat::Table => println!("{}", msg.yellow()),
            _ => output_format.write_message(msg)?,
        }
        return Ok(());
    }

    // Handle token-based login (CI/CD scenario)
    if let Some(access_token) = token {
        eprintln!(
            "{}",
            "⚠️  WARNING: Using token-based login. Tokens should be kept secure!"
                .yellow()
                .bold()
        );
        eprintln!(
            "{}",
            "   This is intended for CI/CD environments. Never commit tokens to version control."
                .dimmed()
        );

        let scopes = if use_defaults {
            DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect()
        } else {
            DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect() // Default scopes for token login
        };

        let stored = auth_client
            .login_with_token(access_token, refresh_token, expires_in, scopes)
            .await?;

        let output = LoginOutput {
            success: true,
            access_token: mask_string(&stored.access_token),
            refresh_token_stored: stored.refresh_token.is_some(),
            scopes: stored.scopes.clone(),
        };

        match output_format {
            OutputFormat::Table => {
                println!("\n{} Login successful!", "✓".green().bold());
                println!("  {} {}", "Access Token:".bold(), output.access_token);
                if output.refresh_token_stored {
                    println!("  {} {}", "Refresh Token:".bold(), "stored".green());
                }
                println!("  {} {:?}", "Scopes:".bold(), output.scopes);
            }
            _ => {
                output_format.write(&output)?;
            }
        }

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

    if output_format.supports_colors() {
        println!("{}", "Starting 3-legged OAuth login...".dimmed());
        println!("  {} {:?}", "Scopes:".bold(), scopes);
    }

    // Use device code flow if requested
    let token = if device {
        auth_client.login_device(&scopes).await?
    } else {
        auth_client.login(&scopes).await?
    };

    let output = LoginOutput {
        success: true,
        access_token: mask_string(&token.access_token),
        refresh_token_stored: token.refresh_token.is_some(),
        scopes: token.scopes.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{} Login successful!", "✓".green().bold());
            println!("  {} {}", "Access Token:".bold(), output.access_token);
            if output.refresh_token_stored {
                println!("  {} {}", "Refresh Token:".bold(), "stored".green());
            }
            println!("  {} {:?}", "Scopes:".bold(), output.scopes);
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct LogoutOutput {
    success: bool,
    message: String,
}

async fn logout(auth_client: &AuthClient, output_format: OutputFormat) -> Result<()> {
    if !auth_client.is_logged_in().await {
        let msg = "Not currently logged in.";
        match output_format {
            OutputFormat::Table => println!("{}", msg.yellow()),
            _ => {
                let output = LogoutOutput {
                    success: false,
                    message: msg.to_string(),
                };
                output_format.write(&output)?;
            }
        }
        return Ok(());
    }

    auth_client.logout().await?;

    let output = LogoutOutput {
        success: true,
        message: "Logged out successfully. Stored tokens cleared.".to_string(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("{} {}", "✓".green().bold(), output.message);
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct StatusOutput {
    two_legged: TwoLeggedStatus,
    three_legged: ThreeLeggedStatus,
}

#[derive(Serialize)]
struct TwoLeggedStatus {
    available: bool,
}

#[derive(Serialize)]
struct ThreeLeggedStatus {
    logged_in: bool,
    token: Option<String>,
    expires_at: Option<i64>,
    expires_in_seconds: Option<i64>,
}

async fn status(auth_client: &AuthClient, output_format: OutputFormat) -> Result<()> {
    let two_legged_available = auth_client.test_auth().await.is_ok();
    let three_legged_logged_in = auth_client.is_logged_in().await;
    let token = if three_legged_logged_in {
        auth_client
            .get_3leg_token()
            .await
            .ok()
            .map(|t| mask_string(&t))
    } else {
        None
    };

    let expires_at = auth_client.get_token_expiry().await;
    let expires_in_seconds = expires_at.map(|exp| {
        let now = chrono::Utc::now().timestamp();
        (exp - now).max(0)
    });

    let output = StatusOutput {
        two_legged: TwoLeggedStatus {
            available: two_legged_available,
        },
        three_legged: ThreeLeggedStatus {
            logged_in: three_legged_logged_in,
            token,
            expires_at,
            expires_in_seconds,
        },
    };

    match output_format {
        OutputFormat::Table => {
            println!("{}", "Authentication Status".bold());
            println!("{}", "─".repeat(40));

            print!("  {} ", "2-legged (Client Credentials):".bold());
            if output.two_legged.available {
                println!("{}", "✓ Available".green());
            } else {
                println!("{}", "✗ Not configured".red());
            }

            print!("  {} ", "3-legged (User Login):".bold());
            if output.three_legged.logged_in {
                println!("{}", "✓ Logged in".green());
                if let Some(ref token) = output.three_legged.token {
                    println!("    {} {}", "Token:".dimmed(), token);
                }
                if let Some(expires_in) = output.three_legged.expires_in_seconds {
                    if expires_in > 0 {
                        let hours = expires_in / 3600;
                        let minutes = (expires_in % 3600) / 60;
                        println!("    {} {}h {}m", "Expires in:".dimmed(), hours, minutes);
                    } else {
                        println!("    {} {}", "Status:".dimmed(), "Expired".red());
                    }
                }
            } else {
                println!("{}", "✗ Not logged in".yellow());
                println!("    {}", "Run 'raps auth login' to authenticate".dimmed());
            }

            println!("{}", "─".repeat(40));
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct WhoamiOutput {
    name: Option<String>,
    email: Option<String>,
    email_verified: Option<bool>,
    username: Option<String>,
    aps_id: String,
    profile_url: Option<String>,
}

async fn whoami(auth_client: &AuthClient, output_format: OutputFormat) -> Result<()> {
    if !auth_client.is_logged_in().await {
        let msg = "Not logged in. Please run 'raps auth login' first.";
        match output_format {
            OutputFormat::Table => println!("{}", msg.yellow()),
            _ => output_format.write_message(msg)?,
        }
        return Ok(());
    }

    if output_format.supports_colors() {
        println!("{}", "Fetching user profile...".dimmed());
    }

    let user = auth_client.get_user_info().await?;

    let output = WhoamiOutput {
        name: user.name.clone(),
        email: user.email.clone(),
        email_verified: user.email_verified,
        username: user.preferred_username.clone(),
        aps_id: user.sub.clone(),
        profile_url: user.profile.clone(),
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "User Profile".bold());
            println!("{}", "─".repeat(50));

            if let Some(ref name) = output.name {
                println!("  {} {}", "Name:".bold(), name.cyan());
            }

            if let Some(ref email) = output.email {
                let verified = if output.email_verified.unwrap_or(false) {
                    " ✓".green().to_string()
                } else {
                    "".to_string()
                };
                println!("  {} {}{}", "Email:".bold(), email, verified);
            }

            if let Some(ref username) = output.username {
                println!("  {} {}", "Username:".bold(), username);
            }

            println!("  {} {}", "APS ID:".bold(), output.aps_id.dimmed());

            if let Some(ref profile) = output.profile_url {
                println!("  {} {}", "Profile URL:".bold(), profile.dimmed());
            }

            println!("{}", "─".repeat(50));
        }
        _ => {
            output_format.write(&output)?;
        }
    }
    Ok(())
}

/// Mask a string for display (show first 4 and last 4 characters)
fn mask_string(s: &str) -> String {
    if s.len() <= 8 {
        "*".repeat(s.len())
    } else {
        format!("{}...{}", &s[..4], &s[s.len() - 4..])
    }
}

#[derive(Serialize)]
struct InspectOutput {
    authenticated: bool,
    token_type: Option<String>,
    expires_in_seconds: Option<i64>,
    expires_at: Option<String>,
    scopes: Option<Vec<String>>,
    is_expiring_soon: bool,
    warning: Option<String>,
}

async fn inspect_token(
    _auth_client: &AuthClient,
    warn_expiry_seconds: Option<u64>,
    output_format: OutputFormat,
) -> Result<()> {
    let backend = crate::storage::StorageBackend::from_env();
    let storage = crate::storage::TokenStorage::new(backend);
    
    // Try to load stored token info
    let token_data = storage.load()?;
    
    let output = if let Some(data) = token_data {
        let now = chrono::Utc::now().timestamp();
        let expires_at = data.expires_at;
        let expires_in = expires_at - now;
        
        // Use scopes directly (already Vec<String>)
        let scopes: Vec<String> = data.scopes.clone();

        // Check if expiring soon
        let warn_threshold = warn_expiry_seconds.unwrap_or(300) as i64; // Default 5 minutes
        let is_expiring_soon = expires_in > 0 && expires_in < warn_threshold;
        let is_expired = expires_in <= 0;
        
        let warning = if is_expired {
            Some("Token has expired!".to_string())
        } else if is_expiring_soon {
            Some(format!("Token expires in {} seconds", expires_in))
        } else {
            None
        };

        InspectOutput {
            authenticated: !is_expired,
            token_type: Some(if data.access_token.starts_with("ey") { "JWT".to_string() } else { "Opaque".to_string() }),
            expires_in_seconds: Some(expires_in),
            expires_at: Some(
                chrono::DateTime::from_timestamp(expires_at, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "Unknown".to_string())
            ),
            scopes: Some(scopes),
            is_expiring_soon: is_expiring_soon || is_expired,
            warning,
        }
    } else {
        InspectOutput {
            authenticated: false,
            token_type: None,
            expires_in_seconds: None,
            expires_at: None,
            scopes: None,
            is_expiring_soon: true,
            warning: Some("No token found. Run 'raps auth login' first.".to_string()),
        }
    };

    match output_format {
        OutputFormat::Table => {
            println!("\n{}", "Token Inspection".bold());
            println!("{}", "─".repeat(60));

            if output.authenticated {
                println!("  {} {}", "Authenticated:".bold(), "Yes".green());
            } else {
                println!("  {} {}", "Authenticated:".bold(), "No".red());
            }

            if let Some(ref token_type) = output.token_type {
                println!("  {} {}", "Token type:".bold(), token_type);
            }

            if let Some(expires_in) = output.expires_in_seconds {
                let color = if expires_in <= 0 {
                    "Expired".red().to_string()
                } else if expires_in < 300 {
                    format!("{} seconds", expires_in).yellow().to_string()
                } else {
                    format!("{} seconds ({:.1} hours)", expires_in, expires_in as f64 / 3600.0).to_string()
                };
                println!("  {} {}", "Expires in:".bold(), color);
            }

            if let Some(ref expires_at) = output.expires_at {
                println!("  {} {}", "Expires at:".bold(), expires_at.dimmed());
            }

            if let Some(ref scopes) = output.scopes {
                println!("  {} {}", "Scopes:".bold(), scopes.len());
                for scope in scopes {
                    println!("    {} {}", "•".cyan(), scope);
                }
            }

            if let Some(ref warning) = output.warning {
                println!("\n  {} {}", "!".yellow().bold(), warning.yellow());
            }

            println!("{}", "─".repeat(60));
        }
        _ => {
            output_format.write(&output)?;
        }
    }

    // Exit with code 1 if token is expiring soon (for CI)
    if warn_expiry_seconds.is_some() && output.is_expiring_soon {
        std::process::exit(1);
    }

    Ok(())
}
