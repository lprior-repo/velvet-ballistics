use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, MutexGuard, OnceLock};
use xtask::evidence::{
    CoreParityClaim, DiagnosticCode, FixtureBackedState, FixtureGate, FixtureStatus,
    ParsedAiReleaseDocument, ParsedNegativeFixtureDocument, ParsedOverlapExpectedFailure,
    ParsedOverlapFixtureEvidence, ParsedSecretExpectedFailure, ParsedSecretFixtureEvidence,
    ParsedSnapshotDocument, XtaskCommandDiagnostic, parse_ai_release_document,
    parse_negative_fixture_document, parse_snapshot_document,
};

const BEAD_ID: &str = "vb-nf2u";
const AI_RELEASE_YAML: &str = ".evidence/vb-nf2u/ai-release.yaml";
const UI_SNAPSHOT_REPORT_YAML: &str = ".evidence/vb-nf2u/ui_snapshots/ui_snapshot_report.yaml";
const NEGATIVE_FIXTURES_TXT: &str = ".evidence/vb-nf2u/negative-fixtures.txt";
const NEGATIVE_FIXTURE_DIR: &str = "target/vb-nf2u-negative-fixtures";

const RAW_SENTINEL: &str = "vb_nf2u_secret_sentinel";
const RAW_API_KEY: &str = "sk_test_vb_nf2u_raw_secret";
const RAW_TOKEN: &str = "Bearer vb_nf2u_token";
const RAW_PASSWORD: &str = "password=hunter2";
const RAW_IDEMPOTENCY_KEY: &str = "Idempotency-Key: idem_vb_nf2u_secret";
const RAW_TAINT: &str = "tainted_fixture_value_vb_nf2u";

#[test]
fn all_eight_screens_pass_reachability_and_overlap_gates() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    let output = run_ai_release_for_vb_nf2u()?;

    // When
    assert_command_succeeded(&output);
    let ai_release = read_required_artifact(AI_RELEASE_YAML)?;
    let snapshot_report = read_required_artifact(UI_SNAPSHOT_REPORT_YAML)?;

    // Then
    assert_ui_subgates_are_exact(&ai_release)?;
    assert_snapshot_inventory_is_exact(&snapshot_report)?;
    assert_screen_has_required_checks(&snapshot_report, "execution_overview")?;
    assert_screen_has_required_checks(&snapshot_report, "workflow_graph_authoring")?;
    assert_screen_has_required_checks(&snapshot_report, "execution_details")?;
    assert_screen_has_required_checks(&snapshot_report, "verification_certificate")?;
    assert_screen_has_required_checks(&snapshot_report, "replay_theater")?;
    assert_screen_has_required_checks(&snapshot_report, "incident_failure")?;
    assert_screen_has_required_checks(&snapshot_report, "action_registry")?;
    assert_screen_has_required_checks(&snapshot_report, "storage_doctor_ai_context")?;
    assert_fixture_evidence_disclaims_core_parity(&ai_release, &snapshot_report)?;
    drop(fixture_guard);
    Ok(())
}

#[test]
fn secret_values_are_redacted_in_every_screen() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    let output = run_ai_release_for_vb_nf2u()?;

    // When
    assert_command_succeeded(&output);
    let ai_release = read_required_artifact(AI_RELEASE_YAML)?;
    let snapshot_report = read_required_artifact(UI_SNAPSHOT_REPORT_YAML)?;
    let negative_evidence = read_required_artifact(NEGATIVE_FIXTURES_TXT)?;
    let evidence_text = format!("{ai_release}\n{snapshot_report}\n{negative_evidence}");

    // Then
    assert_no_raw_denied_values_are_emitted(&evidence_text);
    assert_redaction_coverage_for_screen(&ai_release, "execution_overview")?;
    assert_redaction_coverage_for_screen(&ai_release, "workflow_graph_authoring")?;
    assert_redaction_coverage_for_screen(&ai_release, "execution_details")?;
    assert_redaction_coverage_for_screen(&ai_release, "verification_certificate")?;
    assert_redaction_coverage_for_screen(&ai_release, "replay_theater")?;
    assert_redaction_coverage_for_screen(&ai_release, "incident_failure")?;
    assert_redaction_coverage_for_screen(&ai_release, "action_registry")?;
    assert_redaction_coverage_for_screen(&ai_release, "storage_doctor_ai_context")?;
    drop(fixture_guard);
    Ok(())
}

#[test]
fn intentional_overlap_fixture_fails_gate() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_overlap_fixture(
        "screen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nrun_button={x:10,y:10,width:100,height:60}\nstop_button={x:80,y:40,width:50,height:50}\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=failed\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(NEGATIVE_FIXTURES_TXT)?;
    let entry = parse_negative_evidence(&negative_evidence)?.overlap;
    assert_default_overlap_entry(&entry)?;
    drop(fixture_guard);
    Ok(())
}

#[test]
fn intentional_secret_fixture_fails_redaction_gate() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_secret_fixture(
        "fixture_id=intentional_secret_fixture\nraw_sentinel=vb_nf2u_secret_sentinel\napi_key=sk_test_vb_nf2u_raw_secret\ntoken=Bearer vb_nf2u_token\npassword=password=hunter2\nidempotency=Idempotency-Key: idem_vb_nf2u_secret\ntaint=tainted_fixture_value_vb_nf2u\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(NEGATIVE_FIXTURES_TXT)?;
    let entry = parse_negative_evidence(&negative_evidence)?.secret;
    let expected = require_secret_expected(&entry)?;
    assert_eq!(expected.status, FixtureStatus::ExpectedFailed);
    assert_eq!(expected.diagnostic_code, DiagnosticCode::Redaction);
    assert_eq!(expected.secret_class.as_str(), "api_key");
    assert_eq!(expected.redacted_sample.as_str(), "[REDACTED:api_key]");
    assert_no_raw_denied_values_are_emitted(&negative_evidence);
    drop(fixture_guard);
    Ok(())
}

#[test]
fn overlap_negative_fixture_is_consumed_by_command_boundary() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_overlap_fixture(
        "screen_id=execution_overview\nfirst_control_id=changed_run_button\nsecond_control_id=changed_stop_button\nchanged_run_button={x:1,y:1,width:10,height:10}\nchanged_stop_button={x:5,y:5,width:20,height:20}\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=25\nbounds={ x: 1, y: 1, width: 10, height: 10 }\nactual_status=failed\nfixture_nonce=overlap_fixture_must_be_read\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(NEGATIVE_FIXTURES_TXT)?;
    let entry = parse_negative_evidence(&negative_evidence)?.overlap;
    assert_changed_overlap_entry(&entry)?;
    drop(fixture_guard);
    Ok(())
}

#[test]
fn secret_negative_fixture_is_consumed_by_command_boundary() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_secret_fixture(
        "fixture_id=intentional_secret_fixture\nraw_sentinel=vb_nf2u_secret_sentinel\napi_key=sk_test_vb_nf2u_raw_secret_CHANGED\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\nfixture_nonce=secret_fixture_must_be_read\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_command_succeeded(&output);
    let negative_evidence = read_required_artifact(NEGATIVE_FIXTURES_TXT)?;
    let entry = parse_negative_evidence(&negative_evidence)?.secret;
    let expected = require_secret_expected(&entry)?;
    assert_eq!(
        expected.fixture_nonce.as_ref().map(|nonce| nonce.as_str()),
        Some("secret_fixture_must_be_read")
    );
    assert_no_raw_value(&negative_evidence, "sk_test_vb_nf2u_raw_secret_CHANGED");
    drop(fixture_guard);
    Ok(())
}

#[test]
fn overlap_false_pass_fixture_is_rejected() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_overlap_fixture(
        "fixture_id=intentional_overlap_fixture\nfirst_control_id=run_button\nsecond_control_id=stop_button\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=passed\nfixture_nonce=overlap_false_pass_detector\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_false_pass_diagnostic(&output, "intentional_overlap_fixture", FixtureGate::Layout)?;
    drop(fixture_guard);
    Ok(())
}

#[test]
fn secret_false_pass_fixture_is_rejected() -> Result<(), Box<dyn Error>> {
    // Given
    let fixture_guard = reset_negative_fixtures()?;
    prepare_intentional_secret_fixture(
        "fixture_id=intentional_secret_fixture\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=passed\nfixture_nonce=secret_false_pass_detector\n",
    )?;

    // When
    let output = run_ai_release_for_vb_nf2u()?;

    // Then
    assert_false_pass_diagnostic(
        &output,
        "intentional_secret_fixture",
        FixtureGate::Redaction,
    )?;
    drop(fixture_guard);
    Ok(())
}

fn run_ai_release_for_vb_nf2u() -> Result<Output, Box<dyn Error>> {
    Command::new("cargo")
        .args(["xtask", "ai-release", "--bead", BEAD_ID])
        .output()
        .map_err(Into::into)
}

fn assert_command_succeeded(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "expected `cargo xtask ai-release --bead vb-nf2u` to succeed and emit UI release evidence\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_false_pass_diagnostic(
    output: &Output,
    expected_fixture: &str,
    expected_gate: FixtureGate,
) -> Result<(), Box<dyn Error>> {
    assert_ne!(
        output.status.code(),
        Some(0),
        "expected `cargo xtask ai-release --bead vb-nf2u` to fail closed for false-pass negative fixture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_exact_false_pass_diagnostic(&combined, expected_fixture, expected_gate)?;
    Ok(())
}

fn read_required_artifact(path: &str) -> Result<String, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    assert_ne!(content, "", "required artifact was empty: {path}");
    Ok(content)
}

fn assert_ui_subgates_are_exact(ai_release: &str) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        subgate_names(&parse_ai_release(ai_release)?),
        canonical_subgates()
    );
    Ok(())
}

fn assert_snapshot_inventory_is_exact(snapshot_report: &str) -> Result<(), Box<dyn Error>> {
    let report = parse_snapshot_report(snapshot_report)?;
    assert_eq!(report.total_screens, 8);
    assert_eq!(report.passed_screens, 8);
    assert_eq!(report.failed_screens, 0);
    assert_eq!(snapshot_screen_names(&report), canonical_screens());
    Ok(())
}

fn assert_screen_has_required_checks(
    snapshot_report: &str,
    screen: &str,
) -> Result<(), Box<dyn Error>> {
    let report = parse_snapshot_report(snapshot_report)?;
    let checks = snapshot_checks_for(&report, screen);
    assert_eq!(checks, required_checks());
    Ok(())
}

fn assert_redaction_coverage_for_screen(
    ai_release: &str,
    screen: &str,
) -> Result<(), Box<dyn Error>> {
    let ai = parse_ai_release(ai_release)?;
    let classes = redaction_classes_for(&ai, screen);
    assert_eq!(classes, redaction_classes());
    Ok(())
}

fn snapshot_screen_names(report: &SnapshotReport) -> Vec<String> {
    report
        .screens
        .iter()
        .map(|screen| screen.screen_name.as_str().to_string())
        .collect()
}

fn snapshot_checks_for(report: &SnapshotReport, screen: &str) -> Vec<String> {
    report
        .screens
        .iter()
        .find_map(|entry| {
            (entry.screen_name.as_str() == screen).then(|| {
                entry
                    .checks
                    .iter()
                    .map(|check| check.as_str().to_string())
                    .collect()
            })
        })
        .map_or_else(Vec::new, |checks| checks)
}

fn redaction_classes_for(doc: &AiReleaseDoc, screen: &str) -> Vec<String> {
    doc.redaction
        .iter()
        .find_map(|entry| {
            (entry.screen_id.as_str() == screen).then(|| {
                entry
                    .classes
                    .iter()
                    .map(|class| class.as_str().to_string())
                    .collect()
            })
        })
        .map_or_else(Vec::new, |classes| classes)
}

fn subgate_names(doc: &AiReleaseDoc) -> Vec<String> {
    doc.subgates
        .iter()
        .map(|subgate| subgate.as_str().to_string())
        .collect()
}

fn assert_no_raw_denied_values_are_emitted(evidence_text: &str) {
    assert_no_raw_value(evidence_text, RAW_SENTINEL);
    assert_no_raw_value(evidence_text, RAW_API_KEY);
    assert_no_raw_value(evidence_text, RAW_TOKEN);
    assert_no_raw_value(evidence_text, RAW_PASSWORD);
    assert_no_raw_value(evidence_text, RAW_IDEMPOTENCY_KEY);
    assert_no_raw_value(evidence_text, RAW_TAINT);
}

fn assert_no_raw_value(evidence_text: &str, raw: &str) {
    assert_eq!(
        raw_value_count(evidence_text, raw),
        0,
        "raw value leaked: {raw}"
    );
}

fn raw_value_count(evidence_text: &str, raw: &str) -> usize {
    evidence_text.match_indices(raw).count()
}

fn assert_exact_false_pass_diagnostic(
    text: &str,
    expected_fixture: &str,
    expected_gate: FixtureGate,
) -> Result<(), Box<dyn Error>> {
    let diag = XtaskCommandDiagnostic::parse_output(text)?;
    assert_eq!(diag.error_code.as_str(), "false_pass_fixture_violation");
    assert_eq!(diag.fixture_id.as_str(), expected_fixture);
    assert_eq!(diag.expected_gate, expected_gate);
    assert_eq!(diag.actual_status, FixtureStatus::Passed);
    Ok(())
}

fn assert_fixture_evidence_disclaims_core_parity(
    ai_release: &str,
    snapshot_report: &str,
) -> Result<(), Box<dyn Error>> {
    assert_eq!(
        parse_ai_release(ai_release)?.fixture_backed,
        FixtureBackedState::FixtureBacked
    );
    assert_eq!(
        parse_ai_release(ai_release)?.core_runtime_parity_claim,
        CoreParityClaim::Unsupported
    );
    assert_eq!(
        parse_snapshot_report(snapshot_report)?.fixture_backed,
        FixtureBackedState::FixtureBacked
    );
    assert_eq!(
        parse_snapshot_report(snapshot_report)?.core_runtime_parity_claim,
        CoreParityClaim::Unsupported
    );
    Ok(())
}

type SnapshotReport = ParsedSnapshotDocument;
type AiReleaseDoc = ParsedAiReleaseDocument;
type NegativeEvidenceDoc = ParsedNegativeFixtureDocument;
type NegativeEntry = ParsedOverlapFixtureEvidence;

fn assert_default_overlap_entry(entry: &NegativeEntry) -> Result<(), Box<dyn Error>> {
    let expected = require_overlap_expected(entry)?;
    assert_eq!(expected.status, FixtureStatus::ExpectedFailed);
    assert_eq!(expected.diagnostic_code, DiagnosticCode::Layout);
    assert_eq!(expected.screen_id.as_str(), "execution_overview");
    assert_eq!(expected.control_id.as_str(), "run_button");
    assert_eq!(expected.second_control_id.as_str(), "stop_button");
    assert_eq!(expected.predicate.as_str(), "Overlap");
    assert_eq!(expected.overlap_area_px.as_u32(), 600);
    assert_default_overlap_bounds(expected);
    Ok(())
}

fn assert_default_overlap_bounds(entry: &ParsedOverlapExpectedFailure) {
    assert_eq!(
        entry.bounds.as_str(),
        "{ x: 10, y: 10, width: 100, height: 60 }"
    );
}

fn assert_changed_overlap_entry(entry: &NegativeEntry) -> Result<(), Box<dyn Error>> {
    let expected = require_overlap_expected(entry)?;
    assert_eq!(
        expected.fixture_nonce.as_ref().map(|nonce| nonce.as_str()),
        Some("overlap_fixture_must_be_read")
    );
    assert_eq!(expected.overlap_area_px.as_u32(), 25);
    assert_eq!(expected.control_id.as_str(), "changed_run_button");
    assert_eq!(expected.second_control_id.as_str(), "changed_stop_button");
    assert_changed_overlap_bounds(expected);
    Ok(())
}

fn assert_changed_overlap_bounds(entry: &ParsedOverlapExpectedFailure) {
    assert_eq!(
        entry.bounds.as_str(),
        "{ x: 1, y: 1, width: 10, height: 10 }"
    );
}

fn require_overlap_expected(
    entry: &ParsedOverlapFixtureEvidence,
) -> Result<&ParsedOverlapExpectedFailure, Box<dyn Error>> {
    match entry {
        ParsedOverlapFixtureEvidence::ExpectedFailed(value) => Ok(value),
        ParsedOverlapFixtureEvidence::Rejected(_) => Err("overlap fixture was rejected".into()),
    }
}

fn require_secret_expected(
    entry: &ParsedSecretFixtureEvidence,
) -> Result<&ParsedSecretExpectedFailure, Box<dyn Error>> {
    match entry {
        ParsedSecretFixtureEvidence::ExpectedFailed(value) => Ok(value),
        ParsedSecretFixtureEvidence::Rejected(_) => Err("secret fixture was rejected".into()),
    }
}

fn parse_snapshot_report(text: &str) -> Result<SnapshotReport, Box<dyn Error>> {
    parse_snapshot_document(text).map_err(Into::into)
}

fn parse_ai_release(text: &str) -> Result<AiReleaseDoc, Box<dyn Error>> {
    parse_ai_release_document(text).map_err(Into::into)
}

fn parse_negative_evidence(text: &str) -> Result<NegativeEvidenceDoc, Box<dyn Error>> {
    parse_negative_fixture_document(text).map_err(Into::into)
}

fn canonical_screens() -> Vec<String> {
    [
        "execution_overview",
        "workflow_graph_authoring",
        "execution_details",
        "verification_certificate",
        "replay_theater",
        "incident_failure",
        "action_registry",
        "storage_doctor_ai_context",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn canonical_subgates() -> Vec<String> {
    [
        "ui_snapshot",
        "layout_readability",
        "redaction",
        "negative_fixture",
        "deterministic_capture",
        "evidence_shape",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn required_checks() -> Vec<String> {
    [
        "Overlap",
        "Clipping",
        "Bounds",
        "ChipReadability",
        "SelectedState",
        "FixtureArtifactProvenance",
        "Redaction",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn redaction_classes() -> Vec<String> {
    [
        "sentinel",
        "api_key",
        "token",
        "password",
        "idempotency_key",
        "tainted_fixture_value",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn prepare_intentional_overlap_fixture(content: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(NEGATIVE_FIXTURE_DIR).join("intentional_overlap_fixture.txt");
    write_fixture(path, content)
}

fn prepare_intentional_secret_fixture(content: &str) -> Result<(), Box<dyn Error>> {
    let path = Path::new(NEGATIVE_FIXTURE_DIR).join("intentional_secret_fixture.txt");
    write_fixture(path, content)
}

fn write_fixture(path: PathBuf, content: &str) -> Result<(), Box<dyn Error>> {
    let parent = path.parent().ok_or("fixture path must have parent")?;
    fs::create_dir_all(parent)?;
    fs::write(path, content)?;
    Ok(())
}

fn reset_negative_fixtures() -> Result<MutexGuard<'static, ()>, Box<dyn Error>> {
    let guard = match fixture_mutex().lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    match fs::remove_dir_all(NEGATIVE_FIXTURE_DIR) {
        Ok(()) => {
            write_default_negative_fixtures()?;
            Ok(guard)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            write_default_negative_fixtures()?;
            Ok(guard)
        }
        Err(error) => Err(Box::new(error)),
    }
}

fn write_default_negative_fixtures() -> Result<(), Box<dyn Error>> {
    prepare_intentional_overlap_fixture(
        "fixture_id=intentional_overlap_fixture\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=failed\n",
    )?;
    prepare_intentional_secret_fixture(
        "fixture_id=intentional_secret_fixture\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\n",
    )?;
    Ok(())
}

fn fixture_mutex() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}
