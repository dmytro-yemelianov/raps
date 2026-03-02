// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Compound MCP tools — multi-step workflows as single tool calls.
//!
//! These tools compose atomic operations (upload, translate, poll, etc.)
//! into complete workflows that AI agents can invoke in one call.

use super::server::RapsServer;

impl RapsServer {
    /// Upload a file and start SVF2 translation for viewing.
    ///
    /// Steps: upload → get URN → translate SVF2 → return URN + status
    pub(crate) async fn workflow_prepare_for_viewing(
        &self,
        file_path: String,
        bucket_key: Option<String>,
        object_key: Option<String>,
    ) -> String {
        // Step 1: Upload
        let bucket = bucket_key.unwrap_or_else(|| "raps-workflow-temp".to_string());
        let obj_key = object_key.unwrap_or_else(|| {
            std::path::Path::new(&file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "upload".to_string())
        });

        let upload_result = self
            .object_upload(bucket.clone(), file_path.clone(), Some(obj_key.clone()))
            .await;

        if upload_result.contains("Error") || upload_result.contains("Failed") {
            return format!("Upload failed: {upload_result}");
        }

        // Step 2: Get URN
        let urn_result = self.object_urn(bucket.clone(), obj_key.clone()).await;
        if urn_result.contains("Error") {
            return format!("Failed to get URN: {urn_result}");
        }

        // The URN result contains the base64-encoded URN
        let urn = urn_result.trim().to_string();

        // Step 3: Start translation
        let translate_result = self.translate_start(urn.clone(), "svf2".to_string()).await;

        if translate_result.contains("Error") && !translate_result.contains("already") {
            return format!("Translation start failed: {translate_result}");
        }

        format!(
            "Workflow: Prepare for Viewing\n\
             ─────────────────────────────\n\
             File: {file_path}\n\
             Bucket: {bucket}\n\
             Object: {obj_key}\n\
             URN: {urn}\n\
             Translation: SVF2 started\n\n\
             Next steps:\n\
             • Use `translate_status` with the URN to check progress\n\
             • Once complete, the model can be viewed in Autodesk Viewer"
        )
    }

    /// Get a comprehensive model analysis: translation status.
    ///
    /// Steps: check status → report
    pub(crate) async fn workflow_analyze_model(
        &self,
        urn: String,
        _region: Option<String>,
    ) -> String {
        let mut output = String::from("Workflow: Analyze Model\n──────────────────────\n\n");

        // Step 1: Get translation status (includes manifest details)
        let status = self.translate_status(urn.clone()).await;
        output.push_str(&format!("Translation Status:\n{status}\n\n"));

        if status.contains("failed") || status.contains("not found") {
            output.push_str("Cannot analyze: model translation is not complete or failed.\n");
            output.push_str("Suggestions:\n");
            output.push_str("  • Use `translate_start` to begin translation\n");
            output.push_str("  • Check if the URN is correct\n");
        } else if status.contains("complete") || status.contains("success") {
            output.push_str("Model is fully translated and ready for use.\n");
            output.push_str("Available actions:\n");
            output.push_str("  • Use `translate_download` to download derivatives\n");
            output.push_str("  • The model can be viewed in Autodesk Viewer\n");
        } else {
            output.push_str("Translation is in progress. Check back with `translate_status`.\n");
        }

        output
    }

    /// Translate multiple URNs in batch with progress tracking.
    ///
    /// Takes comma-separated URNs, translates all, returns status for each.
    pub(crate) async fn workflow_batch_translate(
        &self,
        urns: String,
        output_format: Option<String>,
        _region: Option<String>,
    ) -> String {
        let format = output_format.unwrap_or_else(|| "svf2".to_string());
        let urn_list: Vec<&str> = urns
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if urn_list.is_empty() {
            return "No URNs provided. Pass comma-separated URNs.".to_string();
        }

        let mut output = format!(
            "Workflow: Batch Translate ({} URNs, format: {})\n{}\n\n",
            urn_list.len(),
            format,
            "─".repeat(50),
        );

        let mut success = 0;
        let mut failed = 0;

        for (i, urn) in urn_list.iter().enumerate() {
            let result = self.translate_start(urn.to_string(), format.clone()).await;

            let status = if result.contains("Error") && !result.contains("already") {
                failed += 1;
                "FAILED"
            } else {
                success += 1;
                "STARTED"
            };

            output.push_str(&format!(
                "  [{}/{}] {} — {}\n",
                i + 1,
                urn_list.len(),
                status,
                if urn.len() > 50 { &urn[..50] } else { urn },
            ));
        }

        output.push_str(&format!(
            "\nSummary: {} started, {} failed out of {} total\n\
             Use `translate_status` to check individual progress.",
            success,
            failed,
            urn_list.len()
        ));

        output
    }

    /// Compare two model versions by checking translation status for both URNs.
    ///
    /// Steps: get status for URN A → get status for URN B → diff report
    pub(crate) async fn workflow_compare_versions(
        &self,
        urn_a: String,
        urn_b: String,
        label_a: Option<String>,
        label_b: Option<String>,
    ) -> String {
        let label_a = label_a.unwrap_or_else(|| "Version A".to_string());
        let label_b = label_b.unwrap_or_else(|| "Version B".to_string());

        let mut output = format!("Workflow: Compare Versions\n{}\n\n", "─".repeat(40),);

        // Step 1: Get status for URN A
        let status_a = self.translate_status(urn_a.clone()).await;
        output.push_str(&format!("{label_a}:\n"));
        output.push_str(&format!(
            "  URN: {}\n",
            if urn_a.len() > 60 {
                format!("{}…", &urn_a[..59])
            } else {
                urn_a.clone()
            }
        ));
        let state_a = if status_a.contains("complete") || status_a.contains("success") {
            "complete"
        } else if status_a.contains("failed") {
            "failed"
        } else if status_a.contains("not found") {
            "not found"
        } else {
            "in progress"
        };
        output.push_str(&format!("  Status: {state_a}\n\n"));

        // Step 2: Get status for URN B
        let status_b = self.translate_status(urn_b.clone()).await;
        output.push_str(&format!("{label_b}:\n"));
        output.push_str(&format!(
            "  URN: {}\n",
            if urn_b.len() > 60 {
                format!("{}…", &urn_b[..59])
            } else {
                urn_b.clone()
            }
        ));
        let state_b = if status_b.contains("complete") || status_b.contains("success") {
            "complete"
        } else if status_b.contains("failed") {
            "failed"
        } else if status_b.contains("not found") {
            "not found"
        } else {
            "in progress"
        };
        output.push_str(&format!("  Status: {state_b}\n\n"));

        // Step 3: Comparison summary
        output.push_str("Comparison:\n");
        if state_a == "complete" && state_b == "complete" {
            output.push_str("  Both versions are fully translated and ready for comparison.\n");
            output.push_str("  Suggestions:\n");
            output.push_str("  • Use `translate_download` to download derivatives for both\n");
            output.push_str(
                "  • Both models can be loaded in Autodesk Viewer for visual comparison\n",
            );
        } else {
            if state_a != "complete" {
                output.push_str(&format!("  {label_a} is not ready ({state_a})\n"));
            }
            if state_b != "complete" {
                output.push_str(&format!("  {label_b} is not ready ({state_b})\n"));
            }
            output.push_str("  Wait for both translations to complete before comparing.\n");
        }

        output
    }

    /// Set up a new project workspace: create bucket, prepare for uploads.
    ///
    /// Steps: create bucket → report ready state
    pub(crate) async fn workflow_setup_project(
        &self,
        project_name: String,
        bucket_region: Option<String>,
    ) -> String {
        let mut output = format!("Workflow: Setup Project\n{}\n\n", "─".repeat(40),);

        // Normalize project name to a valid bucket key
        let bucket_key = project_name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .collect::<String>();

        if bucket_key.len() < 3 {
            return format!(
                "Error: Project name '{project_name}' is too short for a bucket key (min 3 chars)."
            );
        }

        output.push_str(&format!("Project: {project_name}\n"));
        output.push_str(&format!("Bucket key: {bucket_key}\n\n"));

        // Step 1: Create bucket
        let region = bucket_region.unwrap_or_else(|| "US".to_string());
        let create_result = self
            .bucket_create(bucket_key.clone(), "persistent".to_string(), region.clone())
            .await;

        if create_result.contains("Error") || create_result.contains("Failed") {
            if create_result.contains("409")
                || create_result.contains("already exists")
                || create_result.contains("Conflict")
            {
                output.push_str(&format!(
                    "Bucket '{bucket_key}' already exists — using existing bucket.\n\n"
                ));
            } else {
                return format!("Bucket creation failed: {create_result}");
            }
        } else {
            output.push_str(&format!(
                "Bucket '{bucket_key}' created (region: {region}, policy: persistent).\n\n"
            ));
        }

        output.push_str("Project workspace is ready.\n\n");
        output.push_str("Next steps:\n");
        output.push_str(&format!(
            "  • Upload files: use `object_upload` with bucket '{bucket_key}'\n"
        ));
        output.push_str("  • Start translations: use `workflow_prepare_for_viewing`\n");
        output.push_str("  • List contents: use `object_list`\n");

        output
    }

    /// Get swarm orchestration status for AI agent introspection.
    pub(crate) async fn swarm_status_tool(&self) -> String {
        let cb_snap = raps_kernel::circuit_breaker::registry().snapshot();
        let rb_snap = raps_kernel::rate_budget::registry().snapshot();
        let cache_len = raps_kernel::response_cache::cache().len();

        let mut output = String::from("Swarm Orchestration Status\n──────────────────────────\n\n");

        // Circuit breakers
        output.push_str("Circuit Breakers:\n");
        if cb_snap.is_empty() {
            output.push_str("  All circuits closed (healthy)\n");
        } else {
            for (name, state, failures) in &cb_snap {
                output.push_str(&format!(
                    "  {} — {} (failures: {})\n",
                    name, state, failures
                ));
            }
        }

        // Rate budgets
        output.push_str("\nRate Budgets:\n");
        if rb_snap.is_empty() {
            output.push_str("  No budget data yet (first request will populate)\n");
        } else {
            for (name, remaining, limit) in &rb_snap {
                let pct = if *limit > 0 {
                    *remaining as f64 / *limit as f64 * 100.0
                } else {
                    100.0
                };
                output.push_str(&format!(
                    "  {} — {}/{} ({:.0}% remaining)\n",
                    name, remaining, limit, pct
                ));
            }
        }

        // Response cache
        output.push_str(&format!("\nResponse Cache: {} entries\n", cache_len));

        output
    }
}
