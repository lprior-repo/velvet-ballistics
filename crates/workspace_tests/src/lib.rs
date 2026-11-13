#![forbid(unsafe_code)]
//! Velvet Ballistics Workspace Tests
//!
//! Test infrastructure and workspace-level testing support.

pub mod acceptance_catalog;
#[path = "../../boundary_inventory/src/api.rs"]
mod api;
pub mod bdd_runner;
pub mod boundary_inventory;
#[path = "../../boundary_inventory/src/inventory.rs"]
mod inventory;
#[path = "../../boundary_inventory/src/parser.rs"]
mod parser;
pub mod quality;
#[path = "../../boundary_inventory/src/record.rs"]
mod record;
#[path = "../../boundary_inventory/src/status.rs"]
mod status;
pub mod test_util;
#[path = "../../boundary_inventory/src/types.rs"]
mod types;
#[path = "../../boundary_inventory/src/validation.rs"]
mod validation;

pub use test_util::TestSetupError;
