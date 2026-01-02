// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Object operations

use raps_kernel::{AuthClient, Config, HttpClient, Result, RapsError, BucketKey, ObjectKey};
use crate::types::*;

/// Object client for OSS operations
pub struct ObjectClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl ObjectClient {
    /// Create new object client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// List objects in a bucket
    pub async fn list_objects(&self, bucket_key: &BucketKey) -> Result<Vec<ObjectItem>> {
        let token = self.auth.get_token().await?;
        let mut all_objects = Vec::new();
        let mut start_at: Option<String> = None;

        loop {
            let mut url = format!("{}/buckets/{}/objects", self.base_url, bucket_key.as_str());
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = self
                .http
                .inner()
                .get(&url)
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| RapsError::Network {
                    message: "Failed to list objects".to_string(),
                    source: Some(e),
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(RapsError::Api {
                    message: format!("Failed to list objects ({}): {}", status, error_text),
                    status: Some(status.as_u16()),
                    source: None,
                });
            }

            let response_text = response
                .text()
                .await
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to read objects response: {}", e),
                })?;

            let objects_response: ObjectsResponse = serde_json::from_str(&response_text)
                .map_err(|e| RapsError::Internal {
                    message: format!("Failed to parse objects response: {}", e),
                })?;

            all_objects.extend(objects_response.items);

            if objects_response.next.is_none() {
                break;
            }
            start_at = objects_response.next;
        }

        Ok(all_objects)
    }

    /// Delete an object from a bucket
    pub async fn delete_object(
        &self,
        bucket_key: &BucketKey,
        object_key: &ObjectKey,
    ) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!(
            "{}/buckets/{}/objects/{}",
            self.base_url,
            bucket_key.as_str(),
            urlencoding::encode(object_key.as_str())
        );

        let response = self
            .http
            .inner()
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to delete object".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to delete object ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        Ok(())
    }

    /// Generate a base64-encoded URN for an object
    pub fn get_urn(&self, bucket_key: &BucketKey, object_key: &ObjectKey) -> String {
        use base64::Engine;
        let object_id = format!(
            "urn:adsk.objects:os.object:{}/{}",
            bucket_key.as_str(),
            object_key.as_str()
        );
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(object_id)
    }
}
