#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

pub mod checks;
pub mod error;
pub mod fixtures;
pub mod report;
pub mod tokens;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_fixture_names_returns_all_required_fixtures() {
        let names = demo_fixture_names();
        assert_eq!(names.len(), REQUIRED_FIXTURES.len());
        for name in REQUIRED_FIXTURES {
            assert!(names.contains(&name), "missing fixture: {name}");
        }
    }

    #[test]
    fn demo_fixture_names_exact_list() {
        let names = demo_fixture_names();
        assert!(names.contains(&"execution_overview"));
        assert!(names.contains(&"workflow_graph_authoring"));
        assert!(names.contains(&"execution_details"));
        assert!(names.contains(&"verification_certificate"));
        assert!(names.contains(&"replay_theater"));
        assert!(names.contains(&"incident_failure"));
        assert!(names.contains(&"action_registry"));
        assert!(names.contains(&"storage_doctor_ai_context"));
    }

    #[test]
    fn required_fixtures_is_8_items() {
        assert_eq!(REQUIRED_FIXTURES.len(), 8);
    }

    #[test]
    fn baseline_dimensions_are_1920x1080() {
        assert_eq!(BASELINE_WIDTH, 1920);
        assert_eq!(BASELINE_HEIGHT, 1080);
    }

    #[test]
    fn outer_margin_is_32() {
        assert_eq!(OUTER_MARGIN, 32);
    }

    #[test]
    fn sidebar_width_is_246() {
        assert_eq!(SIDEBAR_WIDTH, 246);
    }

    #[test]
    fn top_bar_height_is_78() {
        assert_eq!(TOP_BAR_HEIGHT, 78);
    }

    #[test]
    fn chip_radius_is_10() {
        assert_eq!(CHIP_RADIUS, 10.0);
    }

    #[test]
    fn color_drift_threshold_is_3_percent() {
        assert_eq!(COLOR_DRIFT_THRESHOLD, 0.03);
    }

    #[test]
    fn demo_fixture_names_produces_vec_not_static_slice() {
        let names = demo_fixture_names();
        // Should be an owned Vec, not a &'static str slice
        drop(names);
    }

    #[test]
    fn all_public_constants_are_nonzero() {
        assert!(BASELINE_WIDTH > 0);
        assert!(BASELINE_HEIGHT > 0);
        assert!(OUTER_MARGIN > 0);
        assert!(SIDEBAR_WIDTH > 0);
        assert!(TOP_BAR_HEIGHT > 0);
        assert!(CHIP_RADIUS > 0.0);
        assert!(COLOR_DRIFT_THRESHOLD > 0.0);
    }
}
