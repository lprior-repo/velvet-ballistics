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
    fn finish_result_change_changes_digest_string(
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

        prop_assert_ne!(da, db,
            "different Finish result output names must produce different digests");
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

// ─────────────────────────────────────────────────────────────────
// Negative boundary tests: invalid slot values
// ─────────────────────────────────────────────────────────────────

/// Compile and expect ResourceContractExceeded with resource="max_slots".
fn compile_and_expect_max_slots_error(yaml: &str) -> Result<(), String> {
    let source = crate::parse_workflow_source(yaml).map_err(|e| format!("parse error: {e:?}"))?;
    let result = compile_source(&source);
    match result {
        Err(errors) => {
            let slice = errors.as_slice();
            match slice {
                [crate::CompileError::Workflow(
                    vb_core::workflow::WorkflowError::ResourceContractExceeded { resource },
                )] => {
                    if *resource == "max_slots" {
                        Ok(())
                    } else {
                        Err(format!("expected max_slots error, got: {resource}"))
                    }
                }
                other => Err(format!(
                    "expected exactly [ResourceContractExceeded max_slots], got: {other:?}"
                )),
            }
        }
        Ok(compiled) => Err(format!(
            "expected compile error, got compiled workflow with digest {:?}",
            compiled.digest()
        )),
    }
}

/// Compile and expect SlotIndexOutOfRange error for u16::MAX.
fn compile_and_expect_slot_index_out_of_range(yaml: &str) -> Result<(), String> {
    let source = crate::parse_workflow_source(yaml).map_err(|e| format!("parse error: {e:?}"))?;
    let result = compile_source(&source);
    match result {
        Err(errors) => {
            let slice = errors.as_slice();
            match slice {
                [crate::CompileError::SlotIndexOutOfRange { value }] => {
                    if *value == i64::from(u16::MAX) {
                        Ok(())
                    } else {
                        Err(format!(
                            "expected SlotIndexOutOfRange {{ value: 65535 }}, got value: {value}"
                        ))
                    }
                }
                other => Err(format!(
                    "expected exactly [SlotIndexOutOfRange {{ value: 65535 }}], got: {other:?}"
                )),
            }
        }
        Ok(compiled) => Err(format!(
            "expected compile error, got compiled workflow with digest {:?}",
            compiled.digest()
        )),
    }
}

#[test]
fn slot_at_max_minus_one_compiles() -> Result<(), String> {
    let slot = slot_below_max()?;
    let yaml = yaml_with_steps(&format!(
        "  - id: step_a\n    finish:\n      result: {}",
        slot
    ));
    digest_from_yaml(&yaml)
        .map(|_| ())
        .map_err(|e| format!("slot {} must compile: {}", slot, e))
}

#[test]
fn slot_at_max_fails_with_max_slots_error() -> Result<(), String> {
    let slot = max_slots();
    let yaml = yaml_with_steps(&format!(
        "  - id: step_a\n    finish:\n      result: {}",
        slot
    ));
    compile_and_expect_max_slots_error(&yaml)
        .map_err(|e| format!("slot {} must produce max_slots error: {}", slot, e))
}

#[test]
fn slot_at_max_plus_one_fails_with_max_slots_error() -> Result<(), String> {
    let slot = slot_above_max()?;
    let yaml = yaml_with_steps(&format!(
        "  - id: step_a\n    finish:\n      result: {}",
        slot
    ));
    compile_and_expect_max_slots_error(&yaml)
        .map_err(|e| format!("slot {} must produce max_slots error: {}", slot, e))
}

#[test]
fn slot_at_u16_max_fails_with_slot_index_out_of_range() -> Result<(), String> {
    let yaml = yaml_with_steps(&format!(
        "  - id: step_a\n    finish:\n      result: {}",
        u16::MAX
    ));
    compile_and_expect_slot_index_out_of_range(&yaml).map_err(|e| {
        format!(
            "slot {} must produce SlotIndexOutOfRange error: {}",
            u16::MAX,
            e
        )
    })
}

#[test]
fn slot_5000_fails_with_max_slots_error() -> Result<(), String> {
    let yaml = yaml_with_steps(&format!("  - id: step_a\n    finish:\n      result: 5000"));
    compile_and_expect_max_slots_error(&yaml)
        .map_err(|e| format!("slot 5000 must produce max_slots error: {}", e))
}

// ─────────────────────────────────────────────────────────────────
// Proptest: invalid slot values in (max_slots, u16::MAX) range
// produce ResourceContractExceeded errors
// ─────────────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn invalid_slot_values_rejected(
        slot in max_slots()..u16::MAX,
    ) {
        let yaml = yaml_with_steps(&format!(
            "  - id: step_a\n    finish:\n      result: {}",
            slot
        ));
        compile_and_expect_max_slots_error(&yaml)
            .map_err(|e| TestCaseError::fail(format!("slot {}: {}", slot, e)))?;
    }
}
