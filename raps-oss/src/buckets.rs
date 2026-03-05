// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Bucket operations for the OSS API.

use anyhow::{Context, Result};
use raps_kernel::error::RapsError;

use crate::OssClient;
use crate::types::*;

impl OssClient {
    /// Create a new bucket
    pub async fn create_bucket(
        &self,
        bucket_key: &str,
        policy: RetentionPolicy,
        region: Region,
    ) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets", self.config.oss_url());

        let request = CreateBucketRequest {
            bucket_key: bucket_key.to_string(),
            policy_key: policy.to_string(),
        };

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client
                .post(&url)
                .bearer_auth(&token)
                .header("x-ads-region", region.to_string())
                .header("Content-Type", "application/json")
                .json(&request)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let bucket: Bucket = response
            .json()
            .await
            .context("Failed to parse bucket response")?;

        Ok(bucket)
    }

    /// List all buckets from all regions
    ///
    /// Queries US and EMEA regions concurrently with a per-region timeout
    /// to avoid hanging when one region is slow or unreachable.
    pub async fn list_buckets(&self) -> Result<Vec<BucketItem>> {
        use std::time::Duration;

        let per_region_timeout = Duration::from_secs(
            std::env::var("RAPS_REGION_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        );

        // Query both regions concurrently
        let (us_result, emea_result) = tokio::join!(
            tokio::time::timeout(per_region_timeout, self.list_buckets_in_region(Region::US)),
            tokio::time::timeout(
                per_region_timeout,
                self.list_buckets_in_region(Region::EMEA)
            ),
        );

        let mut all_buckets = Vec::new();

        // Process US region
        match us_result {
            Ok(Ok(mut buckets)) => {
                for bucket in &mut buckets {
                    bucket.region = Some("US".to_string());
                }
                all_buckets.extend(buckets);
            }
            Ok(Err(e)) => {
                tracing::warn!(region = "US", error = %e, "Failed to list buckets");
            }
            Err(_) => {
                tracing::warn!(region = "US", timeout = ?per_region_timeout, "Region listing timed out");
            }
        }

        // Process EMEA region
        match emea_result {
            Ok(Ok(mut buckets)) => {
                for bucket in &mut buckets {
                    bucket.region = Some("EMEA".to_string());
                }
                all_buckets.extend(buckets);
            }
            Ok(Err(e)) => {
                tracing::warn!(region = "EMEA", error = %e, "Failed to list buckets");
            }
            Err(_) => {
                tracing::warn!(region = "EMEA", timeout = ?per_region_timeout, "Region listing timed out");
            }
        }

        Ok(all_buckets)
    }

    /// List buckets from all regions, returning results per-region as they complete.
    ///
    /// Uses `tokio::select!` so whichever region finishes first yields its
    /// `RegionResult` immediately. The caller can display partial results
    /// while waiting for slower regions.
    pub async fn list_buckets_streaming(&self) -> Vec<RegionResult> {
        use std::time::{Duration, Instant};

        let per_region_timeout = Duration::from_secs(
            std::env::var("RAPS_REGION_TIMEOUT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
        );

        let us_start = Instant::now();
        let emea_start = Instant::now();

        let mut us_fut = std::pin::pin!(tokio::time::timeout(
            per_region_timeout,
            self.list_buckets_in_region(Region::US),
        ));
        let mut emea_fut = std::pin::pin!(tokio::time::timeout(
            per_region_timeout,
            self.list_buckets_in_region(Region::EMEA),
        ));

        let mut results = Vec::with_capacity(2);
        let mut us_done = false;
        let mut emea_done = false;

        while !us_done || !emea_done {
            tokio::select! {
                us_result = &mut us_fut, if !us_done => {
                    us_done = true;
                    let elapsed = us_start.elapsed();
                    let buckets = match us_result {
                        Ok(Ok(mut buckets)) => {
                            for b in &mut buckets {
                                b.region = Some("US".to_string());
                            }
                            Ok(buckets)
                        }
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(anyhow::anyhow!(
                            "US region timed out after {:?}",
                            per_region_timeout
                        )),
                    };
                    results.push(RegionResult { region: Region::US, buckets, elapsed });
                }
                emea_result = &mut emea_fut, if !emea_done => {
                    emea_done = true;
                    let elapsed = emea_start.elapsed();
                    let buckets = match emea_result {
                        Ok(Ok(mut buckets)) => {
                            for b in &mut buckets {
                                b.region = Some("EMEA".to_string());
                            }
                            Ok(buckets)
                        }
                        Ok(Err(e)) => Err(e),
                        Err(_) => Err(anyhow::anyhow!(
                            "EMEA region timed out after {:?}",
                            per_region_timeout
                        )),
                    };
                    results.push(RegionResult { region: Region::EMEA, buckets, elapsed });
                }
            }
        }

        results
    }

    /// List buckets in a specific region
    async fn list_buckets_in_region(&self, region: Region) -> Result<Vec<BucketItem>> {
        let token = self.auth.get_token().await?;
        let mut buckets = Vec::new();
        let mut start_at: Option<String> = None;
        const MAX_PAGES: usize = 100;

        for _page in 0..MAX_PAGES {
            let mut url = format!("{}/buckets", self.config.oss_url());
            if let Some(ref start) = start_at {
                url = format!("{}?startAt={}", url, start);
            }

            let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
                self.http_client
                    .get(&url)
                    .bearer_auth(&token)
                    .header("x-ads-region", region.to_string())
            })
            .await?;

            tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

            if !response.status().is_success() {
                return Err(RapsError::from_response(response).await.into());
            }

            let buckets_response: BucketsResponse = response
                .json()
                .await
                .context("Failed to parse buckets response")?;

            buckets.extend(buckets_response.items);

            if buckets_response.next.is_none() {
                break;
            }
            start_at = buckets_response.next;
        }

        Ok(buckets)
    }

    /// Get bucket details
    pub async fn get_bucket_details(&self, bucket_key: &str) -> Result<Bucket> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}/details", self.config.oss_url(), bucket_key);

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.get(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        let bucket: Bucket = response
            .json()
            .await
            .context("Failed to parse bucket details")?;

        Ok(bucket)
    }

    /// Delete a bucket
    pub async fn delete_bucket(&self, bucket_key: &str) -> Result<()> {
        let token = self.auth.get_token().await?;
        let url = format!("{}/buckets/{}", self.config.oss_url(), bucket_key);

        let response = raps_kernel::http::send_with_retry(&self.config.http_config, || {
            self.http_client.delete(&url).bearer_auth(&token)
        })
        .await?;

        tracing::info!(status = response.status().as_u16(), url = %raps_kernel::logging::redact_secrets(&url), "HTTP response");

        if !response.status().is_success() {
            return Err(RapsError::from_response(response).await.into());
        }

        Ok(())
    }
}
