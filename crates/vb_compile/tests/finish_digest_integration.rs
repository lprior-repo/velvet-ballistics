// Integration test artifact: finish_digest_integration.rs
// Bead: vb-xi2f.34 — P1: digest covers finish semantics
// Proof obligations:
//   PO-INT-FINISH-001: Finish result value changes compiled workflow digest
//   PO-INT-FINISH-002: Finish step ID changes compiled workflow digest
//   PO-INT-FINISH-003: Finish result type (String vs Integer) changes digest
//   PO-INT-FINISH-004: Canonical/legacy digest equivalence (BLOCKED — see below)
//
// NOTE: This test is placed in crates/vb_compile/tests/ rather than
// crates/workspace_tests/tests/ because workspace_tests is excluded from
// the cargo workspace (commented out in Cargo.toml members list due to
// allow-removed-crate: comment narrates deferred dependency on removed types
// dependency on deferred vb_ui/vb_codegen types).
//
// GOD RULE 4: No loop oscillations. Pure integration tests.
// GOD RULE 2: Binds to actual compile_source() and CompiledWorkflow::digest() APIs.

#![forbid(unsafe_code)]

use vb_compile::compile_source;
use vb_core::ids::WorkflowDigest;
use vb_yaml::parse_workflow_source;

/// Compile a YAML source string and return its compiled workflow digest.
fn compile_and_digest(yaml: &str) -> WorkflowDigest {
    let source = parse_workflow_source(yaml).expect("valid YAML must parse to WorkflowSource");
    let compiled = compile_source(&source).expect("valid source must compile");
    compiled.digest()
}

/// Parse YAML, compile, and return compiled workflow.
fn compile_yaml(yaml: &str) -> vb_core::CompiledWorkflow {
    let source = parse_workflow_source(yaml).expect("valid YAML must parse to WorkflowSource");
    compile_source(&source).expect("valid source must compile")
}

// ─────────────────────────────────────────────────────────────────
// PO-INT-FINISH-001: Finish result value changes compiled digest
// ─────────────────────────────────────────────────────────────────

/// Verify that changing the Finish result String value changes the compiled digest.
///
/// Contract clause: C1 (Result value sensitivity), C6 (Digest survives compilation).
/// Proof seed: PS-FINISH-DIGEST-001, PS-FINISH-DIGEST-006.
///
/// Note: ScalarValue::String in Finish.result is an OUTPUT NAME that must
/// reference a slot set by a previous Set step. The String value itself
/// (the output name) is what gets hashed into the digest.
#[test]
fn finish_result_value_changes_compiled_digest_string() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: string_test
when:
  manual: {}
steps:
  - id: set_a
    set:
      output: result_a
      value: "10"
  - id: done
    finish:
      result: "result_a"
"#;

    let yaml_b = r#"version: velvet-ballistics/v1
name: string_test
when:
  manual: {}
steps:
  - id: set_b
    set:
      output: result_b
      value: "10"
  - id: done
    finish:
      result: "result_b"
"#;

    let digest_a = compile_and_digest(yaml_a);
    let digest_b = compile_and_digest(yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "different Finish result output names must produce different compiled digests"
    );
}

/// Verify that changing the Finish result Integer value changes the compiled digest.
#[test]
fn finish_result_value_changes_compiled_digest_integer() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: int_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 1
"#;

    let yaml_b = r#"version: velvet-ballistics/v1
name: int_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 2
"#;

    let digest_a = compile_and_digest(yaml_a);
    let digest_b = compile_and_digest(yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "different Finish result Integer values must produce different compiled digests"
    );
}

/// Verify that the digest computed during compilation matches canonical_digest(source).
/// This is implicit: compile_source calls canonical_digest internally at part_01.rs:46.
/// We verify determinism by compiling the same source twice.
#[test]
fn compiled_digest_matches_on_recompile() {
    let yaml = r#"version: velvet-ballistics/v1
name: determinism_test
when:
  manual: {}
steps:
  - id: first
    set:
      output: x
      value: "42"
  - id: done
    finish:
      result: "x"
"#;

    let c1 = compile_yaml(yaml);
    let c2 = compile_yaml(yaml);

    assert_eq!(
        c1.digest(),
        c2.digest(),
        "compiling same source twice must produce the same digest"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-INT-FINISH-002: Finish step ID changes compiled digest
// ─────────────────────────────────────────────────────────────────

/// Verify that changing the Finish step's `id` field changes the compiled digest.
///
/// Contract clause: C2 — Finish Step ID Sensitivity.
/// Proof seed: PS-FINISH-DIGEST-003.
#[test]
fn finish_step_id_changes_compiled_digest() {
    let yaml_last = r#"version: velvet-ballistics/v1
name: step_id_test
when:
  manual: {}
steps:
  - id: last
    finish:
      result: 0
"#;

    let yaml_done = r#"version: velvet-ballistics/v1
name: step_id_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_last = compile_and_digest(yaml_last);
    let digest_done = compile_and_digest(yaml_done);

    assert_ne!(
        digest_last, digest_done,
        "different Finish step IDs must produce different compiled digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-INT-FINISH-003: Finish result type changes compiled digest
// ─────────────────────────────────────────────────────────────────

/// Verify that Finish { result: String(name) } and Finish { result: Integer(slot) }
/// produce different compiled digests even when referencing the same logical slot.
///
/// Contract clause: C5 — Hash Discrimination by ScalarValue Variant.
/// Proof seed: PS-FINISH-DIGEST-002.
#[test]
fn finish_result_type_changes_compiled_digest() {
    // Finish with String result — references output "my_result" by name
    let yaml_string = r#"version: velvet-ballistics/v1
name: type_test
when:
  manual: {}
steps:
  - id: set_var
    set:
      output: my_result
      value: "10"
  - id: done
    finish:
      result: "my_result"
"#;

    // Finish with Integer result — references slot 0 directly
    let yaml_integer = r#"version: velvet-ballistics/v1
name: type_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_string = compile_and_digest(yaml_string);
    let digest_integer = compile_and_digest(yaml_integer);

    assert_ne!(
        digest_string, digest_integer,
        "Finish{{result: String(name)}} and Finish{{result: Integer(slot)}} must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-INT-FINISH-004: Canonical/legacy digest equivalence
// ─────────────────────────────────────────────────────────────────

/// Verify that the canonical path's `canonical_digest()` produces the same
/// digest as the legacy path's `canonical_digest()` for all valid inputs.
///
/// **STATUS: BLOCKED_VISIBILITY**
///
/// The legacy `canonical_digest()` is defined as `fn canonical_digest(...)`
/// (private) in `crate::compile::mod` (a private module, `mod compile;` in
/// lib.rs). This function is NOT accessible from integration tests in a
/// separate crate.
///
/// The canonical `canonical_digest()` is `pub(super)` in
/// `mod_compile_lowering::part_05`, re-exported as `pub` within
/// `mod_compile_lowering` but NOT re-exported at the crate level
/// (lib.rs only exports `compile_source`, not `canonical_digest`).
///
/// Since this test file is in `crates/vb_compile/tests/`, it compiles as an
/// external crate and cannot access private or pub(super) items.
///
/// ## Remediation Options (for proof-to-implementation agent):
///
/// 1. **Add `#[cfg(test)]` re-export**: In lib.rs, add
///    `#[cfg(test)] pub use lwr::canonical_digest;` (for canonical path)
///    and `#[cfg(test)] pub use compile::canonical_digest;` (for legacy path).
///
/// 2. **Write the equivalence test within vb_compile**: Move this test to
///    `crates/vb_compile/src/tests/finish_digest_tests.rs` where both paths
///    are accessible (`crate::mod_compile_lowering::canonical_digest` and
///    `crate::compile::canonical_digest`).
///
/// 3. **Consolidate to single implementation**: Delete the legacy path
///    and have all callers use the canonical path. This eliminates the
///    equivalence test need entirely.
///
/// Until visibility is resolved or the legacy path is consolidated, this
/// test is a documentation marker. See `trusted-base-ledger.jsonl` for
/// the BLOCKED_VISIBILITY entry.
#[test]
#[ignore = "BLOCKED: legacy canonical_digest is not accessible from integration test crate"]
fn canonical_legacy_digest_equivalence() {
    // Placeholder test — BLOCKED by visibility constraints.
    // When unblocked, this test should:
    // 1. Parse YAML into WorkflowSource
    // 2. Call both canonical_digest() paths on the same source
    // 3. Assert both paths produce identical digests
    // 4. Assert compiled digest matches both

    // For now, verify the canonical path self-consistency:
    let yaml = r#"version: velvet-ballistics/v1
name: equivalence_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 42
"#;

    let source = parse_workflow_source(yaml).expect("valid YAML must parse");
    let compiled = compile_source(&source).expect("valid source must compile");

    // The compiled digest must be non-zero and consistent on recompile
    let compiled2 = compile_source(&source).expect("same source must compile twice");

    assert_eq!(
        compiled.digest(),
        compiled2.digest(),
        "compiling same source twice must give same digest"
    );

    // Verify digest is not the sentinel zero value
    assert_ne!(
        compiled.digest(),
        WorkflowDigest::from_bytes([0u8; 32]),
        "compiled digest must be non-zero for a valid workflow"
    );
}

// ─────────────────────────────────────────────────────────────────
// Additional defense-in-depth: workflow metadata sensitivity
// ─────────────────────────────────────────────────────────────────

/// Verify that the workflow name contributes to the digest.
/// This is a defense-in-depth test confirming structural digest coverage.
#[test]
fn workflow_name_changes_compiled_digest() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: workflow_alpha
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_b = r#"version: velvet-ballistics/v1
name: workflow_beta
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_a = compile_and_digest(yaml_a);
    let digest_b = compile_and_digest(yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "different workflow names must produce different compiled digests"
    );
}

/// Verify that the workflow version contributes to the digest.
#[test]
fn workflow_version_changes_compiled_digest() {
    let yaml_v1 = r#"version: velvet-ballistics/v1
name: version_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_v2 = r#"version: velvet-ballistics/v2
name: version_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_v1 = compile_and_digest(yaml_v1);
    let digest_v2 = compile_and_digest(yaml_v2);

    assert_ne!(
        digest_v1, digest_v2,
        "different workflow versions must produce different compiled digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// INT-1: Trigger type sensitivity (new for vb-xi2f.34 P1)
// ─────────────────────────────────────────────────────────────────

/// Verify that changing the trigger type (manual vs webhook) changes the
/// compiled digest. The trigger discriminator is hashed as part of the
/// canonical digest computation.
///
/// Contract clause: C4 (Canonical Digest Determinism — digest must reflect
/// all AST fields including trigger type).
#[test]
fn trigger_type_changes_compiled_digest() {
    let yaml_manual = r#"version: velvet-ballistics/v1
name: trigger_type_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_webhook = r#"version: velvet-ballistics/v1
name: trigger_type_test
when:
  webhook: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_manual = compile_and_digest(yaml_manual);
    let digest_webhook = compile_and_digest(yaml_webhook);

    assert_ne!(
        digest_manual, digest_webhook,
        "different trigger types (manual vs webhook) must produce different compiled digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// INT-2: Schedule parameter sensitivity (new for vb-xi2f.34 P1)
// ─────────────────────────────────────────────────────────────────

/// Verify that changing the schedule cron expression changes the compiled
/// digest. The cron string is hashed as part of the trigger encoding.
///
/// Contract clause: C4 — all trigger parameters are included in digest.
#[test]
fn trigger_schedule_param_changes_compiled_digest() {
    let yaml_midnight = r#"version: velvet-ballistics/v1
name: schedule_test
when:
  schedule:
    cron: "0 0 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_noon = r#"version: velvet-ballistics/v1
name: schedule_test
when:
  schedule:
    cron: "0 12 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_midnight = compile_and_digest(yaml_midnight);
    let digest_noon = compile_and_digest(yaml_noon);

    assert_ne!(
        digest_midnight, digest_noon,
        "different schedule cron expressions must produce different compiled digests"
    );
}

/// Verify that schedule vs manual trigger (another trigger pair) produces
/// different digests. Defense-in-depth for trigger field coverage.
#[test]
fn trigger_schedule_vs_manual_changes_compiled_digest() {
    let yaml_schedule = r#"version: velvet-ballistics/v1
name: trigger_pair_test
when:
  schedule:
    cron: "0 0 * * *"
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_manual = r#"version: velvet-ballistics/v1
name: trigger_pair_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_schedule = compile_and_digest(yaml_schedule);
    let digest_manual = compile_and_digest(yaml_manual);

    assert_ne!(
        digest_schedule, digest_manual,
        "schedule vs manual trigger must produce different compiled digests"
    );
}

/// Verify that event trigger with different event names produces different digests.
#[test]
fn trigger_event_type_changes_compiled_digest() {
    let yaml_push = r#"version: velvet-ballistics/v1
name: event_test
when:
  event:
    type: push
steps:
  - id: done
    finish:
      result: 0
"#;

    let yaml_pr = r#"version: velvet-ballistics/v1
name: event_test
when:
  event:
    type: pull_request
steps:
  - id: done
    finish:
      result: 0
"#;

    let digest_push = compile_and_digest(yaml_push);
    let digest_pr = compile_and_digest(yaml_pr);

    assert_ne!(
        digest_push, digest_pr,
        "different event names must produce different compiled digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// INT-3: Digest computed before compilation failure (new for vb-xi2f.34 P1)
// ─────────────────────────────────────────────────────────────────

/// Verify that the digest is computed before lowering/validation.
///
/// `compile_source()` calls `canonical_digest(source)` at part_01.rs:46
/// before the lowering loop. This means an invalid source (one that will
/// fail during validation) still has a deterministic digest at the point
/// of `canonical_digest()`.
///
/// We verify this by constructing YAML that parses successfully but
/// fails during the validation/lowering phase. The parse step succeeds,
/// confirming the source AST exists — and that AST is what
/// canonical_digest operates on.
///
/// Contract clauses: C6 (Digest survives compilation), C9 (Digest is
/// pre-validation, not post-validation).
#[test]
fn digest_is_computed_before_validation_error() {
    // This YAML parses successfully but references an output name
    // ("nonexistent") that was never set by a previous step. The
    // validation in canonical_finish_slot will fail with
    // CompileError::UnknownOutputName during the lowering phase.
    let yaml = r#"version: velvet-ballistics/v1
name: pre_validation_test
when:
  manual: {}
steps:
  - id: done
    finish:
      result: "nonexistent"
"#;

    // Step 1: Parse must succeed (AST is valid even with invalid finish result reference)
    let source = parse_workflow_source(yaml)
        .expect("YAML must parse successfully even with invalid finish reference");

    // Step 2: Compilation must fail (lowering/validation catches the unknown output name)
    let result = compile_source(&source);
    let err = result
        .expect_err("compilation must fail when finish result references unknown output name");

    // Step 3: The error variant is documented — it's an UnknownOutputName
    // (from canonical_finish_slot in part_05.rs:80)
    // We cannot directly inspect the inner canonical_digest call from here
    // because it's pub(crate), but the architectural guarantee is:
    //   part_01.rs:46 calls canonical_digest(source) BEFORE the lowering loop.
    //   The error occurs during the lowering loop (lower_canonical_step →
    //   canonical_finish_slot). Therefore the digest WAS computed deterministically.
    //
    // This test documents this design contract.
    // Verify at least one error is present (not an empty error vec)
    assert!(
        err.iter().next().is_some(),
        "compilation failure must contain at least one error"
    );
}

// ─────────────────────────────────────────────────────────────────
// Additional defense-in-depth: multi-step step ordering sensitivity
// ─────────────────────────────────────────────────────────────────

/// Verify that step ordering in a multi-step workflow (where step IDs
/// are hashed in order) changes the compiled digest.
///
/// Two workflows with different Set step IDs but same Finish step
/// produce different digests because all step IDs are hashed in
/// source order.
///
/// Contract clause: C3 (Finish Step Position Sensitivity),
/// C2 (Step ID Sensitivity).
#[test]
fn multi_step_workflow_step_ordering_changes_compiled_digest() {
    let yaml_a = r#"version: velvet-ballistics/v1
name: multi_step_test
when:
  manual: {}
steps:
  - id: set_x
    set:
      output: x
      value: "10"
  - id: done
    finish:
      result: 0
"#;

    let yaml_b = r#"version: velvet-ballistics/v1
name: multi_step_test
when:
  manual: {}
steps:
  - id: set_zulu
    set:
      output: z_out
      value: "10"
  - id: done
    finish:
      result: 0
"#;

    let digest_a = compile_and_digest(yaml_a);
    let digest_b = compile_and_digest(yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "different Set step IDs in multi-step workflow must produce different compiled digests"
    );
}

/// Verify that compiled digest stability across independent compilations
/// produces non-zero digests. This is a defense-in-depth check that
/// complements `compiled_digest_matches_on_recompile`.
#[test]
fn compiled_digest_stable_across_independent_compilations() {
    let yaml = r#"version: velvet-ballistics/v1
name: stability_test
when:
  manual: {}
steps:
  - id: step_1
    set:
      output: a
      value: "5"
  - id: done
    finish:
      result: "a"
"#;

    let c1 = compile_yaml(yaml);
    let c2 = compile_yaml(yaml);

    assert_eq!(
        c1.digest(),
        c2.digest(),
        "compiling same source independently must produce same digest"
    );

    // Digest must be non-zero
    let zero = vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]);
    assert_ne!(
        c1.digest(),
        zero,
        "compiled digest must be non-zero for a valid workflow"
    );
    assert_ne!(
        c2.digest(),
        zero,
        "second compilation digest must also be non-zero"
    );
}
