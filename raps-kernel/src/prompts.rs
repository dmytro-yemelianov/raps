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
    // Note: Interactive tests are difficult to unit test
    // These would need integration tests with a pseudo-terminal
}
