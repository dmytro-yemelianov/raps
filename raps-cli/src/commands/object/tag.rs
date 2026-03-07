// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! `raps object tag` — manage OSS custom attributes (object metadata).
//!
//! OSS supports arbitrary key/value metadata on objects via the
//! `/buckets/{bucket}/objects/{key}/details` and the `x-ads-meta-*` headers
//! on upload. Since there is no dedicated "set metadata" endpoint in the public
//! OSS v2 API we store tags in a side-car JSON file
//! `<object-key>.raps-tags.json` in the same bucket. This is transparent and
//! interoperable: the side-car is a plain JSON file any client can read.
//!
//! The `search` sub-command performs a client-side filter: it lists all tag
//! side-car files and returns those whose values match the requested attribute.

use anyhow::{Context, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::output::OutputFormat;
use raps_oss::OssClient;

// ── Side-car format ───────────────────────────────────────────────────────────

const TAG_SUFFIX: &str = ".raps-tags.json";

#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
struct TagStore {
    /// Arbitrary string key → value pairs.
    pub attrs: HashMap<String, String>,
}

fn sidecar_key(object_key: &str) -> String {
    format!("{}{}", object_key, TAG_SUFFIX)
}

/// Load the tag store for an object, returning an empty store if none exists.
async fn load_tags(client: &OssClient, bucket: &str, object_key: &str) -> Result<TagStore> {
    let sidecar = sidecar_key(object_key);

    // Download sidecar to an in-memory writer
    let mut buf: Vec<u8> = Vec::new();
    match client
        .download_object_to_writer(bucket, &sidecar, &mut buf)
        .await
    {
        Ok(()) => {
            let store: TagStore = serde_json::from_slice(&buf)
                .with_context(|| format!("Failed to parse tag store for '{}'", object_key))?;
            Ok(store)
        }
        Err(e) => {
            // Treat 404 (object not found) as empty
            let msg = e.to_string();
            if msg.contains("404") || msg.contains("Not Found") || msg.contains("not found") {
                Ok(TagStore::default())
            } else {
                Err(e)
            }
        }
    }
}

/// Persist the tag store back to the bucket as a JSON side-car.
async fn save_tags(
    client: &OssClient,
    bucket: &str,
    object_key: &str,
    store: &TagStore,
) -> Result<()> {
    let sidecar = sidecar_key(object_key);
    let json = serde_json::to_vec_pretty(store)?;

    // Write to a temp file then upload
    let mut tmp = tempfile::NamedTempFile::new()?;
    {
        use std::io::Write;
        tmp.write_all(&json)?;
    }
    client
        .upload_object(bucket, &sidecar, tmp.path())
        .await
        .with_context(|| format!("Failed to save tags for '{}'", object_key))?;
    Ok(())
}

// ── Subcommand handlers ───────────────────────────────────────────────────────

/// `raps object tag set <bucket> <key> <attr>=<value>…`
pub(super) async fn tag_set(
    client: &OssClient,
    bucket: String,
    object_key: String,
    attrs: Vec<String>,
    output_format: OutputFormat,
) -> Result<()> {
    if attrs.is_empty() {
        anyhow::bail!("Provide at least one attr=value pair");
    }

    let mut store = load_tags(client, &bucket, &object_key).await?;

    for pair in &attrs {
        let (k, v) = parse_attr(pair)?;
        store.attrs.insert(k.to_string(), v.to_string());
    }

    save_tags(client, &bucket, &object_key, &store).await?;

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&store)?);
        }
        _ => {
            println!(
                "{} Updated {} attribute(s) on '{}'",
                "✓".green(),
                attrs.len(),
                object_key
            );
            for pair in &attrs {
                println!("  {}", pair.cyan());
            }
        }
    }
    Ok(())
}

/// `raps object tag get <bucket> <key>`
pub(super) async fn tag_get(
    client: &OssClient,
    bucket: String,
    object_key: String,
    output_format: OutputFormat,
) -> Result<()> {
    let store = load_tags(client, &bucket, &object_key).await?;

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&store)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            wtr.write_record(["attribute", "value"])?;
            for (k, v) in &store.attrs {
                wtr.write_record([k.as_str(), v.as_str()])?;
            }
            wtr.flush()?;
        }
        _ => {
            if store.attrs.is_empty() {
                println!("No custom attributes on '{}'.", object_key);
            } else {
                println!("{}", format!("Attributes for '{}':", object_key).bold());
                let mut pairs: Vec<_> = store.attrs.iter().collect();
                pairs.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in pairs {
                    println!("  {:30}  {}", k.cyan(), v);
                }
            }
        }
    }
    Ok(())
}

/// `raps object tag delete <bucket> <key> <attr>`
pub(super) async fn tag_delete(
    client: &OssClient,
    bucket: String,
    object_key: String,
    attr: String,
    output_format: OutputFormat,
) -> Result<()> {
    let mut store = load_tags(client, &bucket, &object_key).await?;

    if store.attrs.remove(&attr).is_none() {
        anyhow::bail!("Attribute '{}' not found on '{}'", attr, object_key);
    }

    save_tags(client, &bucket, &object_key, &store).await?;

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&store)?);
        }
        _ => {
            println!(
                "{} Removed attribute '{}' from '{}'",
                "✓".green(),
                attr.cyan(),
                object_key
            );
        }
    }
    Ok(())
}

/// `raps object tag search <bucket> <attr>=<value>`
///
/// Lists all objects whose side-car contains the given attribute matching the
/// given value. This performs a client-side scan.
pub(super) async fn tag_search(
    client: &OssClient,
    bucket: String,
    filter: String,
    output_format: OutputFormat,
) -> Result<()> {
    let (attr, value) = parse_attr(&filter)?;

    eprintln!(
        "{} Scanning bucket '{}' for {}={}…",
        "→".cyan(),
        bucket,
        attr,
        value
    );

    // List all objects — look for side-car files
    let objects = client.list_objects(&bucket).await?;
    let sidecars: Vec<_> = objects
        .iter()
        .filter(|o| o.object_key.ends_with(TAG_SUFFIX))
        .collect();

    if sidecars.is_empty() {
        println!("No tagged objects found in bucket '{}'.", bucket);
        return Ok(());
    }

    let mut matches: Vec<(String, String)> = Vec::new(); // (original_key, value)

    for sidecar_obj in &sidecars {
        let mut buf: Vec<u8> = Vec::new();
        if client
            .download_object_to_writer(&bucket, &sidecar_obj.object_key, &mut buf)
            .await
            .is_err()
        {
            continue;
        }
        let Ok(store) = serde_json::from_slice::<TagStore>(&buf) else {
            continue;
        };
        if let Some(v) = store.attrs.get(attr) {
            if v == value {
                // Derive original object key by stripping suffix
                let original = sidecar_obj
                    .object_key
                    .strip_suffix(TAG_SUFFIX)
                    .unwrap_or(&sidecar_obj.object_key)
                    .to_string();
                matches.push((original, v.clone()));
            }
        }
    }

    match output_format {
        OutputFormat::Json => {
            let out: Vec<serde_json::Value> = matches
                .iter()
                .map(|(k, v)| {
                    serde_json::json!({"object_key": k, attr: v})
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out)?);
        }
        OutputFormat::Csv => {
            let mut wtr = csv::Writer::from_writer(std::io::stdout());
            wtr.write_record(["object_key", attr])?;
            for (k, v) in &matches {
                wtr.write_record([k.as_str(), v.as_str()])?;
            }
            wtr.flush()?;
        }
        _ => {
            if matches.is_empty() {
                println!(
                    "No objects found with {}={} in bucket '{}'.",
                    attr, value, bucket
                );
            } else {
                println!(
                    "{}",
                    format!("Objects with {}={}: {}", attr, value, matches.len()).bold()
                );
                for (k, _) in &matches {
                    println!("  {}", k.cyan());
                }
            }
        }
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse `attr=value` into `(attr, value)`.
fn parse_attr(s: &str) -> Result<(&str, &str)> {
    let idx = s
        .find('=')
        .ok_or_else(|| anyhow::anyhow!("Expected attr=value, got '{}'", s))?;
    Ok((&s[..idx], &s[idx + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_attr_ok() {
        let (k, v) = parse_attr("discipline=structural").unwrap();
        assert_eq!(k, "discipline");
        assert_eq!(v, "structural");
    }

    #[test]
    fn test_parse_attr_value_contains_equals() {
        let (k, v) = parse_attr("url=https://example.com/path?a=1").unwrap();
        assert_eq!(k, "url");
        assert_eq!(v, "https://example.com/path?a=1");
    }

    #[test]
    fn test_parse_attr_no_equals() {
        assert!(parse_attr("nodivider").is_err());
    }

    #[test]
    fn test_sidecar_key() {
        assert_eq!(
            sidecar_key("models/arch.rvt"),
            "models/arch.rvt.raps-tags.json"
        );
    }
}
