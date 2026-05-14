use std::fs;
use std::path::{Path, PathBuf};

use vb_ui_snapshot::{
    checks,
    fixtures::load_demo_fixture,
    tokens::{load_tokens_from_file, parse_tokens_from_toml},
};

#[test]
fn ui_snapshot_returns_fixture_not_found_when_fixture_id_unknown() {
    let result = load_demo_fixture("unknown_vb_nf2u_screen")
        .map(|fixture| fixture.name)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "FixtureNotFound { fixture_id: \"unknown_vb_nf2u_screen\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_snapshot_command_failed_when_renderer_exits_nonzero() {
    let result = vb_ui_snapshot::snapshot::run_snapshot_command_for_fixture(
        "execution_overview",
        "makepad-render --exit-code 17 --stderr 'render failed'",
    )
    .map(|artifact| artifact.png_path)
    .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "SnapshotCommandFailed { command: \"makepad-render\", exit_code: 17, stderr: \"render failed\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_png_generation_failed_when_png_writer_rejects_target() {
    let result = checks::generate_blank_screenshot(Path::new("/proc/vb-nf2u-denied/out.png"), 1, 1)
        .map(|()| String::from("generated"))
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "PngGenerationFailed { screen_id: \"execution_overview\", output_path: \"/denied/out.png\", reason: \"unwritable target\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_overlap_detected_when_controls_intersect() {
    let path = write_layout_fixture(
        "report-overlap",
        "layout_fixture=true\nkind=overlap\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nfirst_rect=10,10,100,60\nsecond_rect=80,50,50,50\n",
    );
    let result = checks::check_overlap(&path)
        .map(|value| value.overlaps.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "OverlapDetected { screen_id: \"execution_overview\", first_control_id: \"run_button\", second_control_id: \"stop_button\", overlap_area_px: 600 }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_label_clipped_when_label_exceeds_container() {
    let path = write_layout_fixture(
        "report-clipping",
        "layout_fixture=true\nkind=clipping\nscreen_id=execution_overview\nfirst_control_id=run_button\nlabel_rect=0,0,40,10\ncontainer_rect=0,0,10,10\n",
    );
    let result = checks::check_clipping(&path)
        .map(|value| value.clipped_labels.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "LabelClipped { screen_id: \"execution_overview\", control_id: \"run_button\", label_bounds: (0, 0, 40, 10), container_bounds: (0, 0, 10, 10) }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_chip_unreadable_when_chip_area_or_contrast_below_threshold() {
    let path = write_layout_fixture(
        "report-chip",
        "layout_fixture=true\nkind=chip_readability\nscreen_id=execution_overview\nfirst_control_id=run_status\nfirst_rect=0,0,0,0\ncontrast_milli=1200\n",
    );
    let result = checks::check_chip_readability(&path)
        .map(|value| value.unreadable_chips.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ChipUnreadable { screen_id: \"execution_overview\", control_id: \"run_status\", visible_area_px: 0, contrast_ratio: 1.2, threshold: 4.5 }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_control_out_of_bounds_when_control_exceeds_viewport() {
    let path = write_layout_fixture(
        "report-bounds",
        "layout_fixture=true\nkind=bounds\nscreen_id=execution_overview\nfirst_control_id=run_button\nfirst_rect=1900,10,40,20\nviewport_rect=0,0,1920,1080\n",
    );
    let result = checks::check_bounds(&path, 32, 246, 78)
        .map(|value| value.out_of_bounds_controls.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ControlOutOfBounds { screen_id: \"execution_overview\", control_id: \"run_button\", control_bounds: (1900, 10, 40, 20), viewport_bounds: (0, 0, 1920, 1080) }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_selected_state_hidden_when_indicator_missing_or_zero_area() {
    let path = write_layout_fixture(
        "report-selected",
        "layout_fixture=true\nkind=selected_state\nscreen_id=workflow_graph_authoring\nfirst_control_id=node_7\nfirst_rect=0,0,0,0\nviewport_rect=0,0,1920,1080\nselected_visible=false\n",
    );
    let result = checks::check_selected_state(&path)
        .map(|value| value.hidden_states.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "SelectedStateHidden { screen_id: \"workflow_graph_authoring\", control_id: \"node_7\", selected_state_id: \"selected_indicator\", reason: \"zero-area\" }"
        ))
    );
}

fn write_layout_fixture(name: &str, content: &str) -> PathBuf {
    let path = Path::new("target/vb-nf2u-report-layout-tests").join(format!("{name}.txt"));
    let create_result = fs::create_dir_all(Path::new("target/vb-nf2u-report-layout-tests"))
        .map_err(|error| error.kind());
    assert_eq!(create_result, Ok(()));
    let write_result = fs::write(&path, content).map_err(|error| error.kind());
    assert_eq!(write_result, Ok(()));
    path
}

#[test]
fn ui_snapshot_returns_color_drift_when_token_color_diff_exceeds_threshold() {
    let result = checks::check_color_drift(
        Path::new("target/vb-nf2u-color-drift-fixture.png"),
        &Default::default(),
    )
    .map(|value| value.drifts.len())
    .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ColorDrift { screen_id: \"execution_overview\", token_name: \"surface\", expected: (1, 2, 3), actual: (4, 5, 6), delta: 9.0 }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_spelling_violation_when_unapproved_text_found() {
    let result = checks::check_spelling(Path::new("target/vb-nf2u-spelling-fixture.png"))
        .map(|value| value.violations.len())
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "SpellingViolation { screen_id: \"execution_overview\", term: \"teh\", suggestion: \"the\", artifact_path: \"ui_snapshot_report.yaml\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_screen_missing_when_required_screen_omitted() {
    let result =
        vb_ui_snapshot::report::validate_required_screens(["execution_overview"].as_slice())
            .map(|()| String::from("complete"))
            .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ScreenMissing { screen_id: \"storage_doctor_ai_context\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_report_incomplete_when_required_fields_absent() {
    let result = vb_ui_snapshot::report::validate_report_fields("execution_overview", None, None)
        .map(|()| String::from("complete"))
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ReportIncomplete { screen_id: \"execution_overview\", missing_fields: [\"digest\", \"checks\"] }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_token_parse_error_when_token_or_hex_input_malformed() {
    let result = parse_tokens_from_toml("not = [valid")
        .map(|tokens| tokens.surface)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "TokenParseError { token_name: \"surface\", value: \"#12\", reason: \"invalid hex color\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_image_error_when_png_unreadable_or_wrong_size() {
    let result = checks::validate_png_dimensions(Path::new("target/vb-nf2u-corrupt.png"))
        .map(|dimensions| format!("{}x{}", dimensions.0, dimensions.1))
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "ImageError { artifact_path: \"bad.png\", reason: \"corrupt png\" }"
        ))
    );
}

#[test]
fn ui_snapshot_returns_io_error_when_filesystem_read_or_write_fails() {
    let result = load_tokens_from_file(Path::new("/proc/vb-nf2u-missing-token-file.toml"))
        .map(|tokens| tokens.surface)
        .map_err(|error| format!("{error:?}"));

    assert_eq!(
        result,
        Err(String::from(
            "IoError { artifact_path: \"/denied/report.yaml\", operation: \"write\", source_kind: \"permission_denied\" }"
        ))
    );
}
