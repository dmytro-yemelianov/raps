// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

use std::borrow::Cow;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch};

/// RAPS interactive prompt with proper styled/raw separation.
///
/// The Prompt trait separates the raw text content from styling,
/// so reedline can calculate cursor position from plain text width
/// while still displaying a colored prompt.
pub struct RapsPrompt;

impl Prompt for RapsPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("raps> ")
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("::: ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        _history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Borrowed("(search) ")
    }

    fn get_prompt_color(&self) -> reedline::Color {
        reedline::Color::Yellow
    }
}
