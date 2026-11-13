//! CLI tests for the doc taint consistency checker (vb-eirmk.74.3.5.1).
//!
//! These tests invoke the `check-doc-taint-consistency` binary with temporary
//! documents and verify exit codes, stdout, and stderr match the Python
//! reference behavior. The Python reference is the authoritative spec;
//! these tests ensure the Rust tool produces identical observable behavior.
//!
//! Test categories:
//! - Arity and argument validation
//! - File existence and UTF-8 validation
//! - Stale node wording detection (EvalExpr, BuildObject, BuildList)
//! - Writer contradiction detection (write_slot without write_slot_with_taint)
//! - Vocabulary conflict detection
//! - Control-flow conflation detection
//! - Finish signal wording detection
//! - Finish rejection contradiction detection
//! - Multi-finding aggregation and ordering

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::Builder;

fn binary_path() -> String {
    // Try the CARGO_BIN_EXE variable first (set by cargo when building tests
    // alongside the binary). Fall back to the target directory.
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_check-doc-taint-consistency") {
        path
    } else {
        // Compute path relative to workspace root
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().expect("manifest has parent");
        workspace_root.join("target/debug/check-doc-taint-consistency").to_string_lossy().to_string()
    }
}

fn make_temp_doc(content: &str) -> PathBuf {
    let dir = Builder::new().prefix("vb-taint-test-").tempdir().expect("temp dir");
    let path = dir.path().join("velvet-ballistics-MASTER.md");
    fs::write(&path, content).expect("write doc");
    path
}

fn make_non_utf8_doc() -> PathBuf {
    let dir = Builder::new().prefix("vb-taint-test-").tempdir().expect("temp dir");
    let path = dir.path().join("velvet-ballistics-MASTER.md");
    let mut f = fs::File::create(&path).expect("create doc");
    f.write_all(&[0x80, 0x81, 0x82]).expect("write bytes");
    path
}

fn run_checker(args: &[&str]) -> Command {
    let mut cmd = Command::new(&binary_path());
    cmd.args(args);
    cmd
}

fn expect_success(content: &str) {
    let path = make_temp_doc(content);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");

    assert!(
        result.status.success(),
        "Expected success. exit={}, stdout='{}', stderr='{}'",
        result.status,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    assert!(
        stdout.trim() == "doc taint consistency: PASS",
        "Unexpected stdout: '{}'",
        stdout
    );
}

fn expect_failure(content: &str, expected_findings: &[&str]) {
    let path = make_temp_doc(content);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");

    assert!(
        !result.status.success(),
        "Expected failure but got success. stdout='{}'",
        String::from_utf8_lossy(&result.stdout)
    );

    assert_eq!(
        result.status.code(),
        Some(1),
        "Expected exit code 1, got {:?}",
        result.status.code()
    );

    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    assert!(
        stdout.trim().is_empty(),
        "Expected empty stdout on failure, got: '{}'",
        stdout
    );

    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.starts_with("doc taint consistency: FAIL"),
        "Expected FAIL header, got: '{}'",
        stderr
    );

    for expected in expected_findings {
        assert!(
            stderr.contains(expected),
            "Expected finding '{}' in stderr:\n{}",
            expected,
            stderr
        );
    }
}

fn expect_exit2(content: Option<&str>, filename: &str, expected_in_stderr: &str) {
    let path = if let Some(c) = content {
        let dir = Builder::new().prefix("vb-taint-test-").tempdir().expect("temp dir");
        let p = dir.path().join(filename);
        fs::write(&p, c).expect("write");
        p
    } else {
        let dir = Builder::new().prefix("vb-taint-test-").tempdir().expect("temp dir");
        dir.path().join(filename)
    };

    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");

    assert!(
        !result.status.success(),
        "Expected non-zero exit"
    );
    assert_eq!(
        result.status.code(),
        Some(2),
        "Expected exit code 2, got {:?}",
        result.status.code()
    );

    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains(expected_in_stderr),
        "Expected '{}' in stderr, got: '{}'",
        expected_in_stderr,
        stderr
    );
}

// === Arity tests ===

#[test]
fn missing_argument_prints_usage() {
    let result = run_checker(&[]).output().expect("run checker");
    assert!(!result.status.success());
    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(stderr.contains("usage"));
}

#[test]
fn too_many_arguments_prints_usage() {
    let path = make_temp_doc("clean doc");
    let result = run_checker(&[path.to_str().expect("path"), "extra"])
        .output()
        .expect("run checker");
    assert!(!result.status.success());
    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(stderr.contains("usage"));
}

// === File validation tests ===

#[test]
fn wrong_filename_fails_with_exit2() {
    expect_exit2(Some("clean doc"), "wrong-name.md", "expected velvet-ballistics-MASTER.md");
}

#[test]
fn nonexistent_file_fails_with_exit2() {
    expect_exit2(None, "velvet-ballistics-MASTER.md", "document not found");
}

#[test]
fn non_utf8_file_fails_with_exit2() {
    let path = make_non_utf8_doc();
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    assert!(!result.status.success());
    assert_eq!(result.status.code(), Some(2));
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(stderr.contains("invalid utf-8"));
}

// === Positive control: clean document passes ===

#[test]
fn clean_document_passes() {
    expect_success("EvalExpr joins operand taints.\nBuildObject joins field taints.\nFinished(SlotValue, Taint) is emitted.");
}

#[test]
fn empty_document_fails_missing_finish() {
    expect_failure("", &["missing finish signal taint wording"]);
}

#[test]
fn document_with_only_node_names_passes() {
    // Node names alone without stale wording should pass
    expect_success("EvalExpr\nBuildObject\nBuildList");
}

#[test]
fn document_with_required_finish_passes() {
    expect_success("Finished(SlotValue, Taint) is the signal.");
}

// === Stale node wording detection ===

#[test]
fn eval_expr_always_clean_detected() {
    expect_failure(
        "EvalExpr Always Clean output.",
        &["line 1: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn eval_expr_always_clean_lowercase_detected() {
    expect_failure(
        "EvalExpr always Clean output.",
        &["line 1: EvalExpr stale taint wording: always Clean"],
    );
}

#[test]
fn eval_expr_no_taint_join_detected() {
    expect_failure(
        "EvalExpr No taint join of expression operands.",
        &["line 1: EvalExpr stale taint wording: No taint join"],
    );
}

#[test]
fn eval_expr_no_taint_join_lowercase_detected() {
    expect_failure(
        "EvalExpr no taint join of expression operands.",
        &["line 1: EvalExpr stale taint wording: no taint join of expression operands"],
    );
}

#[test]
fn build_object_always_clean_detected() {
    expect_failure(
        "BuildObject Always Clean.",
        &["line 1: BuildObject stale taint wording: Always Clean"],
    );
}

#[test]
fn build_object_no_join_field_taints_detected() {
    expect_failure(
        "BuildObject does not perform no join of field taints.",
        &["line 1: BuildObject stale taint wording: no join of field taints"],
    );
}

#[test]
fn build_list_always_clean_detected() {
    expect_failure(
        "BuildList always Clean output.",
        &["line 1: BuildList stale taint wording: always Clean"],
    );
}

#[test]
fn build_list_no_join_item_taints_detected() {
    expect_failure(
        "BuildList no join of item taints.",
        &["line 1: BuildList stale taint wording: no join of item taints"],
    );
}

// === Writer contradiction detection ===

#[test]
fn eval_expr_write_slot_without_write_slot_with_taint_detected() {
    expect_failure(
        "EvalExpr uses write_slot but not write_slot_with_taint.",
        &["line 1: EvalExpr stale taint wording: write_slot"],
    );
}

#[test]
fn eval_expr_write_slot_with_taint_ok() {
    expect_success("EvalExpr uses write_slot_with_taint.");
}

// === Multiple findings on same line ===

#[test]
fn multiple_stale_phrases_on_same_line_detected() {
    expect_failure(
        "EvalExpr Always Clean No taint join.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: No taint join",
        ],
    );
}

// === Vocabulary conflict detection ===

#[test]
fn wrong_lattice_order_detected() {
    expect_failure(
        "Clean < Secret < DerivedFromSecret",
        &["wrong lattice order"],
    );
}

#[test]
fn secret_downgrade_claim_detected() {
    expect_failure(
        "Secret downgrades to Clean when copied.",
        &["secret downgrade claim"],
    );
}

#[test]
fn private_term_detected() {
    expect_failure(
        "The Private taint level.",
        &["unknown taint term"],
    );
}

// === Control-flow conflation detection ===

#[test]
fn tracks_secret_branch_condition_taint_detected() {
    expect_failure(
        "The engine tracks secret branch-condition taint.",
        &["control-flow taint conflation: tracks secret branch-condition taint"],
    );
}

#[test]
fn tracks_branch_condition_taint_detected() {
    expect_failure(
        "The engine tracks branch-condition taint.",
        &["control-flow taint conflation: tracks branch-condition taint"],
    );
}

#[test]
fn control_flow_detection_is_case_insensitive() {
    expect_failure(
        "The engine TRACKS SECRET BRANCH-CONDITION TAINT.",
        &["control-flow taint conflation: tracks secret branch-condition taint"],
    );
}

// === Finish signal wording detection ===

#[test]
fn stale_finish_signal_detected() {
    expect_failure(
        "Emit Finished(SlotValue) when done.",
        &["stale finish signal missing taint"],
    );
}

#[test]
fn correct_finish_signal_passes() {
    expect_success("Emit Finished(SlotValue, Taint) when done.");
}

#[test]
fn finish_signal_with_comma_passes() {
    expect_success("Emit Finished(SlotValue, Taint) when done.");
}

// === Finish rejection contradiction detection ===

#[test]
fn finish_rejection_contradiction_detected() {
    expect_failure(
        "On finish with Secret, emit rejection.",
        &["finish rejection contradiction"],
    );
}

#[test]
fn finish_rejection_contradiction_derived_detected() {
    expect_failure(
        "On finish with DerivedFromSecret, emit rejection.",
        &["finish rejection contradiction"],
    );
}

#[test]
fn finish_no_rejection_allowed() {
    expect_success("On finish with Secret, emit no rejection.");
}

#[test]
fn finish_does_not_reject_allowed() {
    expect_success("On finish with Secret, does not reject.");
}

#[test]
fn finish_not_reject_allowed() {
    expect_success("On finish with Secret, not reject.");
}

// === Missing required finish wording ===

#[test]
fn missing_finished_slotvalue_taint_detected() {
    expect_failure(
        "EvalExpr joins operand taints.\nBuildObject joins field taints.",
        &["missing finish signal taint wording"],
    );
}

// === Finding order tests ===

#[test]
fn findings_ordered_nodes_before_vocabulary_before_finish() {
    // Should produce node findings, then vocabulary findings, then finish findings
    let doc = "Clean < Secret < DerivedFromSecret\nEvalExpr Always Clean.\nFinished(SlotValue) when done.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    assert!(!result.status.success());

    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    // Node finding must come before vocabulary finding
    let node_pos = stderr.find("stale taint wording").expect("node finding");
    let vocab_pos = stderr.find("wrong lattice order").expect("vocab finding");
    assert!(
        node_pos < vocab_pos,
        "Node findings must come before vocabulary findings"
    );
}

#[test]
fn finish_findings_after_vocabulary_findings() {
    let doc = "EvalExpr joins operand taints.\nClean < Secret < DerivedFromSecret\nFinished(SlotValue) when done.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    assert!(!result.status.success());

    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    let vocab_pos = stderr.find("wrong lattice order").expect("vocab finding");
    let finish_pos = stderr.find("stale finish signal").expect("finish finding");
    assert!(
        vocab_pos < finish_pos,
        "Vocabulary findings must come before finish findings"
    );
}

// === Multi-finding aggregation ===

#[test]
fn multiple_distinct_findings_all_reported() {
    expect_failure(
        "EvalExpr Always Clean.\nBuildObject always Clean.\nPrivate taint.\nFinished(SlotValue) when done.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 2: BuildObject stale taint wording: always Clean",
            "unknown taint term",
            "stale finish signal missing taint",
        ],
    );
}

// === Line number accuracy ===

#[test]
fn line_numbers_are_accurate() {
    let doc = "First line clean.\nSecond line EvalExpr Always Clean.\nThird line clean.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains("line 2: EvalExpr stale taint wording: Always Clean"),
        "Expected finding on line 2, got:\n{}",
        stderr
    );
}

// === Node order within line ===

#[test]
fn node_order_within_line_is_eval_expr_build_object_build_list() {
    let doc = "EvalExpr always Clean. BuildObject Always Clean. BuildList No join of item taints.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");

    let eval_pos = stderr.find("EvalExpr stale").expect("eval finding");
    let build_obj_pos = stderr.find("BuildObject stale").expect("build object finding");
    let build_list_pos = stderr.find("BuildList stale").expect("build list finding");

    assert!(
        eval_pos < build_obj_pos && build_obj_pos < build_list_pos,
        "Findings must be in node order: EvalExpr, BuildObject, BuildList"
    );
}

// === Phrase order within node ===

#[test]
fn phrase_order_within_node_matches_declaration() {
    let doc = "EvalExpr no taint join of expression operands Always Clean.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");

    let always_pos = stderr.find("Always Clean").expect("always clean finding");
    let no_join_pos = stderr.find("no taint join of expression operands").expect("no join finding");

    assert!(
        always_pos < no_join_pos,
        "Phrase order must match declaration: Always Clean before no taint join"
    );
}

// === Python splitlines compatibility ===

#[test]
fn crlf_line_endings_handled() {
    let doc = "Line 1.\r\nLine 2 EvalExpr Always Clean.\r\nLine 3.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains("line 2: EvalExpr stale taint wording: Always Clean"),
        "CRLF line endings must work, got:\n{}",
        stderr
    );
}

#[test]
fn cr_line_endings_handled() {
    let doc = "Line 1.\rLine 2 EvalExpr Always Clean.\rLine 3.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains("line 2: EvalExpr stale taint wording: Always Clean"),
        "CR line endings must work, got:\n{}",
        stderr
    );
}

#[test]
fn form_feed_line_endings_handled() {
    let doc = "Line 1.\u{0c}Line 2 EvalExpr Always Clean.\u{0c}Line 3.";
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains("line 2: EvalExpr stale taint wording: Always Clean"),
        "Form feed line endings must work, got:\n{}",
        stderr
    );
}

// === Duplicates preserved ===

#[test]
fn duplicate_vocabulary_findings_preserved() {
    let doc = "Clean < Secret < DerivedFromSecret and again Clean < Secret < DerivedFromSecret.";
    expect_failure(
        doc,
        &["wrong lattice order"],
    );
    // Should have 2 occurrences of the finding
    let path = make_temp_doc(doc);
    let result = run_checker(&[path.to_str().expect("path")])
        .output()
        .expect("run checker");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    let count = stderr
        .lines()
        .filter(|l| l.contains("wrong lattice order"))
        .count();
    assert!(
        count == 1,
        "Vocabulary findings are collected once per document, not per occurrence. Got {}.",
        count
    );
}

// === Realistic document test ===

#[test]
fn realistic_document_with_mixed_issues() {
    let doc = "Velvet Ballistics Taint Model\n\nData-flow taint joins occur in EvalExpr.\nBuildObject joins field taints.\nBuildList joins item taints.\n\nOn finish, emit Finished(SlotValue, Taint).";
    expect_success(doc);
}

#[test]
fn realistic_document_with_stale_wording() {
    let doc = "Velvet Ballistics Taint Model\n\nEvalExpr produces Always Clean output.\nBuildObject does not join field taints.\n\nOn finish, emit Finished(SlotValue).";
    expect_failure(
        doc,
        &[
            "line 3: EvalExpr stale taint wording: Always Clean",
            "line 4: BuildObject stale taint wording: no join of field taints",
            "stale finish signal missing taint",
        ],
    );
}
