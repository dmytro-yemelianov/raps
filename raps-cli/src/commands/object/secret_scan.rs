// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Secret scanning: detect potential credentials in files before upload.

use anyhow::Result;
use regex::Regex;
use std::path::Path;

pub struct SecretMatch {
    pub line_number: usize,
    pub pattern_name: String,
    pub snippet: String, // redacted
}

pub fn scan_file(path: &Path) -> Result<Vec<SecretMatch>> {
    // Skip binary files: check first 8KB for null bytes
    let mut file = std::fs::File::open(path)?;
    use std::io::Read;
    let file_len = path.metadata().map(|m| m.len()).unwrap_or(8192) as usize;
    let probe_len = 8192.min(file_len);
    let mut probe = vec![0u8; probe_len];
    let n = file.read(&mut probe)?;
    if probe[..n].contains(&0u8) {
        return Ok(vec![]); // binary file, skip
    }

    // Read up to 100KB of text
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let content: String = content.chars().take(100 * 1024).collect();

    let patterns: &[(&str, &str)] = &[
        (
            "APS Client Secret",
            r"(?i)aps[_-]?client[_-]?secret\s*[=:]\s*\S{8,}",
        ),
        (
            "Generic API Key",
            r"(?i)api[_-]?key\s*[=:]\s*[A-Za-z0-9_\-]{16,}",
        ),
        ("AWS Access Key", r"AKIA[0-9A-Z]{16}"),
        (
            "Private Key",
            r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----",
        ),
        (
            "Password in env",
            r#"(?i)(password|passwd|secret|token)\s*[=:]\s*["']?[A-Za-z0-9_\-/+=]{8,}"#,
        ),
        ("GitHub Token", r"gh[pousr]_[A-Za-z0-9]{36,}"),
    ];

    let mut matches = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        for (name, pattern) in patterns {
            let re = Regex::new(pattern).unwrap();
            if re.is_match(line) {
                let snippet = if line.len() > 60 {
                    format!("{}...", &line[..60])
                } else {
                    line.to_string()
                };
                matches.push(SecretMatch {
                    line_number: line_num + 1,
                    pattern_name: name.to_string(),
                    snippet,
                });
                break; // one match per line is enough
            }
        }
    }
    Ok(matches)
}
