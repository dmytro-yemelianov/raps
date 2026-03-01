// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Path sanitization and directory permission utilities
//!
//! Defense-in-depth for filenames derived from API responses or untrusted input.

use anyhow::{Context, Result, bail};
use std::path::{Component, Path, PathBuf};

/// Strip path traversal components and return only the final filename.
///
/// Removes `..` components, path separators, control characters, and NUL bytes.
/// Unicode filenames are preserved.
pub fn sanitize_filename(name: &str) -> Result<String> {
    // Strip NUL bytes and control characters (except space)
    let cleaned: String = name
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect();

    let path = Path::new(&cleaned);

    // Walk components and take only the last Normal component
    let filename = path
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .next_back();

    match filename {
        Some(f) if !f.is_empty() => Ok(f.to_string()),
        _ => bail!("Filename is empty or contains only traversal components"),
    }
}

/// Canonicalize both paths and confirm `target` is a descendant of `base_dir`.
pub fn validate_path_within(target: &Path, base_dir: &Path) -> Result<PathBuf> {
    let canon_base = base_dir
        .canonicalize()
        .with_context(|| format!("Cannot canonicalize base dir: {}", base_dir.display()))?;
    let canon_target = target
        .canonicalize()
        .with_context(|| format!("Cannot canonicalize target: {}", target.display()))?;

    if canon_target.starts_with(&canon_base) {
        Ok(canon_target)
    } else {
        bail!(
            "Path '{}' escapes base directory '{}'",
            target.display(),
            base_dir.display()
        )
    }
}

/// Sanitize `untrusted_name` then join it to `base_dir` and validate the result.
pub fn safe_join(base_dir: &Path, untrusted_name: &str) -> Result<PathBuf> {
    let safe_name = sanitize_filename(untrusted_name)?;
    let joined = base_dir.join(&safe_name);

    // If base_dir already exists on disk, do a full canonicalize check.
    // Otherwise just confirm the filename itself is safe (no traversal).
    if base_dir.exists() {
        // Touch-create so canonicalize works on the target
        if !joined.exists() {
            // Just check the logic — the caller will create the real file.
            // Verify the safe_name doesn't escape via symlinks by checking
            // that the parent resolves within base_dir.
            if let Some(parent) = joined.parent() {
                let canon_parent = parent
                    .canonicalize()
                    .with_context(|| format!("Cannot canonicalize parent: {}", parent.display()))?;
                let canon_base = base_dir.canonicalize()?;
                if !canon_parent.starts_with(&canon_base) {
                    bail!(
                        "Path '{}' escapes base directory '{}'",
                        joined.display(),
                        base_dir.display()
                    );
                }
            }
        } else {
            validate_path_within(&joined, base_dir)?;
        }
    }

    Ok(joined)
}

/// Create directories with mode 0o700 (owner-only) on Unix.
///
/// Uses `DirBuilder::mode()` on Unix to avoid a TOCTOU window between
/// creation and permission setting.
pub fn create_dir_restricted(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
    }

    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_normal_filename_unchanged() {
        assert_eq!(sanitize_filename("report.pdf").unwrap(), "report.pdf");
    }

    #[test]
    fn test_traversal_etc_passwd() {
        assert_eq!(
            sanitize_filename("../../etc/passwd").unwrap(),
            "passwd"
        );
    }

    #[test]
    fn test_absolute_path_stripped() {
        assert_eq!(sanitize_filename("/etc/shadow").unwrap(), "shadow");
    }

    #[test]
    fn test_windows_traversal() {
        assert_eq!(
            sanitize_filename("..\\..\\windows\\system32\\config").unwrap(),
            // On Unix, backslashes are valid filename chars, so the whole
            // last component survives. On Windows, Component parsing would
            // split on backslash. Either way, no traversal escapes.
            if cfg!(windows) {
                "config".to_string()
            } else {
                "..\\..\\windows\\system32\\config".to_string()
            }
        );
    }

    #[test]
    fn test_empty_string_errors() {
        assert!(sanitize_filename("").is_err());
    }

    #[test]
    fn test_dotdot_alone_errors() {
        assert!(sanitize_filename("..").is_err());
    }

    #[test]
    fn test_nul_bytes_stripped() {
        assert_eq!(
            sanitize_filename("file\0name.txt").unwrap(),
            "filename.txt"
        );
    }

    #[test]
    fn test_unicode_preserved() {
        assert_eq!(
            sanitize_filename("日本語ファイル.txt").unwrap(),
            "日本語ファイル.txt"
        );
    }

    #[test]
    fn test_validate_path_within_rejects_escape() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        // Create a file inside base
        let inside = base.join("safe.txt");
        fs::write(&inside, "ok").unwrap();

        // This should pass
        assert!(validate_path_within(&inside, base).is_ok());

        // A path outside base should fail
        let outside = Path::new("/tmp");
        assert!(validate_path_within(outside, base).is_err());
    }

    #[test]
    fn test_safe_join_combines_correctly() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        let result = safe_join(base, "report.pdf").unwrap();
        assert_eq!(result, base.join("report.pdf"));
    }

    #[test]
    fn test_safe_join_strips_traversal() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        let result = safe_join(base, "../../etc/passwd").unwrap();
        assert_eq!(result, base.join("passwd"));
    }

    #[cfg(unix)]
    #[test]
    fn test_create_dir_restricted_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("restricted");

        create_dir_restricted(&dir).unwrap();

        let perms = fs::metadata(&dir).unwrap().permissions();
        assert_eq!(perms.mode() & 0o777, 0o700);
    }
}
