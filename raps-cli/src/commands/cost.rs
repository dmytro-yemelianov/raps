// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Cost and time estimation for uploads and translation operations.

use colored::Colorize;

use crate::output::OutputFormat;

use super::object::format_size;

pub struct CostEstimate {
    pub file_size: u64,
    pub upload_time_estimate_secs: u64,
    pub translation_time_estimate_secs: Option<u64>,
    pub storage_warning: Option<String>,
}

impl CostEstimate {
    pub fn for_upload(file_size: u64) -> Self {
        let upload_secs = file_size / (10 * 1024 * 1024); // 10 MB/s assumed
        let storage_warning = if file_size > 500 * 1024 * 1024 {
            Some(format!(
                "Large file ({:.0} MB) — consider using --resume for reliability",
                file_size as f64 / 1024.0 / 1024.0
            ))
        } else {
            None
        };
        Self {
            file_size,
            upload_time_estimate_secs: upload_secs.max(1),
            translation_time_estimate_secs: None,
            storage_warning,
        }
    }

    pub fn for_translation(file_size: u64, format: &str) -> Self {
        // Rough heuristics: Revit files take longer, simple formats faster
        let base_secs = (file_size / (1024 * 1024)).max(10); // at least 10s
        let multiplier = match format {
            "rvt" | "rfa" => 3,
            "ifc" => 2,
            "nwd" | "nwc" => 2,
            _ => 1,
        };
        Self {
            file_size,
            upload_time_estimate_secs: 0,
            translation_time_estimate_secs: Some(base_secs * multiplier),
            storage_warning: None,
        }
    }

    pub fn print(&self, output_format: &OutputFormat) {
        if !output_format.supports_colors() {
            return;
        }
        println!("{}", "Cost Estimate:".bold().yellow());
        println!("  File size: {}", format_size(self.file_size));
        println!(
            "  Est. upload time: ~{}s (assuming 10 MB/s)",
            self.upload_time_estimate_secs
        );
        if let Some(t) = self.translation_time_estimate_secs {
            println!("  Est. translation time: ~{}s", t);
        }
        if let Some(ref warn) = self.storage_warning {
            println!("  {}: {}", "Warning".yellow().bold(), warn);
        }
    }
}
