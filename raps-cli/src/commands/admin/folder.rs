// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Folder permission management command implementations

use std::sync::Arc;

use anyhow::Result;
use colored::Colorize;

use raps_acc::admin::AccountAdminClient;
use raps_admin::BulkConfig;
use raps_kernel::auth::AuthClient;
use raps_kernel::config::Config;
use raps_kernel::http::HttpClientConfig;

use crate::output::OutputFormat;

use super::operations::display_bulk_result;
use super::{
    FolderCommands, create_bulk_progress_bar, get_account_id, make_progress_callback,
    parse_filter_with_ids,
};

impl FolderCommands {
    pub async fn execute(
        self,
        config: &Config,
        auth_client: &AuthClient,
        output_format: OutputFormat,
    ) -> Result<()> {
        match self {
            FolderCommands::Rights {
                email,
                account,
                level,
                folder,
                filter,
                project_ids,
                concurrency,
                dry_run,
                yes: _,
            } => {
                let account_id = get_account_id(account)?;
                let project_filter = parse_filter_with_ids(&filter, &project_ids)?;

                // Parse folder type
                let folder_type = match folder.to_lowercase().as_str() {
                    "project-files" | "projectfiles" => raps_admin::FolderType::ProjectFiles,
                    "plans" => raps_admin::FolderType::Plans,
                    _ => raps_admin::FolderType::Custom(folder.clone()),
                };

                // Create bulk config
                let bulk_config = BulkConfig {
                    concurrency: concurrency.min(50),
                    dry_run,
                    ..Default::default()
                };

                if output_format.supports_colors() {
                    println!(
                        "\n{} Bulk update folder rights for: {} in account {}",
                        "\u{2192}".cyan(),
                        email.green(),
                        account_id.cyan()
                    );
                    println!("  Folder: {}", folder);
                    println!("  Permission level: {:?}", level);
                    if let Some(f) = &filter {
                        println!("  Filter: {}", f);
                    }
                    println!("  Concurrency: {}", concurrency.min(50));
                    if dry_run {
                        println!("  {} Dry-run mode enabled", "\u{26A0}".yellow());
                    }
                    println!();
                }

                // Create API clients
                let http_config = HttpClientConfig::default();
                let admin_client = AccountAdminClient::new_with_http_config(
                    config.clone(),
                    auth_client.clone(),
                    http_config.clone(),
                );
                let permissions_client = Arc::new(
                    raps_acc::permissions::FolderPermissionsClient::new_with_http_config(
                        config.clone(),
                        auth_client.clone(),
                        http_config,
                    ),
                );

                let progress_bar = create_bulk_progress_bar(output_format);
                let on_progress = make_progress_callback(progress_bar.clone());

                // Execute bulk operation
                let result = raps_admin::bulk_update_folder_rights(
                    &admin_client,
                    permissions_client,
                    &account_id,
                    &email,
                    level.into(),
                    folder_type,
                    &project_filter,
                    bulk_config,
                    on_progress,
                )
                .await?;

                // Finish progress bar
                if let Some(pb) = progress_bar {
                    pb.finish_and_clear();
                }

                // Display results
                display_bulk_result(&result, output_format)?;

                // Exit with appropriate code
                if result.failed > 0 {
                    anyhow::bail!(
                        "Bulk operation partially failed: {} items failed",
                        result.failed
                    );
                }

                Ok(())
            }
        }
    }
}
