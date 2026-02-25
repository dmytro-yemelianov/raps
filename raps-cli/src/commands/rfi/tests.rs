// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use super::crud::CsvRfiRow;
use super::{truncate_str, RfiOutput};

#[test]
fn test_csv_rfi_row_deserialization() {
    let csv_data = "title,description,assigned_to\nMissing specs,Need structural specs for floor 3,jane@example.com\n";
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
    let row: CsvRfiRow = rdr.deserialize().next().unwrap().unwrap();
    assert_eq!(row.title, "Missing specs");
    assert_eq!(
        row.description.unwrap(),
        "Need structural specs for floor 3"
    );
    assert_eq!(row.assigned_to.unwrap(), "jane@example.com");
}

#[test]
fn test_csv_rfi_row_minimal() {
    let csv_data = "title,description,assigned_to\nMissing specs,,\n";
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
    let row: CsvRfiRow = rdr.deserialize().next().unwrap().unwrap();
    assert_eq!(row.title, "Missing specs");
    assert!(
        row.description.is_none() || row.description.as_deref() == Some(""),
        "Expected None or empty for description, got {:?}",
        row.description
    );
    assert!(
        row.assigned_to.is_none() || row.assigned_to.as_deref() == Some(""),
        "Expected None or empty for assigned_to, got {:?}",
        row.assigned_to
    );
}

#[test]
fn test_csv_rfi_row_multiple_rows() {
    let csv_data = "\
title,description,assigned_to
Missing specs,Need structural specs,jane@example.com
Electrical query,Wiring clarification,bob@example.com
HVAC concern,,
";
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv_data.as_bytes());
    let rows: Vec<CsvRfiRow> = rdr.deserialize().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].title, "Missing specs");
    assert_eq!(rows[1].title, "Electrical query");
    assert_eq!(rows[1].assigned_to.as_deref(), Some("bob@example.com"));
    assert_eq!(rows[2].title, "HVAC concern");
}

#[test]
fn test_rfi_output_serialization() {
    let output = RfiOutput {
        id: "rfi-001".to_string(),
        number: Some("RFI-42".to_string()),
        title: "Missing specs".to_string(),
        status: "open".to_string(),
        priority: Some("high".to_string()),
        question: Some("Where are the structural specs?".to_string()),
        answer: None,
        due_date: Some("2025-06-15".to_string()),
        assigned_to_name: Some("Jane Smith".to_string()),
        created_at: Some("2025-01-10T08:00:00Z".to_string()),
    };
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("\"id\":\"rfi-001\""));
    assert!(json.contains("\"number\":\"RFI-42\""));
    assert!(json.contains("\"title\":\"Missing specs\""));
    assert!(json.contains("\"status\":\"open\""));
    assert!(json.contains("\"priority\":\"high\""));
    assert!(json.contains("\"question\":\"Where are the structural specs?\""));
    assert!(json.contains("\"answer\":null"));
    assert!(json.contains("\"due_date\":\"2025-06-15\""));
    assert!(json.contains("\"assigned_to_name\":\"Jane Smith\""));
}

#[test]
fn test_rfi_output_all_none_optionals() {
    let output = RfiOutput {
        id: "rfi-002".to_string(),
        number: None,
        title: "Basic RFI".to_string(),
        status: "draft".to_string(),
        priority: None,
        question: None,
        answer: None,
        due_date: None,
        assigned_to_name: None,
        created_at: None,
    };
    let json = serde_json::to_string(&output).unwrap();
    assert!(json.contains("\"id\":\"rfi-002\""));
    assert!(json.contains("\"title\":\"Basic RFI\""));
    assert!(json.contains("\"status\":\"draft\""));
    // All optional fields should serialize as null
    assert!(json.contains("\"number\":null"));
    assert!(json.contains("\"priority\":null"));
}

#[test]
fn test_truncate_str_short() {
    assert_eq!(truncate_str("hello", 10), "hello");
}

#[test]
fn test_truncate_str_exact() {
    assert_eq!(truncate_str("hello", 5), "hello");
}

#[test]
fn test_truncate_str_long() {
    let result = truncate_str("hello world", 8);
    assert_eq!(result, "hello...");
    assert_eq!(result.len(), 8);
}
