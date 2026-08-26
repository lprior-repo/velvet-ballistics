#![forbid(unsafe_code)]

//! Velvet Ballistics Workspace Tests
//!
//! Test infrastructure and workspace-level testing support.

pub mod acceptance_catalog;
pub mod bdd_runner;
pub mod boundary_inventory;
pub mod quality;
pub mod test_util;

pub use test_util::TestSetupError;
