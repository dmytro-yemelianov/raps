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

#[test]
fn test_snapshot_json_nested_object() {
    #[derive(Serialize)]
    struct Outer {
        name: String,
        inner: Inner,
    }
    #[derive(Serialize)]
    struct Inner {
        key: String,
        count: u32,
    }

    let data = Outer {
        name: "test".to_string(),
        inner: Inner {
            key: "abc".to_string(),
            count: 5,
        },
    };

    let mut buffer = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Json, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn test_snapshot_yaml_nested_object() {
    #[derive(Serialize)]
    struct Config {
        name: String,
        enabled: bool,
        items: Vec<String>,
    }

    let data = Config {
        name: "pipeline".to_string(),
        enabled: true,
        items: vec!["step1".to_string(), "step2".to_string()],
    };

    let mut buffer = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Yaml, &mut buffer).unwrap();
    let output = String::from_utf8(buffer).unwrap();
    insta::assert_snapshot!(output);
}

#[test]
fn test_snapshot_table_boolean_values() {
    let data = serde_json::json!([
        {"name": "Feature A", "enabled": true},
        {"name": "Feature B", "enabled": false}
    ]);

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Table, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn test_snapshot_csv_with_special_chars() {
    #[derive(Serialize)]
    struct Record {
        name: String,
        value: String,
    }

    let data = vec![
        Record {
            name: "with,comma".to_string(),
            value: "normal".to_string(),
        },
        Record {
            name: "with\nnewline".to_string(),
            value: "also\"quoted".to_string(),
        },
    ];

    let mut output = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Csv, &mut output).unwrap();
    let result = String::from_utf8(output).unwrap();
    insta::assert_snapshot!(result);
}

#[test]
fn test_ndjson_writes_one_line_per_item() {
    use serde::Serialize;
    use crate::output::{OutputFormat, formatter::OutputFormatter};

    #[derive(Serialize, schemars::JsonSchema)]
    struct Row { id: u32, name: String }

    let data = vec![
        Row { id: 1, name: "alpha".into() },
        Row { id: 2, name: "beta".into() },
    ];
    let mut buf = Vec::new();
    OutputFormatter::print_output(&data, OutputFormat::Ndjson, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    let lines: Vec<_> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"alpha\""));
    assert!(lines[1].contains("\"beta\""));
    for line in &lines {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
}

#[test]
fn test_ndjson_single_object_one_line() {
    use serde::Serialize;
    use crate::output::{OutputFormat, formatter::OutputFormatter};

    #[derive(Serialize, schemars::JsonSchema)]
    struct Single { value: i32 }

    let mut buf = Vec::new();
    OutputFormatter::print_output(&Single { value: 42 }, OutputFormat::Ndjson, &mut buf).unwrap();
    let out = String::from_utf8(buf).unwrap();
    assert_eq!(out.lines().count(), 1);
    serde_json::from_str::<serde_json::Value>(out.trim()).unwrap();
}

#[test]
fn test_ndjson_from_str_roundtrip() {
    use crate::output::OutputFormat;
    assert_eq!(<OutputFormat as std::str::FromStr>::from_str("ndjson").unwrap(), OutputFormat::Ndjson);
    assert_eq!(OutputFormat::Ndjson.to_string(), "ndjson");
}
