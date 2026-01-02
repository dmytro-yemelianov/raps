// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bucket operations

use crate::types::*;
use raps_kernel::{AuthClient, BucketKey, Config, HttpClient, RapsError, Result};

/// Bucket client for OSS operations
pub struct BucketClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
    base_url: String,
}

impl BucketClient {
    /// Create new bucket client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config, base_url: String) -> Self {
        Self {
            http,
            auth,
            config,
            base_url,
        }
    }

    /// Create a new bucket
    pub async fn create_bucket(
        &self,
        bucket_key: &BucketKey,
        policy: RetentionPolicy,
        region: Region,
    ) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets", self.base_url);

        let request = CreateBucketRequest {
            bucket_key: bucket_key.as_str().to_string(),
            policy_key: policy.to_string(),
        };

        let response = self
            .http
            .inner()
            .post(&url)
            .bearer_auth(&token)
            .header("x-ads-region", region.to_string())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to create bucket".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to create bucket ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let bucket: Bucket = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse bucket response: {}", e),
        })?;

        Ok(bucket)
    }

    /// List all buckets from all regions
    pub async fn list_buckets(&self) -> Result<Vec<BucketItem>> {
        let mut all_buckets = Vec::new();

        // Query both US and EMEA regions
        for region in Region::all() {
            let mut region_buckets = self.list_buckets_in_region(region).await?;
            // Tag each bucket with its region
            for bucket in &mut region_buckets {
                bucket.region = Some(region.to_string());
            }
            all_buckets.extend(region_buckets);
        }

        Ok(all_buckets)
    }

    /// List buckets in a specific region
    pub async fn list_buckets_in_region(&self, region: Region) -> Result<Vec<BucketItem>> {
        let token = self.auth.get_token().await?;
        let mut buckets = Vec::new();
        let mut start_at: Option<String> = None;

        loop {
            let mut url = format!("{}/buckets", self.base_url);
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = self
                .http
                .inner()
                .get(&url)
                .bearer_auth(&token)
                .header("x-ads-region", region.to_string())
                .send()
                .await
                .map_err(|e| RapsError::Network {
                    message: "Failed to list buckets".to_string(),
                    source: Some(e),
                })?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                return Err(RapsError::Api {
                    message: format!("Failed to list buckets ({}): {}", status, error_text),
                    status: Some(status.as_u16()),
                    source: None,
                });
            }

            let buckets_response: BucketsResponse =
                response.json().await.map_err(|e| RapsError::Internal {
                    message: format!("Failed to parse buckets response: {}", e),
                })?;

            buckets.extend(buckets_response.items);

            if buckets_response.next.is_none() {
                break;
            }
            start_at = buckets_response.next;
        }

        Ok(buckets)
    }

    /// Get bucket details
    pub async fn get_bucket_details(&self, bucket_key: &BucketKey) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}/details", self.base_url, bucket_key.as_str());

        let response = self
            .http
            .inner()
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to get bucket details".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to get bucket details ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        let bucket: Bucket = response.json().await.map_err(|e| RapsError::Internal {
            message: format!("Failed to parse bucket details: {}", e),
        })?;

        Ok(bucket)
    }

    /// Delete a bucket
    pub async fn delete_bucket(&self, bucket_key: &BucketKey) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}", self.base_url, bucket_key.as_str());

        let response = self
            .http
            .inner()
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| RapsError::Network {
                message: "Failed to delete bucket".to_string(),
                source: Some(e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(RapsError::Api {
                message: format!("Failed to delete bucket ({}): {}", status, error_text),
                status: Some(status.as_u16()),
                source: None,
            });
        }

        Ok(())
    }
}
