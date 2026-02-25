// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use super::command_tree::{build_command_map, build_command_tree};
use super::completer::get_completions_raw;
use super::hinter::get_hint_raw;

#[test]
fn test_command_tree() {
    let commands = build_command_tree();
    assert!(!commands.is_empty());
    let map = build_command_map(&commands);
    assert!(!map.is_empty());
}

#[test]
fn test_command_completions() {
    let commands = build_command_tree();
    let map = build_command_map(&commands);

    // Test empty line
    let completions = get_completions_raw(&commands, &map, "");
    assert!(!completions.is_empty());

    // Test partial command
    let completions = get_completions_raw(&commands, &map, "au");
    assert!(completions.iter().any(|(r, _)| r == "auth"));

    // Test command with space
    let completions = get_completions_raw(&commands, &map, "auth ");
    assert!(completions.iter().any(|(r, _)| r == "login"));

    // Test partial subcommand
    let completions = get_completions_raw(&commands, &map, "auth log");
    assert!(completions.iter().any(|(r, _)| r == "login"));
    assert!(completions.iter().any(|(r, _)| r == "logout"));
}

#[test]
fn test_hints() {
    let commands = build_command_tree();
    let map = build_command_map(&commands);

    // Test partial command hint
    let hint = get_hint_raw(&commands, &map, "au");
    assert!(hint.is_some());
    let (display, _) = hint.unwrap();
    assert!(display.starts_with("th"));

    // Test complete command shows subcommand hint
    let hint = get_hint_raw(&commands, &map, "auth ");
    assert!(hint.is_some());

    // Test subcommand with params
    let hint = get_hint_raw(&commands, &map, "bucket create ");
    assert!(hint.is_some());
    let (display, _) = hint.unwrap();
    assert!(display.contains("BUCKET_KEY"));
}
