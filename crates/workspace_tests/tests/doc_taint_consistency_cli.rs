//! CLI tests for the doc taint consistency checker (vb-eirmk.74.3.5.1).
//!
//! These tests invoke the `check-doc-taint-consistency` binary with temporary
//! documents and verify exit codes, stdout, and stderr match the Python
//! reference behavior exactly. The Python reference at
//! scripts/check-doc-taint-consistency.py (rev edee830ee26967d92a87123cd72f737c960b4298)
//! is the authoritative spec.
//!
//! Test categories:
//! - Arity and argument validation (exact output)
//! - File existence, directory, and UTF-8 validation (exact output)
//! - Stale node wording detection (all node types and phrases)
//! - Writer contradiction detection (all node types)
//! - Vocabulary conflict detection
//! - Control-flow conflation detection
//! - Finish signal wording detection (including comma exemption)
//! - Finish rejection contradiction detection
//! - Missing required finish wording
//! - Finding ordering (nodes → vocabulary → finish)
//! - Python splitlines boundary compatibility (all separators)
//! - Multi-finding aggregation and duplicates

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::Builder;

const BINARY: &str = env!("CARGO_BIN_EXE_check-doc-taint-consistency");

fn fixture_dir() -> tempfile::TempDir {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/doc-taint-tests");
    fs::create_dir_all(&root).expect("create fixture root");
    Builder::new()
        .prefix("vb-taint-test-")
        .tempdir_in(&root)
        .expect("temp dir")
}

/// Holds a temporary document and its path; keeps TempDir alive while tests run.
struct TestDoc {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

impl TestDoc {
    fn new(content: &str) -> Self {
        let dir = fixture_dir();
        let path = dir.path().join("velvet-ballistics-MASTER.md");
        fs::write(&path, content).expect("write doc");
        Self { _dir: dir, path }
    }

    fn new_non_utf8() -> Self {
        let dir = fixture_dir();
        let path = dir.path().join("velvet-ballistics-MASTER.md");
        let mut f = fs::File::create(&path).expect("create doc");
        f.write_all(&[0x80, 0x81, 0x82]).expect("write bytes");
        Self { _dir: dir, path }
    }

    fn path_str(&self) -> &str {
        self.path.to_str().expect("path is utf-8")
    }
}

fn run_checker(args: &[&str]) -> std::process::Output {
    Command::new(BINARY)
        .args(args)
        .output()
        .expect("run checker")
}

fn assert_success(content: &str) {
    let doc = TestDoc::new(content);
    let result = run_checker(&[doc.path_str()]);
    assert_eq!(result.status.code(), Some(0), "expected exit 0");
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(
        stdout, "doc taint consistency: PASS\n",
        "unexpected stdout: {:?}",
        stdout
    );
    assert_eq!(stderr, "", "unexpected stderr: {:?}", stderr);
}

fn assert_failure(content: &str, expected_reasons: &[&str]) {
    let doc = TestDoc::new(content);
    let result = run_checker(&[doc.path_str()]);
    assert_eq!(result.status.code(), Some(1), "expected exit 1");
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "", "expected empty stdout, got: {:?}", stdout);
    let expected = format!(
        "doc taint consistency: FAIL\n{}",
        expected_reasons
            .iter()
            .map(|r| format!("- {}\n", r))
            .collect::<String>()
    );
    assert_eq!(
        stderr, expected,
        "stderr mismatch.\nexpected:\n{:?}\ngot:\n{:?}",
        expected, stderr
    );
}

// === Arity validation ===

#[test]
fn missing_argument_prints_usage_exit2() {
    let result = run_checker(&[]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "usage: check-doc-taint-consistency velvet-ballistics-MASTER.md\n"
    );
}

#[test]
fn too_many_arguments_prints_usage_exit2() {
    let doc = TestDoc::new("clean");
    let result = run_checker(&[doc.path_str(), "extra"]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(
        stderr,
        "usage: check-doc-taint-consistency velvet-ballistics-MASTER.md\n"
    );
}

// === File validation ===

#[test]
fn wrong_filename_fails_exit2() {
    let dir = fixture_dir();
    let path = dir.path().join("wrong-name.md");
    fs::write(&path, "clean doc").expect("write");
    let path_str = path.to_str().expect("path");
    let expected = format!(
        "error: expected velvet-ballistics-MASTER.md, got {}\n",
        path_str
    );
    let result = run_checker(&[path_str]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(stderr, expected);
}

#[test]
fn nonexistent_file_fails_exit2() {
    let dir = fixture_dir();
    let path = dir.path().join("velvet-ballistics-MASTER.md");
    let path_str = path.to_str().expect("path");
    let expected = format!("error: document not found: {}\n", path_str);
    let result = run_checker(&[path_str]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(stderr, expected);
}

#[test]
fn directory_named_master_fails_exit2() {
    let dir = fixture_dir();
    let master_dir = dir.path().join("velvet-ballistics-MASTER.md");
    fs::create_dir(&master_dir).expect("create dir");
    let path_str = master_dir.to_str().expect("path");
    let expected = format!("error: document not found: {}\n", path_str);
    let result = run_checker(&[path_str]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(stderr, expected);
}

#[cfg(target_os = "linux")]
#[test]
fn unreadable_file_fails_exit2() {
    let dir = fixture_dir();
    let path = dir.path().join("velvet-ballistics-MASTER.md");
    std::os::unix::fs::symlink("/proc/self/mem", &path).expect("create unreadable file link");
    assert!(path.is_file(), "linked target must pass file validation");
    let detail = fs::read(&path).expect_err("linked target must fail to read");
    let path_str = path.to_str().expect("path");
    let expected = format!("error: cannot read {}: {}\n", path_str, detail);
    let result = run_checker(&[path_str]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(stderr, expected);
}

#[test]
fn non_utf8_file_fails_exit2() {
    let doc = TestDoc::new_non_utf8();
    let path_str = doc.path_str();
    let expected = format!(
        "error: invalid utf-8 in {}: invalid utf-8 sequence of 1 bytes from index 0\n",
        path_str
    );
    let result = run_checker(&[path_str]);
    assert_eq!(result.status.code(), Some(2));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert_eq!(stdout, "");
    assert_eq!(stderr, expected);
}

// === Positive controls: clean documents pass ===

#[test]
fn clean_document_passes() {
    assert_success(
        "EvalExpr joins operand taints.\nBuildObject joins field taints.\nBuildList joins item taints.\nFinished(SlotValue, Taint) is emitted.",
    );
}

#[test]
fn document_with_required_finish_passes() {
    assert_success("Finished(SlotValue, Taint) is the signal.");
}

#[test]
fn document_with_only_node_names_passes() {
    assert_success("EvalExpr\nBuildObject\nBuildList\nFinished(SlotValue, Taint) is emitted.");
}

#[test]
fn lowercase_node_names_do_not_match_forbidden_phrases() {
    for content in [
        "evalexpr Always Clean.\nFinished(SlotValue, Taint) is emitted.",
        "buildobject Always Clean.\nFinished(SlotValue, Taint) is emitted.",
        "buildlist Always Clean.\nFinished(SlotValue, Taint) is emitted.",
    ] {
        assert_success(content);
    }
}

#[test]
fn case_variants_of_node_phrases_pass() {
    for content in [
        "EvalExpr Always clean.\nFinished(SlotValue, Taint) is emitted.",
        "EvalExpr always clean.\nFinished(SlotValue, Taint) is emitted.",
        "EvalExpr No Taint join.\nFinished(SlotValue, Taint) is emitted.",
        "EvalExpr no taint JOIN of expression operands.\nFinished(SlotValue, Taint) is emitted.",
        "BuildObject Always clean.\nFinished(SlotValue, Taint) is emitted.",
        "BuildObject always clean.\nFinished(SlotValue, Taint) is emitted.",
        "BuildObject no JOIN of field taints.\nFinished(SlotValue, Taint) is emitted.",
        "BuildList Always clean.\nFinished(SlotValue, Taint) is emitted.",
        "BuildList always clean.\nFinished(SlotValue, Taint) is emitted.",
        "BuildList no JOIN of item taints.\nFinished(SlotValue, Taint) is emitted.",
    ] {
        assert_success(content);
    }
}

#[test]
fn case_variants_of_vocabulary_literals_pass() {
    for content in [
        "clean < Secret < DerivedFromSecret.\nFinished(SlotValue, Taint) is emitted.",
        "secret downgrades to Clean.\nFinished(SlotValue, Taint) is emitted.",
        "private taint level.\nFinished(SlotValue, Taint) is emitted.",
    ] {
        assert_success(content);
    }
}

#[test]
fn empty_document_fails_missing_finish() {
    assert_failure("", &["missing finish signal taint wording"]);
}

// === Stale node wording detection ===

#[test]
fn eval_expr_always_clean_detected() {
    assert_failure(
        "EvalExpr Always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn eval_expr_always_clean_lowercase_detected() {
    assert_failure(
        "EvalExpr always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: always Clean"],
    );
}

#[test]
fn eval_expr_no_taint_join_detected() {
    assert_failure(
        "EvalExpr No taint join of expression operands.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: No taint join"],
    );
}

#[test]
fn eval_expr_no_taint_join_lowercase_detected() {
    assert_failure(
        "EvalExpr no taint join of expression operands.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: no taint join of expression operands"],
    );
}

#[test]
fn build_object_always_clean_detected() {
    assert_failure(
        "BuildObject Always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildObject stale taint wording: Always Clean"],
    );
}

#[test]
fn build_object_always_clean_lowercase_detected() {
    assert_failure(
        "BuildObject always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildObject stale taint wording: always Clean"],
    );
}

#[test]
fn build_object_no_join_field_taints_detected() {
    assert_failure(
        "BuildObject does not perform no join of field taints.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildObject stale taint wording: no join of field taints"],
    );
}

#[test]
fn build_list_always_clean_detected() {
    assert_failure(
        "BuildList Always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildList stale taint wording: Always Clean"],
    );
}

#[test]
fn build_list_always_clean_lowercase_detected() {
    assert_failure(
        "BuildList always Clean output.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildList stale taint wording: always Clean"],
    );
}

#[test]
fn build_list_no_join_item_taints_detected() {
    assert_failure(
        "BuildList no join of item taints.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildList stale taint wording: no join of item taints"],
    );
}

// === Writer contradiction detection (all 3 node types) ===

#[test]
fn eval_expr_write_slot_without_write_slot_with_taint_detected() {
    assert_failure(
        "EvalExpr uses write_slot but not write_slot_with_taint.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: write_slot"],
    );
}

#[test]
fn build_object_write_slot_without_write_slot_with_taint_detected() {
    assert_failure(
        "BuildObject uses write_slot but not write_slot_with_taint.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildObject stale taint wording: write_slot"],
    );
}

#[test]
fn build_list_write_slot_without_write_slot_with_taint_detected() {
    assert_failure(
        "BuildList uses write_slot but not write_slot_with_taint.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: BuildList stale taint wording: write_slot"],
    );
}

#[test]
fn eval_expr_write_slot_with_taint_ok() {
    assert_success("EvalExpr uses write_slot_with_taint.\nFinished(SlotValue, Taint) is emitted.");
}

// === Multiple findings on same line ===

#[test]
fn multiple_stale_phrases_on_same_line_detected() {
    assert_failure(
        "EvalExpr Always Clean No taint join.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: No taint join",
        ],
    );
}

// === Vocabulary conflict detection ===

#[test]
fn wrong_lattice_order_detected() {
    assert_failure(
        "Clean < Secret < DerivedFromSecret\nFinished(SlotValue, Taint) is emitted.",
        &["wrong lattice order"],
    );
}

#[test]
fn secret_downgrade_claim_detected() {
    assert_failure(
        "Secret downgrades to Clean when copied.\nFinished(SlotValue, Taint) is emitted.",
        &["secret downgrade claim"],
    );
}

#[test]
fn private_term_detected() {
    assert_failure(
        "The Private taint level.\nFinished(SlotValue, Taint) is emitted.",
        &["unknown taint term"],
    );
}

// === Control-flow conflation detection ===

#[test]
fn tracks_secret_branch_condition_taint_detected() {
    assert_failure(
        "The engine tracks secret branch-condition taint.\nFinished(SlotValue, Taint) is emitted.",
        &["control-flow taint conflation: tracks secret branch-condition taint"],
    );
}

#[test]
fn tracks_branch_condition_taint_detected() {
    assert_failure(
        "The engine tracks branch-condition taint.\nFinished(SlotValue, Taint) is emitted.",
        &["control-flow taint conflation: tracks branch-condition taint"],
    );
}

#[test]
fn control_flow_detection_is_case_insensitive() {
    assert_failure(
        "The engine TRACKS SECRET BRANCH-CONDITION TAINT.\nFinished(SlotValue, Taint) is emitted.",
        &["control-flow taint conflation: tracks secret branch-condition taint"],
    );
}

// === Finish signal wording detection ===

#[test]
fn stale_finish_signal_detected() {
    assert_failure(
        "Emit Finished(SlotValue) when done.",
        &[
            "stale finish signal missing taint",
            "missing finish signal taint wording",
        ],
    );
}

#[test]
fn correct_finish_signal_passes() {
    assert_success("Emit Finished(SlotValue, Taint) when done.");
}

#[test]
fn finish_signal_with_comma_passes() {
    assert_success("Emit Finished(SlotValue, Taint) when done.");
}

// === Stale Finished(SlotValue) immediate comma exemption ===

#[test]
fn stale_finished_slotvalue_followed_by_comma_is_exempt() {
    // "Finished(SlotValue)," is not stale - it's a list item with the old name
    assert_success("Signals: Finished(SlotValue), Finished(SlotValue, Taint), and other signals.");
}

#[test]
fn exempt_match_followed_by_genuine_stale_match() {
    // One exempt (with comma) and one genuine stale (without comma)
    assert_failure(
        "Old list: Finished(SlotValue), Finished(SlotValue, Taint).\nNew doc: Emit Finished(SlotValue) when done.",
        &["stale finish signal missing taint"],
    );
}

// === Finish rejection contradiction detection ===

#[test]
fn finish_rejection_contradiction_detected() {
    assert_failure(
        "On finish with Secret, emit rejection.\nFinished(SlotValue, Taint) is emitted.",
        &["finish rejection contradiction"],
    );
}

#[test]
fn finish_rejection_contradiction_derived_detected() {
    assert_failure(
        "On finish with DerivedFromSecret, emit rejection.\nFinished(SlotValue, Taint) is emitted.",
        &["finish rejection contradiction"],
    );
}

#[test]
fn finish_no_rejection_allowed() {
    assert_success(
        "On finish with Secret, emit no rejection.\nFinished(SlotValue, Taint) is emitted.",
    );
}

#[test]
fn finish_does_not_reject_allowed() {
    assert_success(
        "On finish with Secret, does not reject.\nFinished(SlotValue, Taint) is emitted.",
    );
}

#[test]
fn finish_not_reject_allowed() {
    assert_success("On finish with Secret, not reject.\nFinished(SlotValue, Taint) is emitted.");
}

// === Missing required finish wording ===

#[test]
fn missing_finished_slotvalue_taint_detected() {
    assert_failure(
        "EvalExpr joins operand taints.\nBuildObject joins field taints.",
        &["missing finish signal taint wording"],
    );
}

// === Finding order tests ===

#[test]
fn findings_ordered_nodes_before_vocabulary_before_finish() {
    let doc = "EvalExpr Always Clean.\nClean < Secret < DerivedFromSecret\nFinished(SlotValue) when done.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
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
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
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
    assert_failure(
        "EvalExpr Always Clean.\nBuildObject always Clean.\nPrivate taint.\nFinished(SlotValue) when done.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 2: BuildObject stale taint wording: always Clean",
            "unknown taint term",
            "stale finish signal missing taint",
            "missing finish signal taint wording",
        ],
    );
}

// === Line number accuracy ===

#[test]
fn line_numbers_are_accurate() {
    let doc = "First line clean.\nSecond line EvalExpr Always Clean.\nThird line clean.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
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
    let doc = "EvalExpr always Clean. BuildObject Always Clean. BuildList no join of item taints.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");

    let eval_pos = stderr.find("EvalExpr stale").expect("eval finding");
    let build_obj_pos = stderr
        .find("BuildObject stale")
        .expect("build object finding");
    let build_list_pos = stderr.find("BuildList stale").expect("build list finding");

    assert!(
        eval_pos < build_obj_pos && build_obj_pos < build_list_pos,
        "Findings must be in node order: EvalExpr, BuildObject, BuildList"
    );
}

// === Phrase order within node ===

#[test]
fn phrase_order_within_node_matches_declaration() {
    let doc = "EvalExpr no taint join of expression operands Always Clean.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");

    let always_pos = stderr.find("Always Clean").expect("always clean finding");
    let no_join_pos = stderr
        .find("no taint join of expression operands")
        .expect("no join finding");

    assert!(
        always_pos < no_join_pos,
        "Phrase order must match declaration: Always Clean before no taint join"
    );
}

// === Python splitlines compatibility: all separators ===

#[test]
fn lf_line_endings_handled() {
    assert_failure(
        "Line 1.\nLine 2 EvalExpr Always Clean.\nLine 3.\nFinished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn crlf_line_endings_handled() {
    assert_failure(
        "Line 1.\r\nLine 2 EvalExpr Always Clean.\r\nLine 3.\r\nFinished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn cr_line_endings_handled() {
    assert_failure(
        "Line 1.\rLine 2 EvalExpr Always Clean.\rLine 3.\rFinished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn form_feed_line_endings_handled() {
    assert_failure(
        "Line 1.\u{0c}Line 2 EvalExpr Always Clean.\u{0c}Line 3.\u{0c}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn vertical_tab_line_endings_handled() {
    assert_failure(
        "Line 1.\u{0b}Line 2 EvalExpr Always Clean.\u{0b}Line 3.\u{0b}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn information_separator_1_line_endings_handled() {
    assert_failure(
        "Line 1.\u{1c}Line 2 EvalExpr Always Clean.\u{1c}Line 3.\u{1c}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn information_separator_2_line_endings_handled() {
    assert_failure(
        "Line 1.\u{1d}Line 2 EvalExpr Always Clean.\u{1d}Line 3.\u{1d}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn information_separator_3_line_endings_handled() {
    assert_failure(
        "Line 1.\u{1e}Line 2 EvalExpr Always Clean.\u{1e}Line 3.\u{1e}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn next_line_line_endings_handled() {
    assert_failure(
        "Line 1.\u{85}Line 2 EvalExpr Always Clean.\u{85}Line 3.\u{85}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn line_separator_line_endings_handled() {
    assert_failure(
        "Line 1.\u{2028}Line 2 EvalExpr Always Clean.\u{2028}Line 3.\u{2028}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn paragraph_separator_line_endings_handled() {
    assert_failure(
        "Line 1.\u{2029}Line 2 EvalExpr Always Clean.\u{2029}Line 3.\u{2029}Finished(SlotValue, Taint) is emitted.",
        &["line 2: EvalExpr stale taint wording: Always Clean"],
    );
}

#[test]
fn u001f_is_not_a_splitlines_separator() {
    // U+001F (Unit Separator) is NOT in Python's splitlines set
    let doc = "Line 1.\u{1f}Line 2 EvalExpr Always Clean.\u{1f}Line 3.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    // All three "lines" are one splitlines result, so finding is on line 1
    assert!(
        stderr.contains("line 1: EvalExpr stale taint wording: Always Clean"),
        "U+001F is not a splitlines separator; finding should be on line 1. Got:\n{}",
        stderr
    );
}

// === CRLF counts as one line ending ===

#[test]
fn crlf_counts_as_one_line_ending() {
    let doc = "Line 1.\r\nLine 2.\r\nLine 3.\r\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    assert_eq!(result.status.code(), Some(0));
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    assert_eq!(stdout, "doc taint consistency: PASS\n");
}

// === Consecutive separators and trailing separator ===

#[test]
fn consecutive_separators_produce_empty_lines() {
    let doc = "Line 1.\n\nLine 3.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    assert_eq!(result.status.code(), Some(0));
}

#[test]
fn trailing_separator_adds_no_spurious_line() {
    let doc = "Line 1.\nLine 2.\n";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    // Should fail with missing finish wording, not extra line issues
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.contains("missing finish signal taint wording"),
        "Expected missing finish wording error. Got:\n{}",
        stderr
    );
}

// === Line 1 detection ===

#[test]
fn finding_on_line_one() {
    assert_failure(
        "EvalExpr Always Clean.\nFinished(SlotValue, Taint) is emitted.",
        &["line 1: EvalExpr stale taint wording: Always Clean"],
    );
}

// === Duplicates: node findings vs vocabulary ===

#[test]
fn node_findings_preserve_duplicates() {
    // Multiple node findings on different lines are all reported
    assert_failure(
        "EvalExpr Always Clean.\nEvalExpr no taint join of expression operands.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 2: EvalExpr stale taint wording: no taint join of expression operands",
        ],
    );
}

#[test]
fn vocabulary_findings_collected_once() {
    // Vocabulary conflicts are checked against the whole document, not per line
    let doc = "Clean < Secret < DerivedFromSecret and again Clean < Secret < DerivedFromSecret.\nFinished(SlotValue, Taint) is emitted.";
    let path = TestDoc::new(doc);
    let result = run_checker(&[path.path_str()]);
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    let count = stderr
        .lines()
        .filter(|l| l.contains("wrong lattice order"))
        .count();
    assert_eq!(
        count, 1,
        "Vocabulary findings collected once per document, not per occurrence. Got {}.",
        count
    );
}

// === Realistic document tests ===

#[test]
fn realistic_document_with_mixed_issues() {
    assert_success(
        "Velvet Ballistics Taint Model\n\nData-flow taint joins occur in EvalExpr.\nBuildObject joins field taints.\nBuildList joins item taints.\n\nOn finish, emit Finished(SlotValue, Taint).",
    );
}

#[test]
fn realistic_document_with_stale_wording() {
    assert_failure(
        "Velvet Ballistics Taint Model\n\nEvalExpr produces Always Clean output.\nBuildObject does no join of field taints.\n\nOn finish, emit Finished(SlotValue). Finished(SlotValue, Taint) is emitted.",
        &[
            "line 3: EvalExpr stale taint wording: Always Clean",
            "line 4: BuildObject stale taint wording: no join of field taints",
            "stale finish signal missing taint",
        ],
    );
}

// === Critical regression: all nodes on one line with correct per-node ordering ===
// Original Python iterates nodes in order (EvalExpr, BuildObject, BuildList) and
// for each node checks all phrases THEN the writer contradiction before moving
// to the next node. This means a line containing all three node names produces
// findings in the order: EvalExpr's phrase findings, EvalExpr's write_slot finding,
// BuildObject's phrase findings, BuildObject's write_slot finding, etc.
// This distinguishes correct per-node interleaving from a buggy grouping of all
// phrase findings first, then all writer findings.

#[test]
fn all_nodes_on_one_line_per_node_ordering() {
    assert_failure(
        "EvalExpr Always Clean write_slot not write_slot_with_taint BuildObject Always Clean write_slot not write_slot_with_taint BuildList Always Clean write_slot not write_slot_with_taint.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: write_slot",
            "line 1: BuildObject stale taint wording: Always Clean",
            "line 1: BuildObject stale taint wording: write_slot",
            "line 1: BuildList stale taint wording: Always Clean",
            "line 1: BuildList stale taint wording: write_slot",
        ],
    );
}

#[test]
fn all_nodes_on_one_line_healthy_baseline() {
    assert_success(
        "EvalExpr joins operand taints. BuildObject joins field taints. BuildList joins item taints.\nFinished(SlotValue, Taint) is emitted.",
    );
}

// === Non-UTF8 OS argument (Unix) ===

#[test]
fn non_utf8_os_argument_no_panic_exit2() {
    // Pass a filename that cannot be converted to UTF-8. Must not panic; must
    // exit 2 with operational error. Use OsString/args_os to bypass UTF-8 checks.
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let bytes: Vec<u8> = vec![
        116, 101, 115, 116, 47, // "test/"
        255, 255, 255, 255, // invalid UTF-8 bytes
        46, 109, 100, // ".md"
    ];
    let arg = OsString::from_vec(bytes);

    let mut cmd = Command::new(BINARY);
    cmd.arg(arg);
    let result = cmd.output().expect("run checker");
    assert_eq!(
        result.status.code(),
        Some(2),
        "non-UTF8 argument must exit 2"
    );
    let stdout = String::from_utf8(result.stdout).expect("stdout utf-8");
    assert_eq!(stdout, "");
    let stderr = String::from_utf8(result.stderr).expect("stderr utf-8");
    assert!(
        stderr.starts_with("error:"),
        "stderr must start with error: prefix, got: {:?}",
        stderr
    );
}

// === All literal rules coverage ===

#[test]
fn all_eval_expr_phrases_detected() {
    // Test all 4 EvalExpr phrases in separate tests via individual tests above
    // This test verifies the full phrase list is covered
    assert_failure(
        "EvalExpr Always Clean always Clean No taint join no taint join of expression operands.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: EvalExpr stale taint wording: Always Clean",
            "line 1: EvalExpr stale taint wording: always Clean",
            "line 1: EvalExpr stale taint wording: No taint join",
            "line 1: EvalExpr stale taint wording: no taint join of expression operands",
        ],
    );
}

#[test]
fn all_build_object_phrases_detected() {
    assert_failure(
        "BuildObject Always Clean always Clean no join of field taints.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: BuildObject stale taint wording: Always Clean",
            "line 1: BuildObject stale taint wording: always Clean",
            "line 1: BuildObject stale taint wording: no join of field taints",
        ],
    );
}

#[test]
fn all_build_list_phrases_detected() {
    assert_failure(
        "BuildList Always Clean always Clean no join of item taints.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "line 1: BuildList stale taint wording: Always Clean",
            "line 1: BuildList stale taint wording: always Clean",
            "line 1: BuildList stale taint wording: no join of item taints",
        ],
    );
}

// === Output write failure ===

#[test]
fn stdout_write_failure_to_dev_full_exit2() {
    // /dev/full always returns ENOSPC on write; writing PASS output must fail
    let doc = TestDoc::new("Finished(SlotValue, Taint) is emitted.");
    let mut cmd = Command::new(BINARY);
    cmd.arg(doc.path_str());
    let dev_full = fs::File::options()
        .write(true)
        .open("/dev/full")
        .expect("open writable /dev/full");
    cmd.stdout(dev_full);
    let result = cmd.output().expect("run checker");
    assert_eq!(result.status.code(), Some(2), "write failure must exit 2");
}

// === Multiple vocabulary conflicts ===

#[test]
fn multiple_vocabulary_conflicts_all_reported() {
    assert_failure(
        "Clean < Secret < DerivedFromSecret Secret downgrades to Clean Private\nFinished(SlotValue, Taint) is emitted.",
        &[
            "wrong lattice order",
            "secret downgrade claim",
            "unknown taint term",
        ],
    );
}

// === Multiple control-flow conflations ===

#[test]
fn multiple_control_flow_conflations_all_reported() {
    assert_failure(
        "The engine tracks secret branch-condition taint and tracks branch-condition taint.\nFinished(SlotValue, Taint) is emitted.",
        &[
            "control-flow taint conflation: tracks secret branch-condition taint",
            "control-flow taint conflation: tracks branch-condition taint",
        ],
    );
}
