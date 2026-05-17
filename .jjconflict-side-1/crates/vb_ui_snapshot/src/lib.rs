#![forbid(unsafe_code)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

pub mod checks;
pub mod error;
pub mod fixtures;
pub mod layout_kernel;
pub mod redaction;
pub mod report;
pub mod snapshot;
pub mod tokens;

#[cfg(kani)]
mod kani_harnesses {
    include!("../kani/inventory.rs");
    include!("../kani/layout_predicates.rs");
}

pub use error::UiSnapshotError;
pub use report::{CheckKind, CheckResult, ScreenResult, UiSnapshotReport};

pub const REQUIRED_FIXTURES: &[&str] = &[
    "execution_overview",
    "workflow_graph_authoring",
    "execution_details",
    "verification_certificate",
    "replay_theater",
    "incident_failure",
    "action_registry",
    "storage_doctor_ai_context",
];

pub const BASELINE_WIDTH: u32 = 1920;
pub const BASELINE_HEIGHT: u32 = 1080;
pub const OUTER_MARGIN: u32 = 32;
pub const SIDEBAR_WIDTH: u32 = 246;
pub const TOP_BAR_HEIGHT: u32 = 78;
pub const CHIP_RADIUS: f32 = 10.0;
pub const COLOR_DRIFT_THRESHOLD: f32 = 0.03;

pub fn demo_fixture_names() -> Vec<&'static str> {
    REQUIRED_FIXTURES.to_vec()
}
