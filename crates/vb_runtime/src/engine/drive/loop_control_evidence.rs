#![forbid(unsafe_code)]

//! Loop control node slot evidence for ForEach and Repeat. Emits
//! SlotWritten events for the actual writes performed by each
//! iteration branch.

use vb_core::engine::EngineError;
use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

use crate::engine::drive::recovery::DriveState;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeEngineResult};

use super::reduce_evidence::emit_distinct;

fn classify_for_each(
    run: &RunFrame,
    node: &CompiledNode,
    iterator: SlotIdx,
    body: StepIdx,
    done: StepIdx,
) -> RuntimeEngineResult<[Option<SlotIdx>; 3]> {
    if run.pc() == done {
        Ok([None, None, None])
    } else if run.pc() == body {
        Ok([node.output, Some(iterator), None])
    } else {
        Err(RuntimeEngineError::Core(
            EngineError::InvalidCompiledWorkflow {
                reason: "loop evidence destination mismatch",
            },
        ))
    }
}

pub(super) fn loop_control_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    node: &CompiledNode,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<Option<()>> {
    let slots = match &node.kind {
        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body,
            done,
            ..
        } => classify_for_each(run, node, *iterator_slot, *body, *done)?,
        CompiledNodeKind::RepeatCheck { attempt_slot, .. } => [Some(*attempt_slot), None, None],
        _ => return Ok(None),
    };
    emit_distinct(run, evidence, drive_state, &slots)?;
    Ok(Some(()))
}
