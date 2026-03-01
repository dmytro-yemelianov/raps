// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Automatic APS region detection and routing.
//!
//! Detects whether a resource lives in the US or EMEA region from hub
//! metadata, URN prefixes, or bucket details. Injects the correct
//! `x-ads-region` header and routes to the right endpoint.

use dashmap::DashMap;

/// APS deployment region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Region {
    US,
    EMEA,
}

impl Region {
    /// Header value for the `x-ads-region` header.
    pub fn header_value(self) -> &'static str {
        match self {
            Region::US => "US",
            Region::EMEA => "EMEA",
        }
    }
}

impl std::fmt::Display for Region {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.header_value())
    }
}

impl std::str::FromStr for Region {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "US" => Ok(Region::US),
            "EMEA" | "EU" => Ok(Region::EMEA),
            _ => anyhow::bail!("Unknown region '{}'. Use US or EMEA.", s),
        }
    }
}

// ---------------------------------------------------------------------------
// Detection logic
// ---------------------------------------------------------------------------

/// Detect region from hub metadata JSON.
///
/// Looks for `extension.data.region` in the hub attributes.
pub fn detect_from_hub(hub_json: &serde_json::Value) -> Option<Region> {
    let region_str = hub_json
        .pointer("/attributes/extension/data/region")
        .or_else(|| hub_json.pointer("/extension/data/region"))
        .or_else(|| hub_json.pointer("/data/region"))
        .and_then(|v| v.as_str())?;

    match region_str.to_uppercase().as_str() {
        "US" => Some(Region::US),
        "EMEA" | "EU" => Some(Region::EMEA),
        _ => None,
    }
}

/// Detect region from a base64-encoded URN.
///
/// EMEA URNs typically contain EMEA-specific bucket prefixes or patterns.
/// This is a heuristic — hub metadata is more reliable.
pub fn detect_from_urn(urn: &str) -> Option<Region> {
    // Decode the URN to inspect the bucket/object path
    let decoded = if urn.starts_with("urn:adsk") {
        // Already readable
        urn.to_string()
    } else {
        // Try base64 decode
        use base64::Engine;
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(urn)
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_default()
    };

    let lower = decoded.to_lowercase();

    // EMEA patterns in URNs
    if lower.contains("emea") || lower.contains("wipemea") || lower.contains("eu-") {
        return Some(Region::EMEA);
    }

    None // Default unknown — caller should check hub metadata
}

/// Detect region from a bucket key.
///
/// ACC/BIM360 buckets often contain region hints.
pub fn detect_from_bucket(bucket_key: &str) -> Option<Region> {
    let lower = bucket_key.to_lowercase();
    if lower.contains("emea") || lower.starts_with("wip.dm.emea") {
        Some(Region::EMEA)
    } else if lower.starts_with("wip.dm.prod") {
        Some(Region::US) // Default WIP buckets are US
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Region cache — memoizes detection results
// ---------------------------------------------------------------------------

/// Cache of detected regions, keyed by hub ID, project ID, or bucket key.
pub struct RegionCache {
    entries: DashMap<String, Region>,
}

impl RegionCache {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Get cached region for a key.
    pub fn get(&self, key: &str) -> Option<Region> {
        self.entries.get(key).map(|r| *r)
    }

    /// Cache a region for a key.
    pub fn set(&self, key: &str, region: Region) {
        self.entries.insert(key.to_string(), region);
    }

    /// Try to detect and cache region from hub metadata.
    pub fn detect_and_cache_hub(&self, hub_id: &str, hub_json: &serde_json::Value) -> Option<Region> {
        if let Some(cached) = self.get(hub_id) {
            return Some(cached);
        }
        let region = detect_from_hub(hub_json)?;
        self.set(hub_id, region);
        Some(region)
    }
}

impl Default for RegionCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

static REGION_CACHE: std::sync::OnceLock<RegionCache> = std::sync::OnceLock::new();

/// Get the global region cache.
pub fn cache() -> &'static RegionCache {
    REGION_CACHE.get_or_init(RegionCache::new)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_from_hub_metadata() {
        let hub = json!({
            "attributes": {
                "extension": {
                    "data": {
                        "region": "EMEA"
                    }
                }
            }
        });
        assert_eq!(detect_from_hub(&hub), Some(Region::EMEA));

        let hub_us = json!({
            "attributes": {
                "extension": {
                    "data": {
                        "region": "US"
                    }
                }
            }
        });
        assert_eq!(detect_from_hub(&hub_us), Some(Region::US));
    }

    #[test]
    fn test_detect_from_hub_missing() {
        let hub = json!({ "id": "b.123" });
        assert_eq!(detect_from_hub(&hub), None);
    }

    #[test]
    fn test_detect_from_bucket() {
        assert_eq!(detect_from_bucket("wip.dm.emea.123456"), Some(Region::EMEA));
        assert_eq!(detect_from_bucket("wip.dm.prod.123456"), Some(Region::US));
        assert_eq!(detect_from_bucket("my-custom-bucket"), None);
    }

    #[test]
    fn test_detect_from_urn() {
        assert_eq!(
            detect_from_urn("urn:adsk.objects:os.object:wip.dm.emea.123/file.rvt"),
            Some(Region::EMEA)
        );
        assert_eq!(
            detect_from_urn("urn:adsk.objects:os.object:mybucket/file.rvt"),
            None
        );
    }

    #[test]
    fn test_region_parse() {
        assert_eq!("US".parse::<Region>().unwrap(), Region::US);
        assert_eq!("emea".parse::<Region>().unwrap(), Region::EMEA);
        assert_eq!("EU".parse::<Region>().unwrap(), Region::EMEA);
        assert!("APAC".parse::<Region>().is_err());
    }

    #[test]
    fn test_region_cache() {
        let cache = RegionCache::new();
        assert!(cache.get("hub-123").is_none());

        cache.set("hub-123", Region::EMEA);
        assert_eq!(cache.get("hub-123"), Some(Region::EMEA));
    }
}
