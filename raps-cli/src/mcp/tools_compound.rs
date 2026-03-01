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

        let upload_result = self.object_upload(
            bucket.clone(),
            file_path.clone(),
            Some(obj_key.clone()),
        ).await;

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
        let translate_result = self.translate_start(
            urn.clone(),
            "svf2".to_string(),
        ).await;

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
        let urn_list: Vec<&str> = urns.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

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
            let result = self.translate_start(
                urn.to_string(),
                format.clone(),
            ).await;

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
            success, failed, urn_list.len()
        ));

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
                output.push_str(&format!("  {} — {} (failures: {})\n", name, state, failures));
            }
        }

        // Rate budgets
        output.push_str("\nRate Budgets:\n");
        if rb_snap.is_empty() {
            output.push_str("  No budget data yet (first request will populate)\n");
        } else {
            for (name, remaining, limit) in &rb_snap {
                let pct = if *limit > 0 { *remaining as f64 / *limit as f64 * 100.0 } else { 100.0 };
                output.push_str(&format!("  {} — {}/{} ({:.0}% remaining)\n", name, remaining, limit, pct));
            }
        }

        // Response cache
        output.push_str(&format!("\nResponse Cache: {} entries\n", cache_len));

        output
    }
}
