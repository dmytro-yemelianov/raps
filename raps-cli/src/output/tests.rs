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
