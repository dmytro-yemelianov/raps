use super::OutputFormat;
use anyhow::Result;
use serde::Serialize;
use std::io::Write;

pub struct OutputFormatter;

impl OutputFormatter {
    pub fn print_output<T, W>(data: &T, format: OutputFormat, writer: &mut W) -> Result<()>
    where
        T: Serialize + ?Sized,
        W: Write,
    {
        match format {
            OutputFormat::Json => {
                serde_json::to_writer_pretty(&mut *writer, data)?;
                writeln!(writer)?;
            }
            OutputFormat::Yaml => {
                serde_yaml::to_writer(&mut *writer, data)?;
            }
            OutputFormat::Csv => {
                // Try to serialize as JSON first to get the structure for CSV
                let json_value = serde_json::to_value(data)?;
                write_csv(json_value, writer)?;
            }
            OutputFormat::Plain => {
                // For plain text, we'll use a simple JSON-like structure without colors
                serde_json::to_writer_pretty(&mut *writer, data)?;
                writeln!(writer)?;
            }
            OutputFormat::Table => {
                // Render a proper table from the serialized data
                let json_value = serde_json::to_value(data)?;
                write_table(json_value, writer)?;
            }
        }
        Ok(())
    }
}

fn write_csv<W: Write>(json_value: serde_json::Value, writer: &mut W) -> Result<()> {
    match json_value {
        serde_json::Value::Array(items) if !items.is_empty() => {
            // Get headers from first item
            if let Some(serde_json::Value::Object(map)) = items.first() {
                // csv::Writer takes any writer. We give it a reborrow.
                let mut wtr = csv::Writer::from_writer(&mut *writer);

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
            // For non-array data, fall back to JSON as CSV doesn't match well
            serde_json::to_writer_pretty(&mut *writer, &json_value)?;
            writeln!(writer)?;
        }
    }
    Ok(())
}

fn write_table<W: Write>(json_value: serde_json::Value, writer: &mut W) -> Result<()> {
    match json_value {
        serde_json::Value::Array(items) if !items.is_empty() => {
            if let Some(serde_json::Value::Object(first)) = items.first() {
                let headers: Vec<String> = first.keys().cloned().collect();

                // Calculate column widths (capped at 50 chars)
                let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
                for item in &items {
                    if let serde_json::Value::Object(map) = item {
                        for (i, header) in headers.iter().enumerate() {
                            let val = map.get(header).unwrap_or(&serde_json::Value::Null);
                            let val_str = format_value_for_table(val);
                            widths[i] = widths[i].max(val_str.len()).min(50);
                        }
                    }
                }

                // Header row
                let header_row: Vec<String> = headers
                    .iter()
                    .enumerate()
                    .map(|(i, h)| format!("{:<width$}", h, width = widths[i]))
                    .collect();
                writeln!(writer, "{}", header_row.join("  "))?;

                // Separator
                let separator: Vec<String> = widths.iter().map(|&w| "\u{2500}".repeat(w)).collect();
                writeln!(writer, "{}", separator.join("\u{2500}\u{2500}"))?;

                // Data rows
                for item in &items {
                    if let serde_json::Value::Object(map) = item {
                        let row: Vec<String> = headers
                            .iter()
                            .enumerate()
                            .map(|(i, header)| {
                                let val = map.get(header).unwrap_or(&serde_json::Value::Null);
                                let val_str = format_value_for_table(val);
                                let truncated = if val_str.len() > 50 {
                                    format!("{}...", &val_str[..47])
                                } else {
                                    val_str
                                };
                                format!("{:<width$}", truncated, width = widths[i])
                            })
                            .collect();
                        writeln!(writer, "{}", row.join("  "))?;
                    }
                }

                writeln!(writer, "\n{} row(s)", items.len())?;
            } else {
                for item in &items {
                    writeln!(writer, "{}", format_value_for_table(item))?;
                }
            }
        }
        serde_json::Value::Object(map) => {
            let max_key_len = map.keys().map(|k| k.len()).max().unwrap_or(0);
            for (key, value) in &map {
                let val_str = format_value_for_table(value);
                writeln!(writer, "{:<width$}  {}", key, val_str, width = max_key_len)?;
            }
        }
        serde_json::Value::Array(_) => {
            writeln!(writer, "(empty)")?;
        }
        other => {
            writeln!(writer, "{}", format_value_for_table(&other))?;
        }
    }
    Ok(())
}

fn format_value_for_table(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "-".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "-".to_string()
            } else {
                arr.iter()
                    .map(format_value_for_table)
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        }
        serde_json::Value::Object(obj) => {
            serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

fn format_value_for_csv(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .map(format_value_for_csv)
            .collect::<Vec<_>>()
            .join("; "),
        serde_json::Value::Object(obj) => serde_json::to_string(obj).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_table_null() {
        assert_eq!(format_value_for_table(&json!(null)), "-");
    }

    #[test]
    fn test_format_table_bool() {
        assert_eq!(format_value_for_table(&json!(true)), "true");
        assert_eq!(format_value_for_table(&json!(false)), "false");
    }

    #[test]
    fn test_format_table_number() {
        assert_eq!(format_value_for_table(&json!(42)), "42");
        assert_eq!(format_value_for_table(&json!(1.5)), "1.5");
    }

    #[test]
    fn test_format_table_string() {
        assert_eq!(format_value_for_table(&json!("hello")), "hello");
    }

    #[test]
    fn test_format_table_array() {
        assert_eq!(format_value_for_table(&json!([1, 2])), "1, 2");
    }

    #[test]
    fn test_format_table_empty_array() {
        assert_eq!(format_value_for_table(&json!([])), "-");
    }

    #[test]
    fn test_format_table_object() {
        let obj = json!({"key": "value"});
        let result = format_value_for_table(&obj);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_format_csv_null() {
        assert_eq!(format_value_for_csv(&json!(null)), "");
    }

    #[test]
    fn test_format_csv_array() {
        assert_eq!(format_value_for_csv(&json!(["a", "b"])), "a; b");
    }

    #[test]
    fn test_format_csv_object() {
        let obj = json!({"key": "value"});
        let result = format_value_for_csv(&obj);
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }
}
