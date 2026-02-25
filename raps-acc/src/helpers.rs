// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Project ID helper functions for ACC/BIM 360

/// Strip "b." or "a." prefix from a project ID if present.
///
/// Used by Admin API and Project Users API which expect raw UUIDs.
/// Idempotent: already-stripped IDs pass through unchanged.
pub fn strip_project_prefix(id: &str) -> String {
    if let Some(stripped) = id.strip_prefix("b.") {
        stripped.to_string()
    } else if let Some(stripped) = id.strip_prefix("a.") {
        stripped.to_string()
    } else {
        id.to_string()
    }
}

/// Ensure a project ID has the "b." prefix required by Data Management API.
///
/// Used by Permissions API which expects "b."-prefixed project IDs.
/// Idempotent: already-prefixed IDs ("b." or "a.") pass through unchanged.
pub fn ensure_project_prefix(id: &str) -> String {
    if id.starts_with("b.") || id.starts_with("a.") {
        id.to_string()
    } else {
        format!("b.{}", id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_project_prefix_bim360() {
        assert_eq!(strip_project_prefix("b.proj-123"), "proj-123");
    }

    #[test]
    fn test_strip_project_prefix_acc() {
        assert_eq!(strip_project_prefix("a.base64id"), "base64id");
    }

    #[test]
    fn test_strip_project_prefix_raw_uuid() {
        assert_eq!(strip_project_prefix("proj-123"), "proj-123");
    }

    #[test]
    fn test_strip_project_prefix_idempotent() {
        let stripped = strip_project_prefix("b.proj-123");
        assert_eq!(strip_project_prefix(&stripped), "proj-123");
    }

    #[test]
    fn test_ensure_project_prefix_raw_uuid() {
        assert_eq!(ensure_project_prefix("proj-123"), "b.proj-123");
    }

    #[test]
    fn test_ensure_project_prefix_already_prefixed() {
        assert_eq!(ensure_project_prefix("b.proj-123"), "b.proj-123");
    }

    #[test]
    fn test_ensure_project_prefix_acc_passthrough() {
        assert_eq!(ensure_project_prefix("a.base64id"), "a.base64id");
    }

    #[test]
    fn test_ensure_project_prefix_idempotent() {
        let prefixed = ensure_project_prefix("proj-123");
        assert_eq!(ensure_project_prefix(&prefixed), "b.proj-123");
    }
}
