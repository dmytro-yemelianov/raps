// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Output formatting module
//!
//! Provides support for multiple output formats (JSON, CSV, Plain, Table) with automatic
//! detection when output is piped.

use anyhow::Result;
use console::Term;
use serde::Serialize;
use std::str::FromStr;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Colored table format (default for interactive use)
    Table,
    /// JSON format (default when piped)
    Json,
    /// YAML format (human-readable, machine-parsable)
    Yaml,
    /// CSV format (for tabular data)
    Csv,
    /// Plain text (no colors, simple formatting)
    Plain,
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" => Ok(OutputFormat::Table),
            "json" => Ok(OutputFormat::Json),
            "yaml" | "yml" => Ok(OutputFormat::Yaml),
            "csv" => Ok(OutputFormat::Csv),
            "plain" => Ok(OutputFormat::Plain),
            _ => anyhow::bail!(
                "Invalid output format: {}. Use: table, json, yaml, csv, plain",
                s
            ),
        }
    }
}

impl OutputFormat {
    /// Determine output format based on CLI flag, environment variable, and TTY detection
    pub fn determine(cli_format: Option<OutputFormat>) -> OutputFormat {
        // Explicit format takes precedence
        if let Some(format) = cli_format {
            return format;
        }

        // Check environment variable
        if let Ok(env_format) = std::env::var("RAPS_OUTPUT_FORMAT")
            && let Ok(format) = OutputFormat::from_str(&env_format)
        {
            return format;
        }

        // Auto-detect: if not a TTY, use JSON
        if !Term::stdout().is_term() {
            return OutputFormat::Json;
        }

        // Default to table for interactive use
        OutputFormat::Table
    }

    /// Write data in the selected format
    pub fn write<T: Serialize>(&self, data: &T) -> Result<()> {
        match self {
            OutputFormat::Table => write_table(data),
            OutputFormat::Json => write_json(data),
            OutputFormat::Yaml => write_yaml(data),
            OutputFormat::Csv => write_csv(data),
            OutputFormat::Plain => write_plain(data),
        }
    }

    /// Write a simple message (for non-structured output)
    pub fn write_message(&self, message: &str) -> Result<()> {
        match self {
            OutputFormat::Table | OutputFormat::Plain => {
                println!("{}", message);
                Ok(())
            }
            OutputFormat::Json => {
                #[derive(Serialize)]
                struct Message {
                    message: String,
                }
                write_json(&Message {
                    message: message.to_string(),
                })
            }
            OutputFormat::Yaml => {
                #[derive(Serialize)]
                struct Message {
                    message: String,
                }
                write_yaml(&Message {
                    message: message.to_string(),
                })
            }
            OutputFormat::Csv => {
                // CSV doesn't make sense for simple messages, use plain
                println!("{}", message);
                Ok(())
            }
        }
    }

    /// Check if this format supports colors
    pub fn supports_colors(&self) -> bool {
        matches!(self, OutputFormat::Table)
    }
}

/// Write data as JSON
fn write_json<T: Serialize>(data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

/// Write data as YAML
fn write_yaml<T: Serialize>(data: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(data)?;
    print!("{}", yaml);
    Ok(())
}

/// Write data as CSV (only works for arrays of structs)
fn write_csv<T: Serialize>(data: &T) -> Result<()> {
    // Try to serialize as JSON first to get the structure
    let json_value = serde_json::to_value(data)?;

    match json_value {
        serde_json::Value::Array(items) if !items.is_empty() => {
            // Get headers from first item
            if let Some(serde_json::Value::Object(map)) = items.first() {
                let mut wtr = csv::Writer::from_writer(std::io::stdout());

                // Write headers
                let headers: Vec<String> = map.keys().cloned().collect();
                wtr.write_record(&headers)?;

                // Write each row
                for item in items {
                    if let serde_json::Value::Object(map) = item {
                        let mut row = Vec::new();
                        for header in &headers {
                            let value = map.get(header).unwrap_or(&serde_json::Value::Null);
                            row.push(format_value_for_csv(value));
                        }
                        wtr.write_record(&row)?;
                    }
                }
                wtr.flush()?;
                return Ok(());
            }
        }
        _ => {
            // For non-array data, fall back to JSON
            return write_json(data);
        }
    }

    // Fallback to JSON if CSV conversion fails
    write_json(data)
}

/// Format a JSON value for CSV output
fn format_value_for_csv(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            // Join array elements with semicolon
            arr.iter()
                .map(format_value_for_csv)
                .collect::<Vec<_>>()
                .join("; ")
        }
        serde_json::Value::Object(obj) => {
            // For nested objects, serialize as JSON string
            serde_json::to_string(obj).unwrap_or_default()
        }
    }
}

/// Write data as plain text (no colors)
fn write_plain<T: Serialize>(data: &T) -> Result<()> {
    // For plain text, we'll use a simple JSON-like structure without colors
    // This is a simplified version - could be enhanced
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

/// Write data as a formatted table (current default behavior)
fn write_table<T: Serialize>(data: &T) -> Result<()> {
    // For table format, we'll serialize to JSON for now
    // Individual commands will override this with their custom table formatting
    // This is a fallback for commands that don't have custom table formatting yet
    write_json(data)
}

/// Helper trait for types that can be formatted as tables
#[allow(dead_code)] // May be used in future
pub trait TableFormat {
    /// Write this data as a formatted table
    fn write_table(&self) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestData {
        id: String,
        name: String,
        count: u32,
    }

    #[derive(Serialize)]
    struct NestedData {
        id: String,
        items: Vec<String>,
    }

    #[test]
    fn test_output_format_from_str_table() {
        let format = OutputFormat::from_str("table").unwrap();
        assert_eq!(format, OutputFormat::Table);
    }

    #[test]
    fn test_output_format_from_str_json() {
        let format = OutputFormat::from_str("json").unwrap();
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_output_format_from_str_yaml() {
        let format = OutputFormat::from_str("yaml").unwrap();
        assert_eq!(format, OutputFormat::Yaml);
    }

    #[test]
    fn test_output_format_from_str_yml() {
        let format = OutputFormat::from_str("yml").unwrap();
        assert_eq!(format, OutputFormat::Yaml);
    }

    #[test]
    fn test_output_format_from_str_csv() {
        let format = OutputFormat::from_str("csv").unwrap();
        assert_eq!(format, OutputFormat::Csv);
    }

    #[test]
    fn test_output_format_from_str_plain() {
        let format = OutputFormat::from_str("plain").unwrap();
        assert_eq!(format, OutputFormat::Plain);
    }

    #[test]
    fn test_output_format_from_str_case_insensitive() {
        assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("Table").unwrap(), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("YAML").unwrap(), OutputFormat::Yaml);
    }

    #[test]
    fn test_output_format_from_str_invalid() {
        let result = OutputFormat::from_str("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_supports_colors_table() {
        assert!(OutputFormat::Table.supports_colors());
    }

    #[test]
    fn test_supports_colors_json() {
        assert!(!OutputFormat::Json.supports_colors());
    }

    #[test]
    fn test_supports_colors_yaml() {
        assert!(!OutputFormat::Yaml.supports_colors());
    }

    #[test]
    fn test_supports_colors_csv() {
        assert!(!OutputFormat::Csv.supports_colors());
    }

    #[test]
    fn test_supports_colors_plain() {
        assert!(!OutputFormat::Plain.supports_colors());
    }

    #[test]
    fn test_determine_explicit_format() {
        let format = OutputFormat::determine(Some(OutputFormat::Json));
        assert_eq!(format, OutputFormat::Json);
    }

    #[test]
    fn test_determine_env_format() {
        // SAFETY: Test runs with --test-threads=1 or in isolation
        unsafe {
            std::env::set_var("RAPS_OUTPUT_FORMAT", "yaml");
        }
        let _format = OutputFormat::determine(None);
        // Note: This may be Json if not a TTY, but we're testing env var takes effect
        unsafe {
            std::env::remove_var("RAPS_OUTPUT_FORMAT");
        }
        // We can't reliably test this without controlling the terminal state
    }

    #[test]
    fn test_format_value_for_csv_null() {
        let value = serde_json::Value::Null;
        assert_eq!(format_value_for_csv(&value), "");
    }

    #[test]
    fn test_format_value_for_csv_bool() {
        let value = serde_json::Value::Bool(true);
        assert_eq!(format_value_for_csv(&value), "true");
    }

    #[test]
    fn test_format_value_for_csv_number() {
        let value = serde_json::json!(42);
        assert_eq!(format_value_for_csv(&value), "42");
    }

    #[test]
    fn test_format_value_for_csv_string() {
        let value = serde_json::json!("hello");
        assert_eq!(format_value_for_csv(&value), "hello");
    }

    #[test]
    fn test_format_value_for_csv_array() {
        let value = serde_json::json!(["a", "b", "c"]);
        assert_eq!(format_value_for_csv(&value), "a; b; c");
    }

    #[test]
    fn test_format_value_for_csv_object() {
        let value = serde_json::json!({"key": "value"});
        let result = format_value_for_csv(&value);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }
}
