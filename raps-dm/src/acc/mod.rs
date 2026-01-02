// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Autodesk Construction Cloud (ACC) modules
//!
//! Provides access to ACC-specific APIs:
//! - Issues - Track and manage construction issues
//! - RFIs - Requests for Information
//! - Assets - Physical asset tracking
//! - Submittals - Submittal workflow management
//! - Checklists - Quality and safety checklists

mod assets;
mod checklists;
mod issues;
mod rfi;
mod submittals;

pub use assets::*;
pub use checklists::*;
pub use issues::*;
pub use rfi::*;
pub use submittals::*;

use raps_kernel::{AuthClient, Config, HttpClient, RapsError, Result};

/// ACC API client - provides access to all ACC modules
pub struct AccClient {
    http: HttpClient,
    auth: AuthClient,
    config: Config,
}

impl AccClient {
    /// Create a new ACC client
    pub fn new(http: HttpClient, auth: AuthClient, config: Config) -> Self {
        Self { http, auth, config }
    }

    /// Get issues client
    pub fn issues(&self) -> IssuesClient {
        IssuesClient::new(self.http.clone(), self.auth.clone(), self.config.clone())
    }

    /// Get RFI client
    pub fn rfis(&self) -> RfiClient {
        RfiClient::new(self.http.clone(), self.auth.clone(), self.config.clone())
    }

    /// Get assets client
    pub fn assets(&self) -> AssetsClient {
        AssetsClient::new(self.http.clone(), self.auth.clone(), self.config.clone())
    }

    /// Get submittals client
    pub fn submittals(&self) -> SubmittalsClient {
        SubmittalsClient::new(self.http.clone(), self.auth.clone(), self.config.clone())
    }

    /// Get checklists client
    pub fn checklists(&self) -> ChecklistsClient {
        ChecklistsClient::new(self.http.clone(), self.auth.clone(), self.config.clone())
    }
}
