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
                // Fallback to JSON if specific table logic isn't invoked by the command directly.
                serde_json::to_writer_pretty(&mut *writer, data)?;
                writeln!(writer)?;
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
