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
    insta::assert_snapshot!(output);
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
    insta::assert_snapshot!(output);
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

    insta::assert_snapshot!(result);
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

    insta::assert_snapshot!(result);
}

#[test]
fn test_table_format_empty_array() {
    let data: Vec<String> = vec![];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();

    insta::assert_snapshot!(result);
}

#[test]
fn test_snapshot_csv_array() {
    #[derive(Serialize)]
    struct Record {
        name: String,
        value: String,
        count: u32,
    }

    let data = vec![
        Record {
            name: "Alpha".to_string(),
            value: "hello, world".to_string(),
            count: 42,
        },
        Record {
            name: "Beta".to_string(),
            value: "foo\"bar".to_string(),
            count: 7,
        },
    ];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Csv, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn test_snapshot_table_null_values() {
    let data = serde_json::json!([
        {"name": "Alpha", "status": null, "count": 1},
        {"name": "Beta", "status": "active", "count": null}
    ]);

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn test_snapshot_table_long_values() {
    #[derive(Serialize)]
    struct Record {
        name: String,
        description: String,
    }

    let data = vec![Record {
        name: "Test".to_string(),
        description: "A".repeat(60),
    }];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();
    insta::assert_snapshot!(result);
}
