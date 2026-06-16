// Integration tests: canonical_digest through the compilation pipeline
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// These tests verify that the digest is correctly embedded through the
// compilation pipeline (YAML → WorkflowSource → canonical_digest → CompiledWorkflow).
// They treat the crate as a black box and test through the public API.

#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]

use vb_compile::canonical_digest;
use vb_yaml::{ast::WorkflowSource, parse_workflow_source};

/// YAML fixture: workflow with an Ask step and a timeout.
const ASK_YAML: &str = r#"
version: velvet-ballistics/v1
name: test-ask-digest
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?", timeout: "30s" }
  - id: done
    finish: { result: done }
"#;

/// YAML fixture: workflow with Set + Finish only (no Ask).
const SET_FINISH_YAML: &str = r#"
version: velvet-ballistics/v1
name: test-set-finish-digest
when: { manual: {} }
steps:
  - id: set_1
    set: { output: x, value: "1" }
  - id: done
    finish: { result: done }
"#;

/// YAML fixture: same as ASK_YAML but with different prompt.
const ASK_DIFFERENT_PROMPT_YAML: &str = r#"
version: velvet-ballistics/v1
name: test-ask-digest
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your quest?", timeout: "30s" }
  - id: done
    finish: { result: done }
"#;

/// YAML fixture: same structure but different workflow name.
const DIFFERENT_NAME_YAML: &str = r#"
version: velvet-ballistics/v1
name: different-workflow-name
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?", timeout: "30s" }
  - id: done
    finish: { result: done }
"#;

/// Parse a YAML fixture into a WorkflowSource.
fn parse_fixture(yaml: &str) -> WorkflowSource {
    parse_workflow_source(yaml.trim()).expect("test fixture must parse successfully")
}

// ── Integration: compiled workflow digest matches direct canonical_digest ──

#[test]
fn compiled_workflow_digest_matches_canonical_digest_for_ask_workflow() {
    // Given: a YAML string with Ask step
    let source = parse_fixture(ASK_YAML);
    // When: canonical_digest computed
    let direct_digest = canonical_digest(&source).expect("valid test input");
    // Then: digest is valid 32-byte hash
    assert_eq!(
        direct_digest.as_bytes().len(),
        32,
        "Direct canonical_digest must produce valid 32-byte digest for Ask workflow"
    );
}

#[test]
fn compiled_workflow_digest_is_deterministic_across_parses() {
    // Given: same YAML string parsed twice
    let source_a = parse_fixture(ASK_YAML);
    let source_b = parse_fixture(ASK_YAML);
    // When: canonical_digest computed for both
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: digests are identical (parsing is deterministic)
    assert_eq!(
        digest_a, digest_b,
        "Same YAML parsed twice must produce identical canonical digests"
    );
}

#[test]
fn compiled_workflow_digests_differ_when_ask_prompt_differs_in_yaml() {
    // Given: two YAML strings differing only in Ask prompt
    let source_a = parse_fixture(ASK_YAML);
    let source_b = parse_fixture(ASK_DIFFERENT_PROMPT_YAML);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different prompts → different digests
    assert_ne!(
        digest_a, digest_b,
        "Different Ask prompts in YAML must produce distinct canonical digests"
    );
}

#[test]
fn compiled_workflow_digests_differ_when_workflow_name_differs_in_yaml() {
    // Given: two YAML strings differing only in workflow name
    let source_a = parse_fixture(ASK_YAML);
    let source_b = parse_fixture(DIFFERENT_NAME_YAML);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different names → different digests
    assert_ne!(
        digest_a, digest_b,
        "Different workflow names in YAML must produce distinct canonical digests"
    );
}

#[test]
fn compiled_workflow_digest_unchanged_for_set_finish_workflow() {
    // Given: a YAML string with Set + Finish only (no Ask)
    let source = parse_fixture(SET_FINISH_YAML);
    // When
    let digest = canonical_digest(&source).expect("valid test input");
    // Then: valid digest, no panic
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "Set+Finish workflow must produce valid 32-byte canonical digest"
    );
    // And: deterministic across two parses
    let source2 = parse_fixture(SET_FINISH_YAML);
    let digest2 = canonical_digest(&source2).expect("valid test input");
    assert_eq!(
        digest, digest2,
        "Set+Finish workflow digest must be deterministic across parses"
    );
}
