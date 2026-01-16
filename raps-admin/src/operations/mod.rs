// SPDX-License-Identifier: Apache-2.0
// Copyright 2024-2025 Dmytro Yemelianov

//! Specific bulk operation implementations

pub mod add_user;
pub mod folder_rights;
pub mod remove_user;
pub mod update_role;

// Re-export main functions
pub use add_user::{BulkAddUserParams, bulk_add_user, resume_bulk_add_user};
pub use folder_rights::{
    BulkUpdateFolderRightsParams, bulk_update_folder_rights, resume_bulk_update_folder_rights,
};
pub use remove_user::{BulkRemoveUserParams, bulk_remove_user, resume_bulk_remove_user};
pub use update_role::{BulkUpdateRoleParams, bulk_update_role, resume_bulk_update_role};
