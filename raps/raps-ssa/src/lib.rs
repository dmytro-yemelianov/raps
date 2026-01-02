//! RAPS SSA Service - Secure Service Accounts client
//!
//! This crate provides functionality for managing Secure Service Accounts (SSA)
//! including robot accounts, keys, and JWT assertion exchange.

#![deny(warnings)]
#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]

/// Robot account management
pub mod robot;

/// JWT assertion exchange
pub mod jwt;
