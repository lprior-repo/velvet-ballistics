// Boundary cases for the Finish digest properties in the parent test module.

use super::{max_slots, slot_above_max, slot_below_max, yaml_with_steps};
use crate::compile_source;
use proptest::prelude::*;

/// Compile and expect ResourceContractExceeded with resource="max_slots".
fn compile_and_expect_max_slots_error(yaml: &str) -> Result<(), String> {
    let source = crate::parse_workflow_source(yaml).map_err(|e| format!("parse error: {e:?}"))?;
    let result = compile_source(&source);
    match result {
        Err(errors) => {
            let slice = errors.as_slice();
            match slice {
                [
                    crate::CompileError::Workflow(
                        vb_core::workflow::WorkflowError::ResourceContractExceeded { resource },
                    ),
                ] => {
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
    super::digest_from_yaml(&yaml)
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
