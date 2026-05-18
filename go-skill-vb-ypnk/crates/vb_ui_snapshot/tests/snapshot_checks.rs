//! Tests for snapshot module: run_snapshot_command_for_fixture success path
//! and constants from lib.rs.

use vb_ui_snapshot::snapshot::{SnapshotArtifact, run_snapshot_command_for_fixture};
use vb_ui_snapshot::{
    BASELINE_HEIGHT, BASELINE_WIDTH, CHIP_RADIUS, COLOR_DRIFT_THRESHOLD, OUTER_MARGIN,
    REQUIRED_FIXTURES, SIDEBAR_WIDTH, TOP_BAR_HEIGHT, demo_fixture_names,
};

//
// run_snapshot_command_for_fixture — success path
//

#[test]
fn run_snapshot_command_for_fixture_returns_artifact_on_success() {
    let result = run_snapshot_command_for_fixture(
        "execution_overview",
        "makepad-render --fixture execution_overview",
    );
    assert!(result.is_ok());
    let artifact = result.unwrap();
    assert_eq!(
        artifact.png_path,
        "target/ui_snapshots/execution_overview.png"
    );
}

#[test]
fn run_snapshot_command_for_fixture_returns_error_on_exit_code_17() {
    let result = run_snapshot_command_for_fixture(
        "execution_overview",
        "makepad-render --exit-code 17 --stderr 'render failed'",
    );
    assert!(result.is_err());
}

#[test]
fn run_snapshot_command_for_fixture_path_includes_fixture_id() {
    let result =
        run_snapshot_command_for_fixture("workflow_graph_authoring", "makepad-render").expect("ok");
    assert!(result.png_path.contains("workflow_graph_authoring"));
}

#[test]
fn run_snapshot_command_for_fixture_multiple_render_calls() {
    let fixtures = [
        "execution_overview",
        "execution_details",
        "verification_certificate",
    ];
    for fixture in fixtures {
        let result = run_snapshot_command_for_fixture(fixture, "makepad-render");
        assert!(result.is_ok(), "failed for {fixture}");
        assert!(result.unwrap().png_path.contains(fixture));
    }
}

//
// Constants from lib.rs
//

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
fn color_drift_threshold_is_0_03() {
    assert_eq!(COLOR_DRIFT_THRESHOLD, 0.03);
}

//
// demo_fixture_names
//

#[test]
fn demo_fixture_names_returns_canonical_eight() {
    let names = demo_fixture_names();
    assert_eq!(names.len(), 8);
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
fn demo_fixture_names_matches_required_fixtures() {
    assert_eq!(demo_fixture_names().as_slice(), REQUIRED_FIXTURES);
}

//
// SnapshotArtifact struct
//

#[test]
fn snapshot_artifact_has_public_fields() {
    let artifact = SnapshotArtifact {
        png_path: "target/ui_snapshots/test.png".to_string(),
    };
    assert_eq!(artifact.png_path, "target/ui_snapshots/test.png");
}
