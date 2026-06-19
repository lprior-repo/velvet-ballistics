#![forbid(unsafe_code)]

//! Velvet Ballistics Workspace
//!
//! This is a virtual workspace container. The actual crates are in `crates/`.

pub mod boundary_inventory;

// HVR-PO-BI-001: feature-isolated vb-god2f boundary-inventory Kani harness.
#[cfg(all(kani, feature = "kani-vb-god2f-boundary-inventory"))]
mod kani_harnesses;

#[cfg(test)]
mod tests;

/// Workspace marker - all actual code is in crates/
pub const WORKSPACE_NAME: &str = "velvet_ballistics";
