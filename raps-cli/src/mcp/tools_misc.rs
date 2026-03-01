// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Miscellaneous tool implementations (Custom API, Webhooks, Design Automation, Reality Capture).

use serde_json::{Map, Value};

use raps_webhooks::UpdateWebhookRequest;

use super::server::{RapsServer, SENSITIVE_HEADERS};

impl RapsServer {
    // ================================================================
    // Custom API Requests
    // ================================================================

    pub(crate) async fn api_request(
        &self,
        method: String,
        endpoint: String,
        query: Option<Map<String, Value>>,
        headers: Option<Map<String, Value>>,
        body: Option<Value>,
    ) -> String {
        use raps_kernel::http::is_allowed_url;
        use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderName, HeaderValue};

        // Validate HTTP method
        let http_method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "PATCH" => reqwest::Method::PATCH,
            "DELETE" => reqwest::Method::DELETE,
            _ => {
                return format!(
                    "Invalid HTTP method '{}'. Supported: GET, POST, PUT, PATCH, DELETE",
                    method
                );
            }
        };

        // Build full URL from endpoint and query parameters
        let full_url = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint.clone()
        } else {
            let endpoint = if endpoint.starts_with('/') {
                endpoint.clone()
            } else {
                format!("/{}", endpoint)
            };
            format!(
                "{}{}",
                self.config().base_url.trim_end_matches('/'),
                endpoint
            )
        };

        // Add query parameters
        let full_url = if let Some(query_params) = &query {
            let query_string: String = query_params
                .iter()
                .map(|(k, v)| {
                    let val = v
                        .as_str()
                        .map(String::from)
                        .unwrap_or_else(|| v.to_string());
                    format!("{}={}", urlencoding::encode(k), urlencoding::encode(&val))
                })
                .collect::<Vec<_>>()
                .join("&");
            if full_url.contains('?') {
                format!("{}&{}", full_url, query_string)
            } else {
                format!("{}?{}", full_url, query_string)
            }
        } else {
            full_url
        };

        // Validate URL is allowed (APS domains only)
        if !is_allowed_url(&full_url) {
            return format!(
                "URL not allowed: {}\n\n\
                 Only APS API endpoints are permitted for security reasons.\n\
                 Allowed domains: developer.api.autodesk.com, api.userprofile.autodesk.com, \
                 acc.autodesk.com, developer.autodesk.com, b360dm.autodesk.com, cdn.derivative.autodesk.io",
                full_url
            );
        }

        // Validate body is only used with appropriate methods
        let supports_body = matches!(
            http_method,
            reqwest::Method::POST | reqwest::Method::PUT | reqwest::Method::PATCH
        );
        if body.is_some() && !supports_body {
            return format!(
                "Request body is not allowed for {} requests",
                http_method.as_str()
            );
        }

        // Get auth token
        let auth_client = self.get_auth_client().await;
        let token = match auth_client.get_3leg_token().await {
            Ok(token) => token,
            Err(_) => match auth_client.get_token().await {
                Ok(token) => token,
                Err(e) => {
                    return format!(
                        "Authentication failed: {}\n\n\
                         Run 'raps auth login' for 3-legged auth or configure client credentials.",
                        e
                    );
                }
            },
        };

        // Build HTTP client
        let client = match self.http_config().create_client() {
            Ok(c) => c,
            Err(e) => return format!("Failed to create HTTP client: {}", e),
        };

        // Build request
        let mut request = client.request(http_method.clone(), &full_url);

        // Add authorization
        request = request.header(AUTHORIZATION, format!("Bearer {}", token));

        // Add custom headers (excluding Authorization)
        if let Some(custom_headers) = headers {
            for (key, value) in custom_headers {
                if key.to_lowercase() == "authorization" {
                    continue; // Cannot override Authorization header
                }
                let val_str = value
                    .as_str()
                    .map(String::from)
                    .unwrap_or_else(|| value.to_string());
                if let (Ok(name), Ok(val)) = (
                    HeaderName::try_from(key.as_str()),
                    HeaderValue::try_from(&val_str),
                ) {
                    request = request.header(name, val);
                }
            }
        }

        // Add body if present
        if let Some(body) = body {
            request = request.header(CONTENT_TYPE, "application/json").json(&body);
        }

        // Execute request
        let response = match request.send().await {
            Ok(r) => r,
            Err(e) => return format!("Request failed: {}", e),
        };

        let status = response.status();
        let status_code = status.as_u16();

        // Collect response headers for display (filter sensitive ones)
        let response_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter(|(k, _)| !SENSITIVE_HEADERS.contains(&k.as_str()))
            .take(10)
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Get content type
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read response body
        let body_result = response.text().await;
        let body_text = match body_result {
            Ok(text) => text,
            Err(e) => return format!("Failed to read response: {}", e),
        };

        // Try to parse as JSON for pretty formatting
        let formatted_body = if content_type.contains("json") {
            match serde_json::from_str::<Value>(&body_text) {
                Ok(json) => serde_json::to_string_pretty(&json).unwrap_or(body_text),
                Err(_) => body_text,
            }
        } else {
            // Truncate non-JSON responses
            if body_text.len() > 2000 {
                format!(
                    "{}...\n[Truncated, {} bytes total]",
                    &body_text[..2000],
                    body_text.len()
                )
            } else {
                body_text
            }
        };

        // Format output
        let mut output = format!(
            "HTTP {} {}\nStatus: {} {}\n",
            http_method.as_str(),
            full_url,
            status_code,
            status.canonical_reason().unwrap_or("")
        );

        output.push_str("\nHeaders:\n");
        for (k, v) in response_headers {
            output.push_str(&format!("  {}: {}\n", k, v));
        }

        output.push_str("\nBody:\n");
        output.push_str(&formatted_body);

        output
    }

    // ================================================================
    // Webhooks (v4.6)
    // ================================================================

    pub(crate) async fn webhook_list(&self) -> String {
        let client = self.get_webhooks_client().await;
        match client.list_all_webhooks().await {
            Ok(hooks) => {
                if hooks.is_empty() {
                    return "No webhooks found.".to_string();
                }
                let mut output = format!("Found {} webhook(s):\n\n", hooks.len());
                for hook in &hooks {
                    output.push_str(&format!(
                        "* {} (system: {}, event: {}, status: {})\n  URL: {}\n",
                        hook.hook_id, hook.system, hook.event, hook.status, hook.callback_url,
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list webhooks: {}", e),
        }
    }

    pub(crate) async fn webhook_create(
        &self,
        system: String,
        event: String,
        callback_url: String,
        folder_urn: Option<String>,
    ) -> String {
        let client = self.get_webhooks_client().await;
        match client
            .create_webhook(&system, &event, &callback_url, folder_urn.as_deref())
            .await
        {
            Ok(hook) => {
                format!(
                    "Webhook created successfully!\n\nID: {}\nSystem: {}\nEvent: {}\nCallback: {}",
                    hook.hook_id, hook.system, hook.event, hook.callback_url,
                )
            }
            Err(e) => format!("Failed to create webhook: {}", e),
        }
    }

    pub(crate) async fn webhook_delete(
        &self,
        system: String,
        event: String,
        hook_id: String,
    ) -> String {
        let client = self.get_webhooks_client().await;
        match client.delete_webhook(&system, &event, &hook_id).await {
            Ok(()) => format!("Webhook {} deleted successfully.", hook_id),
            Err(e) => format!("Failed to delete webhook: {}", e),
        }
    }

    pub(crate) async fn webhook_events(&self) -> String {
        let client = self.get_webhooks_client().await;
        let events = client.available_events();
        let mut output = format!("Available webhook events ({}):\n\n", events.len());
        for (event_name, description) in events {
            output.push_str(&format!("* {} - {}\n", event_name, description));
        }
        output
    }

    pub(crate) async fn webhook_get(
        &self,
        system: String,
        event: String,
        hook_id: String,
    ) -> String {
        let client = self.get_webhooks_client().await;
        match client.get_webhook(&system, &event, &hook_id).await {
            Ok(hook) => {
                let mut output = format!(
                    "Webhook Details:\n\n* Hook ID: {}\n* System: {}\n* Event: {}\n* Callback: {}\n* Status: {}",
                    hook.hook_id, hook.system, hook.event, hook.callback_url, hook.status,
                );
                if let Some(ref created) = hook.created_date {
                    output.push_str(&format!("\n* Created: {}", created));
                }
                if let Some(ref updated) = hook.last_updated_date {
                    output.push_str(&format!("\n* Updated: {}", updated));
                }
                output
            }
            Err(e) => format!("Failed to get webhook: {}", e),
        }
    }

    pub(crate) async fn webhook_update(
        &self,
        system: String,
        event: String,
        hook_id: String,
        callback_url: Option<String>,
        status: Option<String>,
        filter: Option<String>,
    ) -> String {
        let client = self.get_webhooks_client().await;
        let request = UpdateWebhookRequest {
            callback_url,
            status,
            filter,
        };
        match client
            .update_webhook(&system, &event, &hook_id, request)
            .await
        {
            Ok(hook) => {
                format!(
                    "Webhook updated successfully!\n\nID: {}\nSystem: {}\nEvent: {}\nCallback: {}\nStatus: {}",
                    hook.hook_id, hook.system, hook.event, hook.callback_url, hook.status,
                )
            }
            Err(e) => format!("Failed to update webhook: {}", e),
        }
    }

    // ================================================================
    // Design Automation (v4.6)
    // ================================================================

    pub(crate) async fn da_engines_list(&self) -> String {
        let client = self.get_da_client().await;
        match client.list_engines().await {
            Ok(engines) => {
                if engines.is_empty() {
                    return "No engines found.".to_string();
                }
                let mut output = format!("Available engines ({}):\n\n", engines.len());
                for engine in &engines {
                    output.push_str(&format!("* {}\n", engine));
                }
                output
            }
            Err(e) => format!("Failed to list engines: {}", e),
        }
    }

    pub(crate) async fn da_appbundles_list(&self) -> String {
        let client = self.get_da_client().await;
        match client.list_appbundles().await {
            Ok(bundles) => {
                if bundles.is_empty() {
                    return "No appbundles found.".to_string();
                }
                let mut output = format!("AppBundles ({}):\n\n", bundles.len());
                for bundle in &bundles {
                    output.push_str(&format!("* {}\n", bundle));
                }
                output
            }
            Err(e) => format!("Failed to list appbundles: {}", e),
        }
    }

    pub(crate) async fn da_activities_list(&self) -> String {
        let client = self.get_da_client().await;
        match client.list_activities().await {
            Ok(activities) => {
                if activities.is_empty() {
                    return "No activities found.".to_string();
                }
                let mut output = format!("Activities ({}):\n\n", activities.len());
                for activity in &activities {
                    output.push_str(&format!("* {}\n", activity));
                }
                output
            }
            Err(e) => format!("Failed to list activities: {}", e),
        }
    }

    pub(crate) async fn da_workitem_create(
        &self,
        activity_id: String,
        arguments: std::collections::HashMap<String, raps_da::WorkItemArgument>,
    ) -> String {
        let client = self.get_da_client().await;
        if arguments.is_empty() {
            return "Error: workitem requires at least one argument (input/output mappings).\n\
                    Example arguments: {\"input\": {\"url\": \"https://...\"}, \"output\": {\"url\": \"https://...\", \"verb\": \"put\"}}\n\
                    Use da_activities_list to see the activity's expected arguments."
                .to_string();
        }
        match client.create_workitem(&activity_id, arguments).await {
            Ok(item) => {
                format!(
                    "Workitem created!\n\nID: {}\nStatus: {}\nActivity: {}",
                    item.id, item.status, activity_id,
                )
            }
            Err(e) => format!("Failed to create workitem: {}", e),
        }
    }

    pub(crate) async fn da_workitem_status(&self, workitem_id: String) -> String {
        let client = self.get_da_client().await;
        match client.get_workitem_status(&workitem_id).await {
            Ok(item) => {
                let mut output = format!(
                    "Workitem: {}\nStatus: {}\nProgress: {}",
                    item.id,
                    item.status,
                    item.progress.as_deref().unwrap_or("N/A"),
                );
                if let Some(ref report_url) = item.report_url {
                    output.push_str(&format!("\nReport: {}", report_url));
                }
                output
            }
            Err(e) => format!("Failed to get workitem status: {}", e),
        }
    }

    pub(crate) async fn da_workitems_list(&self) -> String {
        let client = self.get_da_client().await;
        match client.list_workitems().await {
            Ok(workitems) => {
                if workitems.is_empty() {
                    return "No workitems found.".to_string();
                }
                let mut output = format!("Workitems ({}):\n\n", workitems.len());
                for item in &workitems {
                    output.push_str(&format!(
                        "* {} - Status: {} | Progress: {}\n",
                        item.id,
                        item.status,
                        item.progress.as_deref().unwrap_or("N/A"),
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list workitems: {}", e),
        }
    }

    // ================================================================
    // Reality Capture
    // ================================================================

    pub(crate) async fn reality_create(
        &self,
        name: String,
        scene_type: Option<String>,
        format: Option<String>,
    ) -> String {
        let client = self.get_reality_client().await;

        let st = match scene_type.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("aerial") => raps_reality::SceneType::Aerial,
            Some("object") | None => raps_reality::SceneType::Object,
            Some(other) => {
                return format!(
                    "Invalid scene_type '{}'. Valid values: aerial, object",
                    other
                );
            }
        };

        let fmt = match format.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("rcm") | None => raps_reality::OutputFormat::Rcm,
            Some("rcs") => raps_reality::OutputFormat::Rcs,
            Some("obj") => raps_reality::OutputFormat::Obj,
            Some("fbx") => raps_reality::OutputFormat::Fbx,
            Some("ortho") => raps_reality::OutputFormat::Ortho,
            Some(other) => {
                return format!(
                    "Invalid format '{}'. Valid values: rcm, rcs, obj, fbx, ortho",
                    other
                );
            }
        };

        match client.create_photoscene(&name, st, fmt).await {
            Ok(scene) => {
                format!(
                    "Photoscene created!\n\nID: {}\nName: {}\nScene Type: {}\nFormat: {}\nStatus: {}",
                    scene.photoscene_id,
                    scene.name.as_deref().unwrap_or("N/A"),
                    scene.scene_type.as_deref().unwrap_or("N/A"),
                    scene.convert_format.as_deref().unwrap_or("N/A"),
                    scene.status.as_deref().unwrap_or("N/A"),
                )
            }
            Err(e) => format!("Failed to create photoscene: {}", e),
        }
    }

    pub(crate) async fn reality_process(&self, photoscene_id: String) -> String {
        let client = self.get_reality_client().await;
        match client.start_processing(&photoscene_id).await {
            Ok(()) => format!(
                "Processing started for photoscene '{}'.\n\nUse reality_status to check progress.",
                photoscene_id
            ),
            Err(e) => format!("Failed to start processing: {}", e),
        }
    }

    pub(crate) async fn reality_status(&self, photoscene_id: String) -> String {
        let client = self.get_reality_client().await;
        match client.get_progress(&photoscene_id).await {
            Ok(progress) => {
                format!(
                    "Photoscene: {}\nProgress: {}%\nMessage: {}\nStatus: {}",
                    progress.photoscene_id,
                    progress.progress,
                    progress.progress_msg.as_deref().unwrap_or("N/A"),
                    progress.status.as_deref().unwrap_or("N/A"),
                )
            }
            Err(e) => format!("Failed to get photoscene status: {}", e),
        }
    }

    pub(crate) async fn reality_result(
        &self,
        photoscene_id: String,
        format: Option<String>,
    ) -> String {
        let client = self.get_reality_client().await;

        let fmt = match format.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("rcm") | None => raps_reality::OutputFormat::Rcm,
            Some("rcs") => raps_reality::OutputFormat::Rcs,
            Some("obj") => raps_reality::OutputFormat::Obj,
            Some("fbx") => raps_reality::OutputFormat::Fbx,
            Some("ortho") => raps_reality::OutputFormat::Ortho,
            Some(other) => {
                return format!(
                    "Invalid format '{}'. Valid values: rcm, rcs, obj, fbx, ortho",
                    other
                );
            }
        };

        match client.get_result(&photoscene_id, fmt).await {
            Ok(result) => {
                let mut output = format!(
                    "Photoscene: {}\nProgress: {}%",
                    result.photoscene_id, result.progress,
                );
                if let Some(ref link) = result.scene_link {
                    output.push_str(&format!("\nDownload: {}", link));
                }
                if let Some(ref size) = result.file_size {
                    output.push_str(&format!("\nFile Size: {} bytes", size));
                }
                output
            }
            Err(e) => format!("Failed to get photoscene result: {}", e),
        }
    }

    pub(crate) async fn reality_delete(&self, photoscene_id: String) -> String {
        let client = self.get_reality_client().await;
        match client.delete_photoscene(&photoscene_id).await {
            Ok(()) => format!("Photoscene '{}' deleted successfully.", photoscene_id),
            Err(e) => format!("Failed to delete photoscene: {}", e),
        }
    }

    pub(crate) async fn reality_formats(&self) -> String {
        let formats = raps_reality::OutputFormat::all();
        let mut output = format!("Available output formats ({}):\n\n", formats.len());
        for fmt in &formats {
            output.push_str(&format!("* {} - {}\n", fmt, fmt.description()));
        }
        output
    }

    pub(crate) async fn reality_list(&self) -> String {
        let client = self.get_reality_client().await;
        match client.list_photoscenes().await {
            Ok(photoscenes) => {
                if photoscenes.is_empty() {
                    return "No photoscenes found.".to_string();
                }
                let mut output = format!("Photoscenes ({}):\n\n", photoscenes.len());
                for scene in &photoscenes {
                    output.push_str(&format!(
                        "* {} - Name: {} | Type: {} | Status: {} | Progress: {}\n",
                        scene.photoscene_id,
                        scene.name.as_deref().unwrap_or("N/A"),
                        scene.scene_type.as_deref().unwrap_or("N/A"),
                        scene.status.as_deref().unwrap_or("N/A"),
                        scene.progress.as_deref().unwrap_or("N/A"),
                    ));
                }
                output
            }
            Err(e) => format!("Failed to list photoscenes: {}", e),
        }
    }
}
