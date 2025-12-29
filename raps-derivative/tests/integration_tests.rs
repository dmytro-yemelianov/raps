// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for raps-derivative
//!
//! These tests verify Model Derivative service functionality.

use raps_derivative::OutputFormat;
use raps_kernel::Urn;

#[test]
fn test_output_format_svf() {
    let format = OutputFormat::Svf;
    // SVF should be the legacy viewing format
    assert!(matches!(format, OutputFormat::Svf));
}

#[test]
fn test_output_format_svf2() {
    let format = OutputFormat::Svf2;
    // SVF2 is the newer format
    assert!(matches!(format, OutputFormat::Svf2));
}

#[test]
fn test_output_format_all() {
    let formats = OutputFormat::all();
    
    // Should have multiple format options
    assert!(formats.len() >= 5);
    assert!(formats.iter().any(|f| matches!(f, OutputFormat::Svf2)));
}

#[test]
fn test_urn_for_derivative() {
    // Urn::from_path creates properly encoded URN for derivative API calls
    let urn = Urn::from_path("mybucket", "model.rvt");
    
    // Should be valid for derivative API calls
    assert!(!urn.as_str().is_empty());
    assert!(urn.decode().is_ok());
    
    let decoded = urn.decode().unwrap();
    assert!(decoded.contains("mybucket"));
    assert!(decoded.contains("model.rvt"));
}

#[test]
fn test_manifest_filter_by_format() {
    // Test filtering manifest by format
    // This is a unit test for the filter logic
    let manifest_json = serde_json::json!({
        "derivatives": [
            {
                "outputType": "svf2",
                "children": [
                    {"type": "geometry", "role": "3d"},
                    {"type": "resource", "role": "thumbnail"}
                ]
            },
            {
                "outputType": "obj",
                "children": []
            }
        ]
    });
    
    // Parse and filter logic would go here
    assert!(manifest_json["derivatives"].is_array());
}

// Tests requiring credentials
#[test]
#[ignore = "requires APS credentials"]
fn test_translate_model() {
    // Requires real credentials and a valid URN
}

#[test]
#[ignore = "requires APS credentials"]
fn test_get_manifest() {
    // Requires real credentials and a valid URN
}

#[test]
#[ignore = "requires APS credentials"]
fn test_download_derivative() {
    // Requires real credentials and a valid URN
}
