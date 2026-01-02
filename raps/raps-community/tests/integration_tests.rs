// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Integration tests for raps-community
//!
//! These tests verify Community tier features.

use raps_community::{AccClient, DesignAutomationClient, RealityCaptureClient, WebhooksClient};
use raps_community::pipeline::{Pipeline, PipelineRunner, PipelineStep};
use raps_community::plugin::{PluginManager, Alias};

#[test]
fn test_pipeline_structure() {
    let pipeline = Pipeline {
        name: "test-pipeline".to_string(),
        description: Some("A test pipeline".to_string()),
        steps: vec![
            PipelineStep {
                name: "step1".to_string(),
                command: "raps bucket list".to_string(),
                args: vec![],
                continue_on_error: false,
            },
            PipelineStep {
                name: "step2".to_string(),
                command: "raps translate status".to_string(),
                args: vec!["--urn".to_string(), "test".to_string()],
                continue_on_error: true,
            },
        ],
    };
    
    assert_eq!(pipeline.name, "test-pipeline");
    assert_eq!(pipeline.steps.len(), 2);
    assert!(pipeline.steps[1].continue_on_error);
}

#[test]
fn test_pipeline_runner_creation() {
    let runner = PipelineRunner::new();
    // Runner should be creatable (unit struct has zero size, but can be created)
    let _ = runner; // Just verify it was created
}

#[test]
fn test_plugin_manager_creation() {
    let mut manager = PluginManager::new();
    
    // Initially no plugins
    assert!(manager.list_plugins().is_empty());
    
    // Add alias
    manager.add_alias("lb".to_string(), "bucket list".to_string());
    
    // Should have one alias
    assert_eq!(manager.list_aliases().len(), 1);
    
    // Resolve alias
    let resolved = manager.resolve_alias("lb");
    assert_eq!(resolved, Some("bucket list"));
}

#[test]
fn test_plugin_alias_removal() {
    let mut manager = PluginManager::new();
    
    manager.add_alias("test".to_string(), "command".to_string());
    assert!(manager.resolve_alias("test").is_some());
    
    let removed = manager.remove_alias("test");
    assert!(removed);
    assert!(manager.resolve_alias("test").is_none());
}

#[test]
fn test_pipeline_yaml_parsing() {
    let yaml = r#"
name: test-pipeline
description: Test
steps:
  - name: list buckets
    command: raps bucket list
    args: []
    continue_on_error: false
"#;
    
    let pipeline: Result<Pipeline, _> = serde_yaml::from_str(yaml);
    assert!(pipeline.is_ok());
    
    let p = pipeline.unwrap();
    assert_eq!(p.name, "test-pipeline");
    assert_eq!(p.steps.len(), 1);
}

#[test]
fn test_pipeline_json_parsing() {
    let json = r#"{
        "name": "test-pipeline",
        "description": "Test",
        "steps": [
            {
                "name": "list buckets",
                "command": "raps bucket list",
                "args": [],
                "continue_on_error": false
            }
        ]
    }"#;
    
    let pipeline: Result<Pipeline, _> = serde_json::from_str(json);
    assert!(pipeline.is_ok());
}

// Tests requiring credentials
#[test]
#[ignore = "requires APS credentials (3-legged)"]
fn test_acc_list_issues() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_da_list_engines() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_reality_create_photoscene() {
    // Requires real credentials
}

#[test]
#[ignore = "requires APS credentials"]
fn test_webhooks_list() {
    // Requires real credentials
}
