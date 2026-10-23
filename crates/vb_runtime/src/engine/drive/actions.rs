#![forbid(unsafe_code)]

//! Action-related drive helpers: emit slot evidence, locate the
//! collected/next/finished slot for a node, and compute the maximum
//! parallel-in-flight branch count for the workflow.

use vb_core::frame::RunFrame;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::workflow::{CompiledNode, CompiledNodeKind, CompiledWorkflow};

use crate::engine::drive::recovery::DriveState;
use crate::engine::types::{EvidenceCollector, RuntimeEngineError, RuntimeEngineResult};
use crate::primitives::collect::CollectStates;

/// Emits a `SlotWritten` evidence event for the just-finished step when
/// the node has a collect-style output slot or a plain `output` slot.
/// `read_slot` errors are recorded on the drive state (RE-004) and the
/// function returns Ok — push_* capacity errors still propagate via `?`.
pub(crate) fn emit_slot_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    collect_states: &CollectStates,
    node: &CompiledNode,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<()> {
    // vb-8eocj.3: branch-specific evidence for Reduce nodes. Empty
    // ReduceStart initializes the accumulator then jumps to done without
    // writing the iterator slot. Exhausted ReduceNext jumps to done
    // without writing the item slot. Emit evidence for the actual
    // writes on each branch rather than unconditionally reading
    // node.output.
    if reduce_evidence(run, evidence, node, drive_state)?.is_some() {
        return Ok(());
    }

    if let Some(slot) = collect_written_slot(node) {
        match run.read_slot(slot) {
            Ok(value) => {
                let extra = collect_states.capture_state(run.run_id(), slot);
                let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
                evidence
                    .push_slot_written_with_extra(slot, *value, taint, extra)
                    .map_err(RuntimeEngineError::Core)?;
            }
            Err(_) => {
                drive_state.record_evidence_gap(slot);
                return Ok(());
            }
        }
    } else if let Some(slot) = node.output {
        match run.read_slot(slot) {
            Ok(value) => {
                let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
                evidence
                    .push_slot_written_with_taint(slot, *value, taint)
                    .map_err(RuntimeEngineError::Core)?;
            }
            Err(_) => {
                drive_state.record_evidence_gap(slot);
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Records SlotWritten evidence for a single slot, or records an
/// evidence gap if the slot cannot be read. Does not propagate read
/// errors; gaps are tracked on the drive state.
fn emit_slot(
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

/// Returns Some(()) when Reduce branch-specific evidence was emitted or
/// the branch legitimately produces no slot writes. Returns None when
/// the node is not a Reduce control node and the caller should fall
/// through to generic handling.
fn reduce_evidence(
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    node: &CompiledNode,
    drive_state: &mut DriveState,
) -> RuntimeEngineResult<Option<()>> {
    let (body, done) = match &node.kind {
        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => (*body, *done),
        _ => return Ok(None),
    };

    let at_done = run.pc() == done;
    let at_body = run.pc() == body;

    if at_done {
        match &node.kind {
            CompiledNodeKind::ReduceStart { accumulator, .. } => {
                // Empty iterator: accumulator was written with initial value.
                emit_slot(run, evidence, drive_state, *accumulator)?;
                Ok(Some(()))
            }
            CompiledNodeKind::ReduceNext { .. } => {
                // Exhausted: no new slot write on this branch.
                Ok(Some(()))
            }
            _ => Ok(None),
        }
    } else if at_body {
        match &node.kind {
            CompiledNodeKind::ReduceStart {
                input,
                accumulator,
                output,
                ..
            } => {
                // Non-empty: accumulator initialized, first item bound to
                // input, iterator state written to output.
                emit_slot(run, evidence, drive_state, *accumulator)?;
                emit_slot(run, evidence, drive_state, *input)?;
                if let Some(out) = *output {
                    emit_slot(run, evidence, drive_state, out)?;
                }
                Ok(Some(()))
            }
            CompiledNodeKind::ReduceNext { iterator_slot, output, .. } => {
                // Not exhausted: next item bound to output, iterator state
                // advanced in iterator_slot.
                if let Some(out) = *output {
                    emit_slot(run, evidence, drive_state, out)?;
                }
                emit_slot(run, evidence, drive_state, *iterator_slot)?;
                Ok(Some(()))
            }
            _ => Ok(None),
        }
    } else {
        // PC is neither body nor done — unexpected control flow.
        // Fall through to generic handling rather than suppressing
        // the gap.
        Ok(None)
    }
}

/// Returns the slot whose `SlotWritten` evidence should carry the
/// extra pagination state (collect family). For `CollectStart` nodes,
/// the source slot is preferred over the (possibly missing) output slot
/// so the first page still emits a collect-bearing event.
fn collect_written_slot(node: &CompiledNode) -> Option<SlotIdx> {
    match &node.kind {
        CompiledNodeKind::CollectStart { source, .. } => match node.output {
            Some(output) => Some(output),
            None => Some(*source),
        },
        CompiledNodeKind::CollectNext { collector_slot, .. }
        | CompiledNodeKind::CollectFinish { collector_slot } => Some(*collector_slot),
        _ => None,
    }
}

/// Walks every node in the workflow and returns the largest
/// `TogetherStart` branch count, or 0 when no fan-out nodes exist.
/// `BranchLimitExceeded` is a defense-in-depth guard for `> u16::MAX`
/// branches; workflow validation rejects fanout > 64 at construction
/// time so the limit cannot be reached through the public API.
pub(crate) fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    let mut max_branches: u16 = 0;
    for i in 0..plan.node_count() {
        let step = StepIdx::new(i);
        if let Some(node) = plan.node(step)
            && let CompiledNodeKind::TogetherStart { branches, .. } = &node.kind
        {
            let branch_count = u16::try_from(branches.len()).map_err(|_| {
                RuntimeEngineError::BranchLimitExceeded {
                    max: u16::MAX.into(),
                    requested: branches.len(),
                }
            })?;
            if branch_count > max_branches {
                max_branches = branch_count;
            }
        }
    }
    Ok(max_branches)
}
