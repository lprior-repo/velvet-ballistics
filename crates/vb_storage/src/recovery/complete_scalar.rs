//! Complete bound scalar traces replay actual activations, including repeated Ask boundaries.
use super::{
    RecoveredSlotEntry, RecoveredStepEntry, RecoveredStepState as S, RecoveryError,
    RecoveryFrameSeed, RecoveryFrameSeedProduct, RecoveryResult, UnsupportedRecoveryState,
};
use crate::{JournalEvent as E, RunInputBinding, RunPolicyPin};
use vb_core::{CompiledNodeKind as K, CompiledWorkflow, RunFrame, SlotValue, StepIdx, StepState};

mod driver;
mod steps;

/// A complete artifact-guided replay; no caller can construct its live frame.
/// This proves trace completeness, not host authorization: cold activation must
/// still validate the accepted artifact, capabilities and retained input receipt.
pub struct CompleteScalarReplay {
    frame: RunFrame,
    seed: RecoveryFrameSeedProduct,
}

impl CompleteScalarReplay {
    /// Existing typed recovery classification for this replay.
    pub fn seed(&self) -> &RecoveryFrameSeedProduct {
        &self.seed
    }
    /// Actual replayed execution count, state and values, rather than a zero-count frame.
    pub fn into_frame(self) -> RunFrame {
        self.frame
    }
}

pub(super) fn invalid(step: StepIdx, reason: impl Into<String>) -> RecoveryError {
    RecoveryError::ReplayDivergence {
        step,
        detail: reason.into(),
    }
}

/// Returns a proof only for the bounded scalar contract; legacy/ineligible traces
/// retain the existing replay path. Eligible but incomplete traces are rejected.
pub fn replay_complete_scalar(
    events: &[E],
    plan: &CompiledWorkflow,
) -> RecoveryResult<Option<CompleteScalarReplay>> {
    if !eligible(events, plan)? {
        return Ok(None);
    }
    let frame = driver::replay(events, plan)?;
    let summary = super::replay::summarize_recovery_events(events)?.summary();
    let mut steps = Vec::new();
    for raw in 0..plan.node_count() {
        let step = StepIdx::new(raw);
        let state = match frame
            .step_state(step)
            .map_err(|e| invalid(step, e.to_string()))?
        {
            StepState::Pending => continue,
            StepState::Running => S::Running,
            StepState::Succeeded => S::Succeeded,
            StepState::Failed => S::Failed,
            StepState::Asking => S::Asking,
            _ => return Err(invalid(step, "unknown scalar replay state")),
        };
        steps.push(RecoveredStepEntry { step, state });
    }
    let slots = frame
        .initialized_slots()
        .map_err(|e| invalid(frame.pc(), e.to_string()))?
        .into_iter()
        .map(|(slot, value, taint)| RecoveredSlotEntry { slot, value, taint })
        .collect();
    let seed = RecoveryFrameSeedProduct::from_seed(RecoveryFrameSeed {
        summary,
        first_step: plan.entry(),
        step_count: plan.node_count(),
        slot_count: plan.slot_count(),
        pc: frame.pc(),
        steps,
        slots,
        pending_actions: Vec::new(),
        unsupported: UnsupportedRecoveryState::SUPPORTED,
    });
    Ok(Some(CompleteScalarReplay { frame, seed }))
}

fn eligible(events: &[E], plan: &CompiledWorkflow) -> RecoveryResult<bool> {
    if !plan.input_slots().is_empty()
        || !events
            .iter()
            .any(|event| matches!(event, E::StepStarted { .. }))
    {
        return Ok(false);
    }
    if events.iter().any(|event| {
        matches!(
            event,
            E::RetryCheckpoint { .. } | E::RetryTimerScheduled { .. }
        )
    }) {
        return Ok(false);
    }
    let empty = crate::inputs::compute_input_digest(&[])
        .map_err(|e| invalid(plan.entry(), e.to_string()))?;
    let bound = events.iter().any(|event| match event {
        E::RunAdmission {
            input_digest: RunInputBinding::Digest(input),
            policy_pin: RunPolicyPin::Pinned(_),
            ..
        }
        | E::RunAdmissionBudget {
            input_digest: RunInputBinding::Digest(input),
            policy_pin: RunPolicyPin::Pinned(_),
            ..
        } => *input == empty,
        _ => false,
    });
    if !bound {
        return Ok(false);
    }
    let mut repeated = false;
    for raw in 0..plan.node_count() {
        let node = plan
            .node(StepIdx::new(raw))
            .ok_or_else(|| invalid(StepIdx::new(raw), "compiled node missing"))?;
        if node.routing.error.error_slot().is_some() {
            return Ok(false);
        }
        match node.kind {
            K::RepeatStart { .. }
            | K::RepeatAttempt { .. }
            | K::RepeatCheck { .. }
            | K::RepeatFinish { .. } => repeated = true,
            K::Nop
            | K::SetConst { .. }
            | K::Copy { .. }
            | K::EvalExpr { .. }
            | K::Finish { .. }
            | K::Jump { .. }
            | K::Choose { .. }
            | K::ChooseSlot { .. }
            | K::AskResume { .. }
            | K::Ask {
                timeout_slot: None, ..
            } => {}
            _ => return Ok(false),
        }
    }
    // Composite values require retained arena/checkpoint authority, never this scalar path.
    for event in events {
        if let E::SlotWrittenEvent {
            value: Some(bytes), ..
        } = event
        {
            let value: SlotValue = postcard::from_bytes(bytes)
                .map_err(|_| invalid(plan.entry(), "invalid scalar capture"))?;
            if matches!(
                value,
                SlotValue::List(_) | SlotValue::Object(_) | SlotValue::Blob(_)
            ) {
                return Ok(false);
            }
        }
    }
    if repeated
        && events
            .iter()
            .any(|event| event.attempt().is_some_and(|attempt| attempt != 1))
    {
        return Err(invalid(plan.entry(), "unproven bound run attempt"));
    }
    Ok(repeated)
}
