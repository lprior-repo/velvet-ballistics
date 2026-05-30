// End-to-end tests: Full YAML → compile → verify semantic digest
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// These tests verify the end-to-end contract: YAML strings with Ask steps
// produce semantically distinct canonical digests. They test the actual
// user-facing behavior of the compiler.
//
// Note: empty prompt ("") is rejected by the YAML parser at the boundary
// level (FieldShape { expected: "non-empty string" }). This is a parser-level
// guard — non-empty prompts are enforced before digest computation.
// Empty-prompt digest behavior is tested via direct API construction in
// digest_ask_empty_prompt.rs and digest_ask_explicit_arm.rs.

#![forbid(unsafe_code)]

use vb_compile::canonical_digest;
use vb_yaml::parse_workflow_source;

// ── E2E: YAML with Ask + timeout → digest reflects timeout ──

#[test]
fn yaml_with_ask_and_timeout_produces_semantic_digest_changing_timeout() {
    // Given: YAML with Ask prompt + timeout "10s"
    let yaml_10s = r#"
version: velvet-ballistics/v1
name: e2e-timeout-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?", timeout: "10s" }
  - id: done
    finish: { result: done }
"#;
    // And: same YAML but with timeout "60s"
    let yaml_60s = r#"
version: velvet-ballistics/v1
name: e2e-timeout-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?", timeout: "60s" }
  - id: done
    finish: { result: done }
"#;
    let s10 = parse_workflow_source(yaml_10s.trim()).expect("parse");
    let s60 = parse_workflow_source(yaml_60s.trim()).expect("parse");
    let digest_10s = canonical_digest(&s10).expect("valid test input");
    let digest_60s = canonical_digest(&s60).expect("valid test input");
    assert_ne!(
        digest_10s, digest_60s,
        "E2E: different Ask timeouts must produce distinct canonical digests"
    );
}

#[test]
fn yaml_with_ask_and_timeout_none_vs_some_produces_distinct_digests() {
    let yaml_no_timeout = r#"
version: velvet-ballistics/v1
name: e2e-timeout-none-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?" }
  - id: done
    finish: { result: done }
"#;
    let yaml_with_timeout = r#"
version: velvet-ballistics/v1
name: e2e-timeout-none-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "What is your name?", timeout: "30s" }
  - id: done
    finish: { result: done }
"#;
    let s1 = parse_workflow_source(yaml_no_timeout.trim()).expect("parse");
    let s2 = parse_workflow_source(yaml_with_timeout.trim()).expect("parse");
    let digest_no_timeout = canonical_digest(&s1).expect("valid test input");
    let digest_with_timeout = canonical_digest(&s2).expect("valid test input");
    assert_ne!(
        digest_no_timeout, digest_with_timeout,
        "E2E: Ask with no timeout vs with timeout must produce distinct digests"
    );
}

#[test]
fn yaml_with_ask_single_char_prompt_produces_valid_digest() {
    let yaml = r#"
version: velvet-ballistics/v1
name: e2e-minimal-prompt-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "x", timeout: "30s" }
  - id: done
    finish: { result: done }
"#;
    let source = parse_workflow_source(yaml.trim()).expect("parse");
    let digest = canonical_digest(&source).expect("valid test input");
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "E2E: single-char prompt must produce valid 32-byte digest"
    );
}

#[test]
fn yaml_with_ask_prompt_changes_produce_distinct_digests() {
    let yaml_a = r#"
version: velvet-ballistics/v1
name: e2e-prompt-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "alpha" }
  - id: done
    finish: { result: done }
"#;
    let yaml_b = r#"
version: velvet-ballistics/v1
name: e2e-prompt-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "beta" }
  - id: done
    finish: { result: done }
"#;
    let s1 = parse_workflow_source(yaml_a.trim()).expect("parse");
    let s2 = parse_workflow_source(yaml_b.trim()).expect("parse");
    let digest_a = canonical_digest(&s1).expect("valid test input");
    let digest_b = canonical_digest(&s2).expect("valid test input");
    assert_ne!(
        digest_a, digest_b,
        "E2E: different Ask prompts must produce distinct digests"
    );
}

#[test]
fn yaml_with_set_finish_produces_deterministic_digest_unaffected_by_ask_fix() {
    let yaml = r#"
version: velvet-ballistics/v1
name: e2e-set-finish-test
when: { manual: {} }
steps:
  - id: set_1
    set: { output: result, value: "42" }
  - id: done
    finish: { result: done }
"#;
    let s1 = parse_workflow_source(yaml.trim()).expect("parse");
    let s2 = parse_workflow_source(yaml.trim()).expect("parse");
    let digest_a = canonical_digest(&s1).expect("valid test input");
    let digest_b = canonical_digest(&s2).expect("valid test input");
    assert_eq!(
        digest_a, digest_b,
        "E2E: Set+Finish workflow digest must be deterministic"
    );
    assert_eq!(
        digest_a.as_bytes().len(),
        32,
        "E2E: Set+Finish workflow must produce valid 32-byte digest"
    );
}

#[test]
fn yaml_with_multi_step_ask_workflow_produces_deterministic_digest() {
    let yaml = r#"
version: velvet-ballistics/v1
name: e2e-multi-step
when: { manual: {} }
steps:
  - id: set_name
    set: { output: name, value: "Alice" }
  - id: ask_continue
    ask: { prompt: "Continue?", timeout: "120s" }
  - id: done
    finish: { result: done }
"#;
    let s1 = parse_workflow_source(yaml.trim()).expect("parse");
    let s2 = parse_workflow_source(yaml.trim()).expect("parse");
    let digest_a = canonical_digest(&s1).expect("valid test input");
    let digest_b = canonical_digest(&s2).expect("valid test input");
    assert_eq!(
        digest_a, digest_b,
        "E2E: multi-step Ask workflow digest must be deterministic"
    );
}

#[test]
fn yaml_same_content_different_layout_produces_identical_digest() {
    let yaml_compact = r#"version: velvet-ballistics/v1
name: e2e-format-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "hello", timeout: "30s" }
  - id: done
    finish: { result: done }"#;

    let yaml_spacious = r#"

version: velvet-ballistics/v1
name: e2e-format-test
when: { manual: {} }
steps:
  - id: ask_1
    ask: { prompt: "hello", timeout: "30s" }
  - id: done
    finish: { result: done }

"#;
    let s1 = parse_workflow_source(yaml_compact.trim()).expect("parse");
    let s2 = parse_workflow_source(yaml_spacious.trim()).expect("parse");
    let digest_compact = canonical_digest(&s1).expect("valid test input");
    let digest_spacious = canonical_digest(&s2).expect("valid test input");
    assert_eq!(
        digest_compact, digest_spacious,
        "E2E: YAML formatting differences must not affect canonical digest"
    );
}
