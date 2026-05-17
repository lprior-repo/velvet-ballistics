use std::ffi::OsString;
use std::path::{Path, PathBuf};

use vb_ui_snapshot::{BASELINE_HEIGHT, BASELINE_WIDTH, checks, tokens::UiTokens};

use crate::ai_profile::{
    cmd_ai_deep, cmd_ai_fast, cmd_ai_release, fail_on_partial_profile_evidence, validate_bead_id,
};
use crate::evidence;
use crate::shell::{render_top_level_help, render_top_level_version, run_required_command};
use crate::ui_overlap::{check_overlap_for_screen, cmd_ui_overlap_check};
use crate::ui_snapshot::{capture_fixture, cmd_ui_snapshot};
use crate::ui_snapshot_render::{
    draw_fixture_marker, draw_token_swatches, fill_rect, generate_fixture_screenshot, rgba_from_hex,
};
use crate::ui_tokens_cmd::cmd_ui_tokens;

const TOKENS_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../design/tokens/velvet_ui_tokens.toml"
);

#[test]
fn command_shell_argument_help_and_required_command_paths_work() {
    assert_eq!(
        normalize_args_for_test(vec!["xtask".into(), "--".into(), "ui-snapshot".into()]),
        vec![OsString::from("xtask"), OsString::from("ui-snapshot")]
    );
    assert_eq!(
        normalize_args_for_test(vec!["xtask".into(), "ui-snapshot".into(), "--".into()]),
        vec![
            OsString::from("xtask"),
            OsString::from("ui-snapshot"),
            OsString::from("--")
        ]
    );
    assert_eq!(unit_result_label(render_top_level_help()), "accepted");
    assert_eq!(unit_result_label(render_top_level_version()), "accepted");
    assert_eq!(
        unit_result_label(run_required_command(xtask::CommandFamily::AiPlan)),
        "accepted"
    );
}

#[test]
fn bead_id_and_profile_guards_reject_traversal_and_unknown_release_beads() {
    assert_eq!(unit_result_label(validate_bead_id("vb-kkvb_1")), "accepted");
    assert_eq!(error_text(validate_bead_id("")), "Invalid bead id: ");
    assert_eq!(
        error_text(validate_bead_id("../vb-kkvb")),
        "Invalid bead id: ../vb-kkvb"
    );
    assert_eq!(
        error_text(cmd_ai_release(None)),
        "unknown ai-release bead id: <missing>"
    );
    assert_eq!(
        error_text(cmd_ai_release(Some("vb-other"))),
        "unknown ai-release bead id: vb-other"
    );
}

#[test]
fn ai_profiles_emit_yaml_or_evidence_for_valid_scopes() {
    assert_eq!(unit_result_label(cmd_ai_fast(None)), "accepted");
    assert_eq!(unit_result_label(cmd_ai_deep(None)), "accepted");
    let bead_id = "vb_main_test_ai_fast";
    assert_eq!(unit_result_label(cmd_ai_fast(Some(bead_id))), "accepted");
    assert_eq!(
        PathBuf::from(".evidence")
            .join(bead_id)
            .join("ai-fast.yaml")
            .exists(),
        true
    );
    assert_eq!(
        error_text(cmd_ai_fast(Some("../bad"))),
        "Invalid bead id: ../bad"
    );
}

#[test]
fn snapshot_render_helpers_preserve_deterministic_pixels() {
    let mut img = image::RgbaImage::new(BASELINE_WIDTH, BASELINE_HEIGHT);
    let tokens = UiTokens::default();
    assert_eq!(rgba_from_hex("#102030").0, [16, 32, 48, 255]);
    assert_eq!(rgba_from_hex("bad").0, [255, 255, 255, 255]);
    let red = image::Rgba([255, 0, 0, 255]);
    fill_rect(&mut img, 1, 1, 20, 20, red);
    assert_eq!(*img.get_pixel(1, 1), red);
    draw_token_swatches(&mut img, &tokens);
    assert_eq!(*img.get_pixel(320, 128), rgba_from_hex(&tokens.surface));
    draw_fixture_marker(&mut img, "execution_overview", &tokens);
    assert_eq!(*img.get_pixel(384, 240), rgba_from_hex(&tokens.running));
}

#[test]
fn snapshot_commands_capture_fixtures_and_reject_bad_selection() {
    let dir = must_tempdir();
    let output_arg = dir.path().to_string_lossy().to_string();
    assert_eq!(
        error_text(cmd_ui_snapshot(false, None, None, output_arg.clone())),
        "Must specify --all or --fixture <name>"
    );
    must_ok(cmd_ui_snapshot(
        false,
        Some("execution_overview".to_string()),
        None,
        output_arg,
    ));
    assert_eq!(dir.path().join("execution_overview.png").exists(), true);
    assert_eq!(dir.path().join("ui_snapshot_report.yaml").exists(), true);

    let mut report = vb_ui_snapshot::UiSnapshotReport::new();
    let error = error_text(capture_fixture("missing_fixture", dir.path(), &mut report));
    assert_eq!(
        error.contains("Failed to load fixture: missing_fixture"),
        true
    );
}

#[test]
fn generated_fixture_screenshot_has_baseline_dimensions() {
    let dir = must_tempdir();
    let png_path = dir.path().join("execution_overview.png");
    must_ok(generate_fixture_screenshot(
        &png_path,
        "execution_overview",
        None,
    ));
    assert_eq!(png_path.exists(), true);
    assert_eq!(png_dimensions_label(&png_path), "1920x1080");
}

#[test]
fn ui_token_command_writes_checks_and_reports_parse_errors() {
    let dir = must_tempdir();
    let output = dir.path().join("tokens_generated.rs");
    let output_arg = output.to_string_lossy().to_string();
    must_ok(cmd_ui_tokens(
        TOKENS_FILE,
        &output_arg,
        Some("json".to_string()),
        false,
    ));
    assert_eq!(must_read_to_string(&output).contains("TokenColors"), true);
    assert_eq!(
        unit_result_label(cmd_ui_tokens(TOKENS_FILE, &output_arg, None, true)),
        "accepted"
    );
    must_write(&output, "stale");
    assert_eq!(
        error_text(cmd_ui_tokens(TOKENS_FILE, &output_arg, None, true))
            .contains("Generated UI tokens are stale"),
        true
    );
}

#[test]
fn ui_overlap_check_rejects_missing_or_uninspectable_screens() {
    let dir = must_tempdir();
    let input_arg = dir.path().to_string_lossy().to_string();
    assert_eq!(
        error_text(cmd_ui_overlap_check(false, None, &input_arg)),
        "Must specify --all or --screen <name>"
    );
    assert_eq!(
        error_text(cmd_ui_overlap_check(
            false,
            Some("missing_screen".to_string()),
            &input_arg
        )),
        "UI overlap check failed"
    );
    assert_eq!(
        bool_result_label(check_overlap_for_screen(dir.path(), "missing_screen")),
        "false"
    );
}

#[test]
fn partial_profile_evidence_fails_closed_when_yaml_set_is_incomplete() {
    let dir = must_tempdir();
    assert_eq!(
        unit_result_label(fail_on_partial_profile_evidence(
            dir.path(),
            evidence::GateProfile::AiFast
        )),
        "accepted"
    );
    must_write(&dir.path().join("fmt.yaml"), "gate: fmt");
    assert_eq!(
        error_text(fail_on_partial_profile_evidence(
            dir.path(),
            evidence::GateProfile::AiFast
        ))
        .contains("Missing required gate evidence"),
        true
    );
}

fn normalize_args_for_test(args: Vec<OsString>) -> Vec<OsString> {
    args.into_iter()
        .enumerate()
        .filter_map(|(index, arg)| {
            let is_legacy_separator = index == 1 && arg == "--";
            (!is_legacy_separator).then_some(arg)
        })
        .collect()
}

fn must_tempdir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"))
}

fn must_ok(result: anyhow::Result<()>) {
    if let Err(error) = result {
        panic!("unexpected error: {error}");
    }
}

fn error_text(result: anyhow::Result<()>) -> String {
    result.map_or_else(|error| error.to_string(), |()| "<ok>".to_string())
}

fn unit_result_label(result: anyhow::Result<()>) -> String {
    result.map_or_else(|error| error.to_string(), |()| "accepted".to_string())
}

fn bool_result_label(result: anyhow::Result<bool>) -> String {
    result.map_or_else(|error| error.to_string(), |value| value.to_string())
}

fn png_dimensions_label(path: &Path) -> String {
    checks::validate_png_dimensions(path).map_or_else(
        |error| error.to_string(),
        |(width, height)| format!("{width}x{height}"),
    )
}

fn must_read_to_string(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn must_write(path: &Path, contents: &str) {
    std::fs::write(path, contents)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()))
}
