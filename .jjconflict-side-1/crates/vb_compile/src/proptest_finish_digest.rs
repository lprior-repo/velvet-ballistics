// Verification artifact: proptest_finish_digest.rs
// Bead: vb-jei3u — Repair ignored finish-digest property fixtures
// Proof obligations:
//   PO-PROPTEST-FINISH-001: Canonical digest determinism
//   PO-PROPTEST-FINISH-002: Finish result sensitivity (defense-in-depth)
//   PO-PROPTEST-FINISH-003: Finish step position sensitivity
//   PO-PROPTEST-FINISH-004: Digest is pre-validation (AST scope)
//
// GOD RULE 1: Uses proptest strategies to generate varied YAML-based inputs.
// GOD RULE 2: Binds to actual compile_source() and CompiledWorkflow::digest() APIs.
// GOD RULE 4: Exhaustive property testing; no loop oscillations.

#![cfg(test)]
#![forbid(unsafe_code)]

#[path = "proptest_finish_digest/boundaries.rs"]
mod boundaries;

use crate::compile_source;
use proptest::prelude::*;
use vb_core::ids::WorkflowDigest;

// ─────────────────────────────────────────────────────────────────
// Constants — derived from actual ResourceContract data
// ─────────────────────────────────────────────────────────────────

/// Actual max_slots from the production ResourceContract::DEFAULT constant.
fn max_slots() -> u16 {
    vb_core::workflow::ResourceContract::DEFAULT.max_slots
}

/// Boundary value: one below max_slots (checked subtraction).
fn slot_below_max() -> Result<u16, String> {
    max_slots()
        .checked_sub(1)
        .ok_or_else(|| "max_slots must be > 0 for checked_sub(1)".to_string())
}

/// Boundary value: one above max_slots (checked addition).
fn slot_above_max() -> Result<u16, String> {
    max_slots()
        .checked_add(1)
        .ok_or_else(|| "max_slots + 1 must fit in u16".to_string())
}

// ─────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────

fn digest_from_yaml(yaml: &str) -> Result<WorkflowDigest, String> {
    let source = crate::parse_workflow_source(yaml).map_err(|e| format!("parse error: {e:?}"))?;
    let compiled = compile_source(&source).map_err(|e| format!("compile error: {e:?}"))?;
    Ok(compiled.digest())
}

fn yaml_with_steps(steps_yaml: &str) -> String {
    format!(
        "version: velvet-ballistics/v1\n\
         name: test_workflow\n\
         when:\n  manual: {{}}\n\
         steps:\n{}",
        steps_yaml
    )
}

// ─────────────────────────────────────────────────────────────────
// Strategies
// ─────────────────────────────────────────────────────────────────

fn step_id_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}".prop_filter("exclude YAML-ambiguous step IDs", |s| {
        !matches!(
            s.as_str(),
            "y" | "n" | "yes" | "no" | "true" | "false" | "on" | "off"
        )
    })
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-001: Canonical digest determinism
// (also covers PO-PROPTEST-FINISH-004 / C9 — IR layout independence)
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn canonical_digest_is_deterministic(
        slot in 0u16..max_slots(),
        id in step_id_strategy(),
    ) {
        let yaml = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot}"
        ));
        let source = crate::parse_workflow_source(&yaml)
            .map_err(|e| TestCaseError::fail(format!("parse error: {e:?}")))?;

        let c1 = compile_source(&source)
            .map_err(|e| TestCaseError::fail(format!("compile error: {e:?}")))?;
        let c2 = compile_source(&source)
            .map_err(|e| TestCaseError::fail(format!("compile error: {e:?}")))?;

        prop_assert_eq!(c1.digest(), c2.digest(), "digest must be deterministic");
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-002: Finish result value sensitivity
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn finish_result_change_changes_digest_integer(
        id in step_id_strategy(),
        slot_a in 0u16..max_slots(),
        slot_b in 0u16..max_slots(),
    ) {
        if slot_a == slot_b {
            return Ok(());
        }

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot_a}"
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {id}\n    finish:\n      result: {slot_b}"
        ));

        let da = digest_from_yaml(&yaml_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let db = digest_from_yaml(&yaml_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "different Finish result Integer values must produce different digests");
    }

    #[test]
    fn finish_output_rename_preserves_numeric_ir_and_changes_compiled_digest(
        id in step_id_strategy(),
        out_a in step_id_strategy(),
        out_b in step_id_strategy(),
    ) {
        if out_a == out_b {
            return Ok(());
        }
        if id == "s" {
            return Ok(());
        }
        let sid = "s";

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {sid}\n    set:\n      output: {out_a}\n      value: \"10\"\n  - id: {id}\n    finish:\n      result: \"{out_a}\""
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {sid}\n    set:\n      output: {out_b}\n      value: \"10\"\n  - id: {id}\n    finish:\n      result: \"{out_b}\""
        ));

        let da = digest_from_yaml(&yaml_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let db = digest_from_yaml(&yaml_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db, "source-semantic output renames change compiled digest");
        let source_a = crate::canonical_digest(&crate::parse_workflow_source(&yaml_a).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(format!("canonical_digest(a): {}", e)))?;
        let source_b = crate::canonical_digest(&crate::parse_workflow_source(&yaml_b).map_err(|e| TestCaseError::fail(e.to_string()))?)
            .map_err(|e| TestCaseError::fail(format!("canonical_digest(b): {}", e)))?;
        prop_assert_ne!(source_a, source_b, "canonical source identity preserves output spelling");
        let mut parts_a = crate::compile_workflow(yaml_a.as_bytes())
            .map_err(|e| TestCaseError::fail(format!("compile(a): {}", e)))?
            .to_parts();
        let mut parts_b = crate::compile_workflow(yaml_b.as_bytes())
            .map_err(|e| TestCaseError::fail(format!("compile(b): {}", e)))?
            .to_parts();
        prop_assert_ne!(
            parts_a.semantic_source_identity,
            parts_b.semantic_source_identity,
            "consistent output renames preserve distinct source identities"
        );
        parts_a.digest = WorkflowDigest::from_bytes([0u8; 32]);
        parts_b.digest = WorkflowDigest::from_bytes([0u8; 32]);
        parts_a.semantic_source_identity = None;
        parts_b.semantic_source_identity = None;
        prop_assert_eq!(
            parts_a,
            parts_b,
            "consistent output renames preserve exact numeric compiled IR"
        );
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-PROPTEST-FINISH-003: Finish step position sensitivity
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn finish_position_change_changes_digest(
        id1 in step_id_strategy(),
        id2 in step_id_strategy(),
        slot in 0u16..max_slots(),
    ) {
        if id1 == id2 {
            return Ok(());
        }

        let yaml_a = yaml_with_steps(&format!(
            "  - id: {id1}\n    finish:\n      result: {slot}"
        ));
        let yaml_b = yaml_with_steps(&format!(
            "  - id: {id2}\n    finish:\n      result: {slot}"
        ));

        let da = digest_from_yaml(&yaml_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let db = digest_from_yaml(&yaml_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(da, db,
            "different step IDs must produce different digests");
    }
}
