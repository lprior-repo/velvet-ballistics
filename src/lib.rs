//! Velvet Ballistics Workspace
//!
//! This is a virtual workspace container. The actual crates are in `crates/`.

pub mod boundary_inventory;
pub mod quality;

/// Workspace marker - all actual code is in crates/
pub const WORKSPACE_NAME: &str = "velvet_ballastics";
