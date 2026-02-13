use super::formatter::OutputFormatter;
use super::*;
use serde::Serialize;

#[derive(Serialize)]
struct TestData {
    name: String,
    value: i32,
}

#[test]
fn test_json_output() {
    let data = TestData {
        name: "test".to_string(),
        value: 42,
    };
    let mut buffer = Vec::new();

    OutputFormatter::print_output(&data, OutputFormat::Json, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let expected = serde_json::to_string_pretty(&data).unwrap() + "\n";

    // Normalize newlines for cross-platform comparison
    assert_eq!(output.replace("\r\n", "\n"), expected.replace("\r\n", "\n"));
}

#[test]
fn test_yaml_output() {
    let data = TestData {
        name: "test".to_string(),
        value: 42,
    };
    let mut buffer = Vec::new();

    OutputFormatter::print_output(&data, OutputFormat::Yaml, &mut buffer).unwrap();

    let output = String::from_utf8(buffer).unwrap();
    let expected = serde_yaml::to_string(&data).unwrap();

    assert_eq!(output, expected);
}

#[test]
fn test_table_format_array_of_objects() {
    #[derive(Serialize)]
    struct Row {
        name: String,
        status: String,
        count: u32,
    }

    let data = vec![
        Row {
            name: "Alpha".to_string(),
            status: "active".to_string(),
            count: 42,
        },
        Row {
            name: "Beta".to_string(),
            status: "pending".to_string(),
            count: 7,
        },
    ];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();

    // Should contain headers and data
    assert!(result.contains("name"));
    assert!(result.contains("status"));
    assert!(result.contains("count"));
    assert!(result.contains("Alpha"));
    assert!(result.contains("Beta"));
    assert!(result.contains("active"));
    assert!(result.contains("2 row(s)"));
    // Should NOT be JSON
    assert!(!result.contains("{"));
}

#[test]
fn test_table_format_single_object() {
    #[derive(Serialize)]
    struct Info {
        id: String,
        name: String,
    }

    let data = Info {
        id: "123".to_string(),
        name: "Test".to_string(),
    };

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();

    // Should be key-value format, NOT JSON
    assert!(result.contains("id"));
    assert!(result.contains("123"));
    assert!(result.contains("name"));
    assert!(result.contains("Test"));
    assert!(!result.contains("{"));
}

#[test]
fn test_table_format_empty_array() {
    let data: Vec<String> = vec![];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();

    assert!(result.contains("(empty)"));
}
