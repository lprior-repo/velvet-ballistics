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

#[path = "reduce_evidence.rs"]
mod reduce_evidence;

#[path = "loop_control_evidence.rs"]
mod loop_control_evidence;

pub(super) fn emit_error_slot_evidence(
    plan: &CompiledWorkflow,
    run: &RunFrame,
    evidence: &mut EvidenceCollector,
    node: &CompiledNode,
) -> RuntimeEngineResult<()> {
    if let Some(slot) = plan.resolved_error_slot(node) {
        let value = *run.read_slot(slot).map_err(RuntimeEngineError::Core)?;
        let taint = run.read_taint(slot).map_err(RuntimeEngineError::Core)?;
        evidence
            .push_slot_written_with_taint(slot, value, taint)
            .map_err(RuntimeEngineError::Core)?;
    }
    Ok(())
}

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
    // Host completions own these writes. A later suspension can still see
    // an initialized output from the preceding loop iteration.
    if matches!(
        node.kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    ) {
        return Ok(());
    }
    // vb-8eocj.3: branch-specific evidence for Reduce nodes. Empty
    // ReduceStart initializes the accumulator then jumps to done without
    // writing the iterator slot. Exhausted ReduceNext jumps to done
    // without writing the item slot. Emit evidence for the actual
    // writes on each branch rather than unconditionally reading
    // node.output.
    if reduce_evidence::reduce_evidence(run, evidence, node, drive_state)?.is_some() {
        return Ok(());
    }

    if loop_control_evidence::loop_control_evidence(run, evidence, node, drive_state)?.is_some() {
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
    } else if let Some(slot) = node.output.slot() {
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
        CompiledNodeKind::CollectStart { source, .. } => match node.output.slot() {
            Some(output) => Some(output),
            None => Some(*source),
        },
        CompiledNodeKind::CollectNext { collector_slot, .. } => Some(*collector_slot),
        _ => None,
    }
}

/// Uses the constructor's shared active-branch bound. Representation overflow
/// retains the runtime BranchLimitExceeded defense for unchecked artifacts.
pub(crate) fn compute_max_parallel_in_flight(plan: &CompiledWorkflow) -> RuntimeEngineResult<u16> {
    vb_core::budget::compute_max_parallel_in_flight(
        (0..plan.node_count()).filter_map(|index| plan.node(StepIdx::new(index))),
    )
    .map_err(|error| match error {
        vb_core::WorkflowError::StepCountOverflow { actual } => {
            RuntimeEngineError::BranchLimitExceeded {
                max: usize::from(u16::MAX),
                requested: usize::try_from(actual).map_or(usize::MAX, core::convert::identity),
            }
        }
        _ => RuntimeEngineError::Core(vb_core::EngineError::InvalidCompiledWorkflow {
            reason: "invalid parallel branch budget",
        }),
    })
}
