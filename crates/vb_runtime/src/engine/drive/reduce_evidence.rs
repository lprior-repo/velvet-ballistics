#![forbid(unsafe_code)]

//! Reduce-node slot evidence. Emits SlotWritten events for the actual
//! writes performed by each ReduceStart/ReduceNext branch.

use vb_core::frame::RunFrame;
use vb_core::ids::SlotIdx;
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

use crate::engine::drive::recovery::DriveState;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeEngineResult};

/// Maximum number of distinct slots a Reduce node can write.
const MAX_REDUCE_SLOTS: usize = 3;

/// Emit slot-written evidence for the Reduce branch that just executed.
/// Returns Some(()) when Reduce-specific evidence was emitted (or the
/// branch legitimately wrote no slots). Returns None when the node is
/// not a Reduce control node and the caller should fall through to
/// generic handling.
pub fn reduce_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    node: &CompiledNode,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<Option<()>> {
    let slots = match reduce_branch_slots(run, node)? {
        Some(slots) => slots,
        None => return Ok(None),
    };
    emit_slots(run, evidence, drive_state, &slots)?;
    Ok(Some(()))
}

/// Determine which slots were written by the executed Reduce branch.
/// Returns None for non-Reduce nodes.
fn reduce_branch_slots(
    run: &RunFrame,
    node: &CompiledNode,
) -> RuntimeEngineResult<Option<SlotList>> {
    let (body, done) = match &node.kind {
        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => (*body, *done),
        _ => return Ok(None),
    };

    if run.pc() == done {
        match &node.kind {
            CompiledNodeKind::ReduceStart { accumulator, .. } => {
                // Empty iterator: only accumulator written.
                Ok(Some(singleton(*accumulator)))
            }
            CompiledNodeKind::ReduceNext { .. } => {
                // Exhausted: no writes.
                Ok(Some(SlotList::empty()))
            }
            _ => Ok(None),
        }
    } else if run.pc() == body {
        match &node.kind {
            CompiledNodeKind::ReduceStart { input, accumulator, .. } => {
                // Non-empty: accumulator + first item + iterator state.
                let mut slots = SlotList::empty();
                slots.push(*accumulator)?;
                slots.push(*input)?;
                if let Some(out) = node.output {
                    slots.push(out)?;
                }
                Ok(Some(slots))
            }
            CompiledNodeKind::ReduceNext { iterator_slot, .. } => {
                // Not exhausted: next item + advanced iterator state.
                let mut slots = SlotList::empty();
                if let Some(out) = node.output {
                    slots.push(out)?;
                }
                slots.push(*iterator_slot)?;
                Ok(Some(slots))
            }
            _ => Ok(None),
        }
    } else {
        // PC is neither body nor done — unexpected. Fall through
        // to generic handling so the gap is still recorded.
        Ok(None)
    }
}

/// Bounded list of slots for deduplication. Uses a fixed array to
/// avoid allocation.
#[derive(Clone, Copy)]
struct SlotList {
    slots: [Option<SlotIdx>; MAX_REDUCE_SLOTS],
    len: usize,
}

impl SlotList {
    fn empty() -> Self {
        Self {
            slots: [None, None, None],
            len: 0,
        }
    }

    /// Add a slot if not already present.
    fn push(&mut self, slot: SlotIdx) -> RuntimeEngineResult<()> {
        if self.contains(slot) {
            return Ok(());
        }
        if self.len >= MAX_REDUCE_SLOTS {
            return Err(RuntimeEngineError::Core(vb_core::errors::EngineError::InternalInvariantViolation {
                reason: "reduce evidence slot list overflow",
            }));
        }
        self.slots[self.len] = Some(slot);
        self.len += 1;
        Ok(())
    }

    fn contains(&self, slot: SlotIdx) -> bool {
        let mut i = 0;
        while i < self.len {
            if self.slots[i] == Some(slot) {
                return true;
            }
            i += 1;
        }
        false
    }
}

fn singleton(slot: SlotIdx) -> SlotList {
    let mut list = SlotList::empty();
    let _ = list.push(slot);
    list
}

fn emit_slots(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    drive_state: &mut DriveState,
    slots: &SlotList,
) -> RuntimeEngineResult<()> {
    let mut i = 0;
    while i < slots.len {
        let slot = slots.slots[i].ok_or(RuntimeEngineError::Core(
            vb_core::errors::EngineError::InternalInvariantViolation {
                reason: "reduce evidence null slot",
            },
        ))?;
        emit_one(run, evidence, drive_state, slot)?;
        i += 1;
    }
    Ok(())
}

fn emit_one(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    drive_state: &mut DriveState,
    slot: SlotIdx,
) -> RuntimeEngineResult<()> {
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
