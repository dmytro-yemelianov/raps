// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Interactive prompt utilities
//!
//! Provides centralized prompt handling with automatic non-interactive mode support.
//! All prompts check for non-interactive mode and return appropriate errors.

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect, Select};

use crate::interactive;

/// Prompt for text input with validation
///
/// Returns an error in non-interactive mode if no default is provided.
pub fn input<S: Into<String>>(prompt: S, default: Option<&str>) -> Result<String> {
    let prompt_str = prompt.into();

    if interactive::is_non_interactive() {
        return default.map(|s| s.to_string()).ok_or_else(|| {
            anyhow::anyhow!(
                "{} is required in non-interactive mode",
                prompt_str.trim_end_matches(':')
            )
        });
    }

    let mut input: Input<String> = Input::new();
    input = input.with_prompt(&prompt_str);

    if let Some(d) = default {
        input = input.default(d.to_string());
    }

    input
        .interact_text()
        .context("Failed to read input from terminal")
}

/// Prompt for text input with custom validation
///
/// Returns an error in non-interactive mode if no default is provided.
pub fn input_validated<S, V>(prompt: S, default: Option<&str>, validator: V) -> Result<String>
where
    S: Into<String>,
    V: Fn(&String) -> Result<(), &'static str> + Clone,
{
    let prompt_str = prompt.into();

    if interactive::is_non_interactive() {
        return default.map(|s| s.to_string()).ok_or_else(|| {
            anyhow::anyhow!(
                "{} is required in non-interactive mode",
                prompt_str.trim_end_matches(':')
            )
        });
    }

    let mut input: Input<String> = Input::new();
    input = input.with_prompt(&prompt_str).validate_with(validator);

    if let Some(d) = default {
        input = input.default(d.to_string());
    }

    input
        .interact_text()
        .context("Failed to read input from terminal")
}

/// Prompt for selection from a list of options
///
/// Returns the selected index. Returns an error in non-interactive mode.
pub fn select<S: Into<String>>(prompt: S, items: &[String]) -> Result<usize> {
    let prompt_str = prompt.into();

    if interactive::is_non_interactive() {
        anyhow::bail!(
            "Selection required for '{}' but running in non-interactive mode",
            prompt_str
        );
    }

    Select::new()
        .with_prompt(&prompt_str)
        .items(items)
        .default(0)
        .interact()
        .context("Failed to read selection from terminal")
}

/// Prompt for selection with a default index
///
/// Returns the selected index. Returns the default in non-interactive mode.
pub fn select_with_default<S: Into<String>>(
    prompt: S,
    items: &[String],
    default: usize,
) -> Result<usize> {
    if interactive::is_non_interactive() {
        return Ok(default);
    }

    let prompt_str = prompt.into();

    Select::new()
        .with_prompt(&prompt_str)
        .items(items)
        .default(default)
        .interact()
        .context("Failed to read selection from terminal")
}

/// Prompt for multiple selections
///
/// Returns an error in non-interactive mode.
pub fn multi_select<S: Into<String>>(prompt: S, items: &[String]) -> Result<Vec<usize>> {
    let prompt_str = prompt.into();

    if interactive::is_non_interactive() {
        anyhow::bail!(
            "Multi-selection required for '{}' but running in non-interactive mode",
            prompt_str
        );
    }

    MultiSelect::new()
        .with_prompt(&prompt_str)
        .items(items)
        .interact()
        .context("Failed to read multi-selection from terminal")
}

/// Prompt for confirmation (yes/no)
///
/// Returns true if --yes flag is set, or prompts interactively.
/// Returns false in non-interactive mode without --yes.
pub fn confirm<S: Into<String>>(prompt: S, default: bool) -> Result<bool> {
    // Auto-confirm if --yes flag is set
    if interactive::is_yes() {
        return Ok(true);
    }

    // Fail in non-interactive mode without --yes
    if interactive::is_non_interactive() {
        return Ok(false);
    }

    let prompt_str = prompt.into();

    Confirm::new()
        .with_prompt(&prompt_str)
        .default(default)
        .interact()
        .context("Failed to read confirmation from terminal")
}

/// Prompt for confirmation with a required affirmative answer
///
/// Use this for destructive operations. Returns true only if user confirms
/// or --yes flag is set. Always returns false in non-interactive mode
/// without --yes.
pub fn confirm_destructive<S: Into<String>>(prompt: S) -> Result<bool> {
    // Auto-confirm if --yes flag is set
    if interactive::is_yes() {
        return Ok(true);
    }

    // Fail in non-interactive mode without --yes
    if interactive::is_non_interactive() {
        return Ok(false);
    }

    let prompt_str = prompt.into();

    Confirm::new()
        .with_prompt(&prompt_str)
        .default(false) // Default to no for destructive operations
        .interact()
        .context("Failed to read confirmation from terminal")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to reset interactive state between tests
    fn reset_state() {
        interactive::init(false, false);
    }

    fn set_non_interactive() {
        interactive::init(true, false);
    }

    fn set_yes_mode() {
        interactive::init(false, true);
    }

    fn set_non_interactive_with_yes() {
        interactive::init(true, true);
    }

    // ==================== Input Tests (Non-Interactive Mode) ====================

    #[test]
    fn test_input_non_interactive_with_default() {
        set_non_interactive();
        let result = input("Enter name:", Some("default_value"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "default_value");
        reset_state();
    }

    #[test]
    fn test_input_non_interactive_without_default() {
        set_non_interactive();
        let result = input("Enter name:", None);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("required"));
        assert!(err.contains("non-interactive"));
        reset_state();
    }

    #[test]
    fn test_input_validated_non_interactive_with_default() {
        set_non_interactive();
        let result = input_validated("Enter email:", Some("test@example.com"), |_| Ok(()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test@example.com");
        reset_state();
    }

    #[test]
    fn test_input_validated_non_interactive_without_default() {
        set_non_interactive();
        let result = input_validated::<_, fn(&String) -> Result<(), &'static str>>(
            "Enter email:",
            None,
            |_| Ok(()),
        );
        assert!(result.is_err());
        reset_state();
    }

    // ==================== Select Tests (Non-Interactive Mode) ====================

    #[test]
    fn test_select_non_interactive_fails() {
        set_non_interactive();
        let items = vec!["Option 1".to_string(), "Option 2".to_string()];
        let result = select("Choose:", &items);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("non-interactive"));
        reset_state();
    }

    #[test]
    fn test_select_with_default_non_interactive() {
        set_non_interactive();
        let items = vec!["Option 1".to_string(), "Option 2".to_string()];
        let result = select_with_default("Choose:", &items, 1);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
        reset_state();
    }

    // ==================== MultiSelect Tests (Non-Interactive Mode) ====================

    #[test]
    fn test_multi_select_non_interactive_fails() {
        set_non_interactive();
        let items = vec!["Option 1".to_string(), "Option 2".to_string()];
        let result = multi_select("Choose multiple:", &items);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("non-interactive"));
        reset_state();
    }

    // ==================== Confirm Tests ====================

    #[test]
    fn test_confirm_yes_mode() {
        set_yes_mode();
        let result = confirm("Proceed?", false);
        assert!(result.is_ok());
        assert!(result.unwrap()); // --yes flag auto-confirms
        reset_state();
    }

    #[test]
    fn test_confirm_non_interactive_no_yes() {
        set_non_interactive();
        let result = confirm("Proceed?", true);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Returns false without --yes
        reset_state();
    }

    #[test]
    fn test_confirm_non_interactive_with_yes() {
        set_non_interactive_with_yes();
        let result = confirm("Proceed?", false);
        assert!(result.is_ok());
        assert!(result.unwrap()); // --yes takes precedence
        reset_state();
    }

    // ==================== Confirm Destructive Tests ====================

    #[test]
    fn test_confirm_destructive_yes_mode() {
        set_yes_mode();
        let result = confirm_destructive("Delete all?");
        assert!(result.is_ok());
        assert!(result.unwrap());
        reset_state();
    }

    #[test]
    fn test_confirm_destructive_non_interactive_no_yes() {
        set_non_interactive();
        let result = confirm_destructive("Delete all?");
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Fails safe - returns false
        reset_state();
    }

    #[test]
    fn test_confirm_destructive_non_interactive_with_yes() {
        set_non_interactive_with_yes();
        let result = confirm_destructive("Delete all?");
        assert!(result.is_ok());
        assert!(result.unwrap());
        reset_state();
    }

    // ==================== Prompt String Trimming Tests ====================

    #[test]
    fn test_input_trims_colon_in_error() {
        set_non_interactive();
        let result = input("Enter name:", None);
        let err = result.unwrap_err().to_string();
        // Should say "Enter name is required" not "Enter name: is required"
        assert!(err.contains("Enter name"));
        assert!(!err.contains("Enter name:"));
        reset_state();
    }
}
