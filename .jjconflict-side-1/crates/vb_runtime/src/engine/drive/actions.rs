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
    // RE-004: surface read_slot errors via `record_evidence_gap` rather
    // than silently swallowing them. The previous `if let Ok(value) =
    // run.read_slot(slot)` pattern hid both the slot evidence AND the
    // underlying read error; explicit `match` makes the gap observable
    // to the drive loop without changing the function's public
    // success/failure contract (a read failure is recorded as a gap and
    // the step continues — capacity overflows and other push_* errors
    // still propagate via `?`).
    //
    // Black-hat FINDING-003: the slot is now threaded through the gap
    // record so operators can see WHICH slot failed; the underlying
    // `CoreError` variant is still collapsed to `Err(_)` because the
    // only two callers (SlotOutOfBounds / SlotUninitialized) are
    // surfaced identically at this layer and the variant will be
    // classified at the report boundary.
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
