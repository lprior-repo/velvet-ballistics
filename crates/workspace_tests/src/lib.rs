//! Velvet Ballistics Workspace Tests
//!
//! Test infrastructure and workspace-level testing support.

pub mod acceptance_catalog;
pub mod bdd_runner;
pub mod boundary_inventory;
pub mod quality;

#[cfg(kani)]
pub mod kani_vb_dybj_trailing_decode;
