//! Tests for report pub fns: UiSnapshotReport lifecycle, CheckKind Display,
//! screen/check result builders, validate_required_screens, validate_report_fields.

use vb_ui_snapshot::report::{
    CheckKind, UiSnapshotReport, make_fail_result, make_pass_result, make_screen_result,
    validate_report_fields, validate_required_screens,
};

//
// UiSnapshotReport lifecycle
//

#[test]
fn report_new_sets_status_pass() {
    let report = UiSnapshotReport::new();
    assert_eq!(report.status, "pass");
    assert_eq!(report.total_screens, 0);
    assert_eq!(report.passed_screens, 0);
    assert_eq!(report.failed_screens, 0);
    assert!(report.screens.is_empty());
}

#[test]
fn add_screen_sets_status_to_fail_when_screen_fails() {
    let mut report = UiSnapshotReport::new();
    report.add_screen(make_screen_result(
        "execution_overview",
        vec![make_fail_result(CheckKind::ColorDrift, "token drift")],
    ));
    assert_eq!(report.status, "fail");
}

#[test]
fn add_screen_keeps_status_pass_when_all_screens_pass() {
    let mut report = UiSnapshotReport::new();
    report.add_screen(make_screen_result(
        "execution_overview",
        vec![make_pass_result(CheckKind::Overlap)],
    ));
    assert_eq!(report.status, "pass");
}

#[test]
fn finalize_counts_screens_correctly() {
    let mut report = UiSnapshotReport::new();
    report.add_screen(make_screen_result(
        "execution_overview",
        vec![make_pass_result(CheckKind::Overlap)],
    ));
    report.add_screen(make_screen_result(
        "execution_details",
        vec![make_fail_result(CheckKind::ColorDrift, "drift")],
    ));
    report.add_screen(make_screen_result(
        "workflow_graph_authoring",
        vec![make_pass_result(CheckKind::Spelling)],
    ));
    report.finalize();

    assert_eq!(report.total_screens, 3);
    assert_eq!(report.passed_screens, 2);
    assert_eq!(report.failed_screens, 1);
}

#[test]
fn finalize_counts_zero_passed_when_all_fail() {
    let mut report = UiSnapshotReport::new();
    report.add_screen(make_screen_result(
        "execution_overview",
        vec![make_fail_result(CheckKind::ColorDrift, "drift")],
    ));
    report.finalize();
    assert_eq!(report.passed_screens, 0);
    assert_eq!(report.failed_screens, 1);
}

#[test]
fn report_status_stays_fail_once_set() {
    let mut report = UiSnapshotReport::new();
    report.add_screen(make_screen_result(
        "execution_overview",
        vec![make_fail_result(CheckKind::Overlap, "overlap")],
    ));
    report.add_screen(make_screen_result(
        "execution_details",
        vec![make_pass_result(CheckKind::Spelling)],
    ));
    assert_eq!(report.status, "fail");
}

//
// CheckKind Display
//

#[test]
fn check_kind_display_overlap() {
    assert_eq!(format!("{}", CheckKind::Overlap), "overlap_check");
}

#[test]
fn check_kind_display_clipping() {
    assert_eq!(format!("{}", CheckKind::Clipping), "clipping_check");
}

#[test]
fn check_kind_display_chip_readability() {
    assert_eq!(
        format!("{}", CheckKind::ChipReadability),
        "chip_readability_check"
    );
}

#[test]
fn check_kind_display_bounds() {
    assert_eq!(format!("{}", CheckKind::Bounds), "bounds_check");
}

#[test]
fn check_kind_display_selected_state() {
    assert_eq!(
        format!("{}", CheckKind::SelectedState),
        "selected_state_check"
    );
}

#[test]
fn check_kind_display_redaction() {
    assert_eq!(format!("{}", CheckKind::Redaction), "redaction_check");
}

#[test]
fn check_kind_display_color_drift() {
    assert_eq!(format!("{}", CheckKind::ColorDrift), "color_drift_check");
}

#[test]
fn check_kind_display_spelling() {
    assert_eq!(format!("{}", CheckKind::Spelling), "spelling_check");
}

#[test]
fn check_kind_display_png_validity() {
    assert_eq!(format!("{}", CheckKind::PngValidity), "png_validity_check");
}

//
// make_pass_result
//

#[test]
fn make_pass_result_sets_passed_true_and_detail_none() {
    let result = make_pass_result(CheckKind::Bounds);
    assert_eq!(result.kind, CheckKind::Bounds);
    assert!(result.passed);
    assert!(result.detail.is_none());
}

#[test]
fn make_pass_result_preserves_check_kind() {
    for kind in [
        CheckKind::Overlap,
        CheckKind::Clipping,
        CheckKind::ChipReadability,
        CheckKind::Bounds,
        CheckKind::SelectedState,
        CheckKind::Redaction,
        CheckKind::ColorDrift,
        CheckKind::Spelling,
        CheckKind::PngValidity,
    ] {
        let result = make_pass_result(kind);
        assert_eq!(result.kind, kind);
    }
}

//
// make_fail_result
//

#[test]
fn make_fail_result_sets_passed_false_and_detail() {
    let result = make_fail_result(CheckKind::ColorDrift, "surface token drifted 5%");
    assert!(!result.passed);
    assert_eq!(result.detail.as_deref(), Some("surface token drifted 5%"));
}

#[test]
fn make_fail_result_preserves_check_kind() {
    let result = make_fail_result(CheckKind::Spelling, "teh");
    assert_eq!(result.kind, CheckKind::Spelling);
}

//
// make_screen_result
//

#[test]
fn make_screen_result_passes_when_all_checks_pass() {
    let screen = make_screen_result(
        "execution_overview",
        vec![
            make_pass_result(CheckKind::Overlap),
            make_pass_result(CheckKind::Clipping),
        ],
    );
    assert!(screen.passed);
    assert_eq!(screen.screen_name, "execution_overview");
    assert_eq!(screen.checks.len(), 2);
    assert!(screen.png_path.is_none());
}

#[test]
fn make_screen_result_fails_when_any_check_fails() {
    let screen = make_screen_result(
        "execution_overview",
        vec![
            make_pass_result(CheckKind::Overlap),
            make_fail_result(CheckKind::ColorDrift, "drift"),
        ],
    );
    assert!(!screen.passed);
}

#[test]
fn make_screen_result_passes_when_no_checks() {
    let screen = make_screen_result("execution_overview", vec![]);
    assert!(screen.passed);
}

//
// validate_required_screens
//

#[test]
fn validate_required_screens_returns_ok_when_all_canonical_present() {
    let screens = [
        "execution_overview",
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "action_registry",
        "storage_doctor_ai_context",
    ];
    assert!(validate_required_screens(&screens).is_ok());
}

#[test]
fn validate_required_screens_returns_err_when_execution_overview_missing() {
    let screens = [
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "action_registry",
        "storage_doctor_ai_context",
    ];
    let result = validate_required_screens(&screens);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        vb_ui_snapshot::UiSnapshotError::ScreenMissing {
            expected_screen: _,
            ..
        }
    ));
    // Reverse iteration finds execution_overview missing first
    assert!(format!("{err}").contains("execution_overview"));
}

#[test]
fn validate_required_screens_returns_err_when_action_registry_missing() {
    let screens = [
        "execution_overview",
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "storage_doctor_ai_context",
    ];
    let result = validate_required_screens(&screens);
    assert!(result.is_err());
}

#[test]
fn validate_required_screens_returns_err_when_duplicate_present_but_missing_others() {
    let screens = [
        "execution_overview",
        "execution_overview", // duplicate
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "action_registry",
        // storage_doctor_ai_context missing
    ];
    let result = validate_required_screens(&screens);
    assert!(result.is_err());
}

#[test]
fn validate_required_screens_empty_slice_fails() {
    let result = validate_required_screens(&[] as &[&str]);
    assert!(result.is_err());
}

//
// validate_report_fields
//

#[test]
fn validate_report_fields_returns_ok_when_both_fields_present() {
    let checks = [make_pass_result(CheckKind::Overlap)];
    let result = validate_report_fields("execution_overview", Some("digest123"), Some(&checks));
    assert!(result.is_ok());
}

#[test]
fn validate_report_fields_returns_err_when_digest_missing() {
    let checks = [make_pass_result(CheckKind::Overlap)];
    let result = validate_report_fields("execution_overview", None, Some(&checks));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        vb_ui_snapshot::UiSnapshotError::ReportIncomplete { .. }
    ));
    assert!(format!("{err}").contains("digest"));
}

#[test]
fn validate_report_fields_returns_err_when_checks_missing() {
    let result = validate_report_fields("execution_overview", Some("digest123"), None);
    assert!(result.is_err());
    assert!(format!("{}", result.unwrap_err()).contains("checks"));
}

#[test]
fn validate_report_fields_returns_err_when_both_missing() {
    let result = validate_report_fields("execution_overview", None, None);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_str = format!("{err}");
    assert!(err_str.contains("execution_overview"));
    // Display format uses "Report incomplete"
    assert!(err_str.contains("Report incomplete"));
    assert!(err_str.contains("digest"));
    assert!(err_str.contains("checks"));
}

#[test]
fn validate_report_fields_ignores_actual_values_content() {
    // Just verify both present is sufficient — values are not inspected
    let result = validate_report_fields("foo", Some("x"), Some(&[]));
    assert!(result.is_ok());
}

//
// Default for UiSnapshotReport
//

#[test]
fn report_default_equivalent_to_new() {
    let default_report = UiSnapshotReport::default();
    let new_report = UiSnapshotReport::new();
    assert_eq!(default_report.status, new_report.status);
    assert_eq!(default_report.screens.len(), new_report.screens.len());
}
