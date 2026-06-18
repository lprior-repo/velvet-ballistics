//! Node-helper error-path tests.
//!
//! Exercises the error branches in `node_helpers`:
//! - `set_const` with invalid const index (GAP-ERROR-006)
//! - `copy_slot` with uninitialized source (GAP-ERROR-007)
//! - `finish_run` with out-of-bounds slot (GAP-ERROR-008)

use crate::engine::node_helpers;
use crate::errors::EngineError;
use crate::frame::RunFrame;
use crate::ids::{ConstIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use crate::value::ConstValue;
use crate::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ResourceContract, WorkflowParts,
};

fn ensure_equal<T>(actual: T, expected: T) -> Result<(), String>
where
    T: core::fmt::Debug + PartialEq,
{
    if actual == expected {
        Ok(())
    } else {
        Err(format!("expected {expected:?}, found {actual:?}"))
    }
}

// ---------------------------------------------------------------------------
// 6. node_helpers::set_const — ConstOutOfBounds
// ---------------------------------------------------------------------------

/// GAP-ERROR-006: When `set_const` is called with a `ConstIdx` that exceeds
/// the constant pool, it returns `EngineError::ConstOutOfBounds`.
///
/// We build a valid workflow (1 constant) and then directly invoke
/// `node_helpers::set_const` with a `ConstIdx` beyond the pool, bypassing
/// the workflow validator.
#[test]
fn set_const_invalid_const_index_returns_const_out_of_bounds() -> Result<(), String> {
    // Valid workflow with 1 constant — only index 0 is valid.
    let plan = CompiledWorkflow::try_from_parts(WorkflowParts {
        name: Box::<str>::from("set_const_oob"),
        digest: WorkflowDigest::from_bytes([0xF6; 32]),
        nodes: vec![
            CompiledNode {
                id: StepIdx::new(0),
                output: None,
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Nop,
            },
        ]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::I64(42)].into_boxed_slice(), // Only 1 constant
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
    .map_err(|e| e.to_string())?;

    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 1).unwrap();
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(99),
        },
    };

    let result = node_helpers::set_const(&plan, &mut run, &node, ConstIdx::new(99));

    match result {
        Err(EngineError::ConstOutOfBounds { index }) if index == ConstIdx::new(99) => Ok(()),
        Err(other) => Err(format!("expected ConstOutOfBounds(99), got {other:?}"))?,
        Ok(_) => Err("expected Err(ConstOutOfBounds), got Ok".to_string())?,
    }
}

// ---------------------------------------------------------------------------
// 7. node_helpers::copy_slot — SlotUninitialized
// ---------------------------------------------------------------------------

/// GAP-ERROR-007: When `copy_slot` is called with a source slot that has
/// never been written to, it returns `EngineError::SlotUninitialized`.
#[test]
fn copy_slot_uninitialized_source_returns_slot_uninitialized() -> Result<(), String> {
    let mut run = RunFrame::new(RunId::new(1), StepIdx::new(0), 3, 3).map_err(|e| e.to_string())?;
    let node = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Copy {
            source: SlotIdx::new(2),
        },
    };

    let result = node_helpers::copy_slot(&mut run, &node, SlotIdx::new(2));

    match result {
        Err(EngineError::SlotUninitialized { slot }) if slot == SlotIdx::new(2) => Ok(()),
        Err(other) => Err(format!("expected SlotUninitialized(2), got {other:?}"))?,
        Ok(_) => Err("expected Err(SlotUninitialized), got Ok".to_string())?,
    }
}

// ---------------------------------------------------------------------------
// 8. node_helpers::finish_run — SlotOutOfBounds
// ---------------------------------------------------------------------------

/// GAP-ERROR-008: When `finish_run` is called with a `SlotIdx` that exceeds
/// the run frame's slot count, it returns `EngineError::SlotOutOfBounds`.
///
/// We create a RunFrame with 10 slots and directly call `finish_run` with
/// `SlotIdx(99)`, which is out of bounds.
#[test]
fn finish_run_slot_out_of_bounds_returns_slot_out_of_bounds() -> Result<(), String> {
    // Create a RunFrame with 10 slots.
    let mut run =
        RunFrame::new(RunId::new(1), StepIdx::new(0), 10, 10).map_err(|e| e.to_string())?;

    // Call finish_run with a slot index beyond the frame's 10 slots.
    let result = node_helpers::finish_run(&mut run, SlotIdx::new(99));

    match result {
        Err(EngineError::SlotOutOfBounds { slot }) if slot == SlotIdx::new(99) => Ok(()),
        Err(other) => Err(format!("expected SlotOutOfBounds(99), got {other:?}"))?,
        Ok(_) => Err("expected Err(SlotOutOfBounds), got Ok".to_string())?,
    }
}
