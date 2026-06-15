#![forbid(unsafe_code)]
//! ForEach at_once gap tests (YAML → AST → CompiledWorkflow → digest).
//!
//! Bead: vb-xi2f.28 | State: 9 (test-writer)
//!
//! Gap: `foreach_digest_tests.rs` covers digest at the `vb_yaml::ast::StepPrimitive`
//! level only. This module tests the full compilation pipeline: YAML source text
//! → `vb_yaml::ast::WorkflowSource` → `CompiledWorkflow` → `ForEachStart.limit`
//! verification and digest determinism.
//!
//! BDD scenarios covered:
//!   B18: at_once=0 compiles → ForEachStart.limit == 0 (AC-FE-08)
//!   B19: at_once=None compiles → ForEachStart.limit == 1 (AC-FE-08)
//!   B20: at_once=Some(1) compiles → ForEachStart.limit == 1 (AC-FE-08)
//!   B21: at_once=0 and at_once=1 produce different workflow digests (AC-FE-07)
//!   B22: at_once=None and at_once=Some(1) produce identical digests (AC-FE-07)
//!   +    Determinism: same YAML compiled twice → same digest
//!   +    Multi-API: compile_workflow vs compile_source agree on limit

use vb_compile::{compile_source, compile_workflow};
use vb_core::{CompiledNodeKind, StepIdx, WorkflowDigest};
use vb_yaml::ast::WorkflowSource;

// ─────────────────────────────────────────────────────────────────────
// YAML templates
// ─────────────────────────────────────────────────────────────────────

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: foreach-atonce-test\nwhen:\n  manual: {}\nsteps:\n";

/// Build a full YAML document with a ForEach step using the given at_once value.
fn foreach_yaml(at_once_value: &str) -> String {
    format!(
        "{HEADER}  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      at_once: {at_once_value}\n      steps:\n        - id: body\n          set:\n            output: x\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    )
}

/// Build a full YAML document with a ForEach step that has NO at_once field.
fn foreach_yaml_no_atonce() -> String {
    format!(
        "{HEADER}  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: body\n          set:\n            output: x\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    )
}

/// Extract the limit from the ForEachStart node at index 0.
fn extract_foreach_start_limit(workflow: &vb_core::CompiledWorkflow) -> u32 {
    let parts = workflow.to_parts();
    match &parts.nodes[0].kind {
        CompiledNodeKind::ForEachStart { limit, .. } => *limit,
        other => panic!(
            "node 0 expected ForEachStart, got {:?} for workflow '{}'",
            other, parts.name
        ),
    }
}

/// Compute digest from compile_workflow.
fn digest_from_yaml(yaml: &str) -> WorkflowDigest {
    let workflow = compile_workflow(yaml.as_bytes())
        .unwrap_or_else(|e| panic!("compile_workflow failed: {e:?}"));
    workflow.to_parts().digest
}

/// Compute digest from compile_source (via YAML parse → compile_source).
fn digest_from_source(yaml: &str) -> WorkflowDigest {
    let source = vb_yaml::parse_workflow_source(yaml).expect("yaml must parse into WorkflowSource");
    let workflow =
        compile_source(&source).unwrap_or_else(|e| panic!("compile_source failed: {e:?}"));
    workflow.to_parts().digest
}

// ─────────────────────────────────────────────────────────────────────
// B18: at_once=0 compiles → ForEachStart.limit == 0
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_zero_compiles_and_limit_is_zero() {
    let yaml = foreach_yaml("0");
    let workflow = compile_workflow(yaml.as_bytes()).expect("YAML with at_once: 0 must compile");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(limit, 0, "ForEachStart.limit must be 0 when at_once is 0");
}

#[test]
fn foreach_at_once_zero_source_path_limit_is_zero() {
    let yaml = foreach_yaml("0");
    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow =
        compile_source(&source).expect("compile_source with at_once=Some(0) must succeed");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(
        limit, 0,
        "ForEachStart.limit must be 0 when at_once is Some(0) (source path)"
    );
}

#[test]
fn foreach_at_once_zero_node_kind_sequence() {
    let yaml = foreach_yaml("0");
    let workflow = compile_workflow(yaml.as_bytes()).expect("at_once=0 must compile");
    let parts = workflow.to_parts();

    // Expected sequence: ForEachStart, SetConst, ForEachNext, Finish
    assert_eq!(parts.nodes.len(), 4, "at_once=0 must produce 4 nodes");

    assert!(
        matches!(&parts.nodes[0].kind, CompiledNodeKind::ForEachStart { .. }),
        "node 0 must be ForEachStart"
    );
    assert!(
        matches!(&parts.nodes[1].kind, CompiledNodeKind::SetConst { .. }),
        "node 1 must be SetConst (body)"
    );
    assert!(
        matches!(&parts.nodes[2].kind, CompiledNodeKind::ForEachNext { .. }),
        "node 2 must be ForEachNext"
    );
    assert!(
        matches!(&parts.nodes[3].kind, CompiledNodeKind::Finish { .. }),
        "node 3 must be Finish"
    );
}

// ─────────────────────────────────────────────────────────────────────
// B19: at_once=None compiles → ForEachStart.limit == 1
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_none_compiles_and_limit_is_one() {
    let yaml = foreach_yaml_no_atonce();
    let workflow = compile_workflow(yaml.as_bytes()).expect("YAML without at_once must compile");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(
        limit, 1,
        "ForEachStart.limit must be 1 when at_once is omitted (defaults to 1)"
    );
}

#[test]
fn foreach_at_once_none_source_path_limit_is_one() {
    let yaml = foreach_yaml_no_atonce();
    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow = compile_source(&source).expect("compile_source with at_once=None must succeed");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(
        limit, 1,
        "ForEachStart.limit must be 1 when at_once is None (source path)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// B20: at_once=Some(1) compiles → ForEachStart.limit == 1
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_one_compiles_and_limit_is_one() {
    let yaml = foreach_yaml("1");
    let workflow = compile_workflow(yaml.as_bytes()).expect("YAML with at_once: 1 must compile");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(limit, 1, "ForEachStart.limit must be 1 when at_once is 1");
}

#[test]
fn foreach_at_once_one_source_path_limit_is_one() {
    let yaml = foreach_yaml("1");
    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow =
        compile_source(&source).expect("compile_source with at_once=Some(1) must succeed");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(
        limit, 1,
        "ForEachStart.limit must be 1 when at_once is Some(1) (source path)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// B21: at_once=0 and at_once=1 produce different workflow digests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_zero_and_one_produce_different_digests() {
    let yaml_zero = foreach_yaml("0");
    let yaml_one = foreach_yaml("1");
    let digest_zero = digest_from_yaml(&yaml_zero);
    let digest_one = digest_from_yaml(&yaml_one);

    assert_ne!(
        digest_zero, digest_one,
        "at_once=0 and at_once=1 must produce different workflow digests"
    );
}

#[test]
fn foreach_at_once_zero_and_one_source_digests_differ() {
    let yaml_zero = foreach_yaml("0");
    let yaml_one = foreach_yaml("1");
    let digest_zero = digest_from_source(&yaml_zero);
    let digest_one = digest_from_source(&yaml_one);

    assert_ne!(
        digest_zero, digest_one,
        "at_once=Some(0) and at_once=Some(1) must produce different digests (source path)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// B22: at_once=None and at_once=Some(1) produce identical digests
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_none_and_one_produce_identical_digests() {
    let yaml_none = foreach_yaml_no_atonce();
    let yaml_one = foreach_yaml("1");
    let digest_none = digest_from_yaml(&yaml_none);
    let digest_one = digest_from_yaml(&yaml_one);

    assert_eq!(
        digest_none, digest_one,
        "at_once=None and at_once=Some(1) must produce identical workflow digests"
    );
}

#[test]
fn foreach_at_once_none_and_one_source_digests_identical() {
    let yaml_none = foreach_yaml_no_atonce();
    let yaml_one = foreach_yaml("1");
    let digest_none = digest_from_source(&yaml_none);
    let digest_one = digest_from_source(&yaml_one);

    assert_eq!(
        digest_none, digest_one,
        "at_once=None and at_once=Some(1) must produce identical digests (source path)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Determinism: same YAML compiled twice → same digest
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_zero_digest_is_deterministic() {
    let yaml = foreach_yaml("0");
    let d1 = digest_from_yaml(&yaml);
    let d2 = digest_from_yaml(&yaml);
    assert_eq!(d1, d2, "at_once=0 digest must be deterministic");
}

#[test]
fn foreach_at_once_none_digest_is_deterministic() {
    let yaml = foreach_yaml_no_atonce();
    let d1 = digest_from_yaml(&yaml);
    let d2 = digest_from_yaml(&yaml);
    assert_eq!(d1, d2, "at_once=None digest must be deterministic");
}

#[test]
fn foreach_at_once_one_digest_is_deterministic() {
    let yaml = foreach_yaml("1");
    let d1 = digest_from_yaml(&yaml);
    let d2 = digest_from_yaml(&yaml);
    assert_eq!(d1, d2, "at_once=1 digest must be deterministic");
}

#[test]
fn foreach_at_once_zero_source_digest_is_deterministic() {
    let yaml = foreach_yaml("0");
    let d1 = digest_from_source(&yaml);
    let d2 = digest_from_source(&yaml);
    assert_eq!(
        d1, d2,
        "at_once=Some(0) source digest must be deterministic"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Multi-API: compile_workflow vs compile_source agree on limit and digest
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_zero_limit_agrees_across_apis() {
    let yaml = foreach_yaml("0");
    let workflow_yaml = compile_workflow(yaml.as_bytes()).expect("compile_workflow must succeed");
    let limit_yaml = extract_foreach_start_limit(&workflow_yaml);

    // Parse source from same YAML
    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow_source = compile_source(&source).expect("compile_source must succeed");
    let limit_source = extract_foreach_start_limit(&workflow_source);

    assert_eq!(
        limit_yaml, limit_source,
        "ForEachStart.limit must agree: yaml={limit_yaml}, source={limit_source}"
    );
}

#[test]
fn foreach_at_once_none_limit_agrees_across_apis() {
    let yaml = foreach_yaml_no_atonce();
    let workflow_yaml = compile_workflow(yaml.as_bytes()).expect("compile_workflow must succeed");
    let limit_yaml = extract_foreach_start_limit(&workflow_yaml);

    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow_source = compile_source(&source).expect("compile_source must succeed");
    let limit_source = extract_foreach_start_limit(&workflow_source);

    assert_eq!(
        limit_yaml, limit_source,
        "ForEachStart.limit must agree: yaml={limit_yaml}, source={limit_source}"
    );
}

#[test]
fn foreach_at_once_zero_digest_agrees_across_apis() {
    let yaml = foreach_yaml("0");
    let workflow_yaml = compile_workflow(yaml.as_bytes()).expect("compile_workflow must succeed");
    let digest_yaml = workflow_yaml.to_parts().digest;

    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow_source = compile_source(&source).expect("compile_source must succeed");
    let digest_source = workflow_source.to_parts().digest;

    assert_eq!(
        digest_yaml, digest_source,
        "Digest must agree across compile_workflow and compile_source"
    );
}

#[test]
fn foreach_at_once_one_digest_agrees_across_apis() {
    let yaml = foreach_yaml("1");
    let workflow_yaml = compile_workflow(yaml.as_bytes()).expect("compile_workflow must succeed");
    let digest_yaml = workflow_yaml.to_parts().digest;

    let source =
        vb_yaml::parse_workflow_source(&yaml).expect("YAML must parse into WorkflowSource");
    let workflow_source = compile_source(&source).expect("compile_source must succeed");
    let digest_source = workflow_source.to_parts().digest;

    assert_eq!(
        digest_yaml, digest_source,
        "Digest must agree across compile_workflow and compile_source"
    );
}

// ─────────────────────────────────────────────────────────────────────
// Boundary: at_once=0 and at_once=2 are distinct
// ─────────────────────────────────────────────────────────────────────

#[test]
fn foreach_at_once_zero_and_two_produce_different_digests() {
    let yaml_zero = foreach_yaml("0");
    let yaml_two = foreach_yaml("2");
    let digest_zero = digest_from_yaml(&yaml_zero);
    let digest_two = digest_from_yaml(&yaml_two);

    assert_ne!(
        digest_zero, digest_two,
        "at_once=0 and at_once=2 must produce different digests"
    );
}

#[test]
fn foreach_at_once_two_limit_is_two() {
    let yaml = foreach_yaml("2");
    let workflow = compile_workflow(yaml.as_bytes()).expect("at_once=2 must compile");
    let limit = extract_foreach_start_limit(&workflow);
    assert_eq!(limit, 2, "ForEachStart.limit must be 2 when at_once is 2");
}
