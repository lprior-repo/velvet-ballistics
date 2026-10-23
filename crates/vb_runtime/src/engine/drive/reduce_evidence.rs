#![forbid(unsafe_code)]

//! Reduce-node slot evidence. Emits SlotWritten events for the actual
//! writes performed by each ReduceStart/ReduceNext branch.

use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

use crate::engine::drive::recovery::DriveState;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeEngineResult};

/// Emit slot-written evidence for the Reduce branch that just executed.
/// Returns Some(()) when Reduce-specific evidence was emitted. Returns
/// None when the node is not a Reduce control node.
pub(super) fn reduce_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    node: &CompiledNode,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<Option<()>> {
    let slots = match &node.kind {
        CompiledNodeKind::ReduceStart {
            input,
            accumulator,
            initial: _,
            body,
            done,
        } => classify_reduce_start(run, node, *input, *accumulator, *body, *done),
        CompiledNodeKind::ReduceNext {
            iterator_slot,
            body,
            done,
        } => classify_reduce_next(run, node, *iterator_slot, *body, *done),
        _ => return Ok(None),
    };

    for (position, &slot) in slots.iter().enumerate() {
        if slot.is_none() {
            continue;
        }
        let earlier_has = slots
            .iter()
            .take(position)
            .any(|prev| *prev == slot);
        if earlier_has {
            continue;
        }
        emit_if_some(run, evidence, drive_state, slot)?;
    }

    Ok(Some(()))
}

fn classify_reduce_start(
    run: &RunFrame,
    node: &CompiledNode,
    input: SlotIdx,
    accumulator: SlotIdx,
    body: vb_core::ids::StepIdx,
    done: vb_core::ids::StepIdx,
) -> [Option<SlotIdx>; 3] {
    if run.pc() == done {
        [Some(accumulator), None, None]
    } else if run.pc() == body {
        [Some(accumulator), Some(input), node.output]
    } else {
        [None, None, None]
    }
}

fn classify_reduce_next(
    run: &RunFrame,
    node: &CompiledNode,
    iterator_slot: SlotIdx,
    body: vb_core::ids::StepIdx,
    done: vb_core::ids::StepIdx,
) -> [Option<SlotIdx>; 3] {
    if run.pc() == done {
        [None, None, None]
    } else if run.pc() == body {
        [node.output, Some(iterator_slot), None]
    } else {
        [None, None, None]
    }
}

fn emit_if_some(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    drive_state: &mut DriveState,
    slot: Option<SlotIdx>,
) -> RuntimeEngineResult<()> {
    let Some(slot) = slot else {
        return Ok(());
    };
    match run.read_slot(slot) {
        Ok(value) => {
            let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
            evidence
                .push_slot_written_with_taint(slot, *value, taint)
                .map_err(RuntimeEngineError::Core)?;
        }
        Err(_) => {
            drive_state.record_evidence_gap(slot);
        }
    }
    Ok(())
}
