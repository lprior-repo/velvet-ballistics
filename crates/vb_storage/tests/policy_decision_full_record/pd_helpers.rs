#![forbid(unsafe_code)]

use vb_core::action::{compute_action_idempotency_key, ActionTicket};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest, PolicyLayer,
};
use vb_core::value::Taint;

/// Build an ActionTicket with all 8 fields distinct and non-trivial.
/// The idempotency key is computed canonically from run/run_attempt/seq/action.
pub(crate) fn make_ticket(run: u64, step: u16, seq: u64, action: u16) -> ActionTicket {
    let run_id = RunId::new(run);
    let step_id = StepIdx::new(step);
    let seq_id = SeqNo::new(seq);
    let action_id = ActionId::new(action);
    let key = compute_action_idempotency_key(run_id, 2, seq_id, action_id);
    ActionTicket {
        run: run_id,
        step: step_id,
        seq: seq_id,
        action: action_id,
        run_attempt: 2,
        attempt: 3,
        idempotency_key: key,
        capacity: 10,
    }
}

/// Build a distinct non-zero PolicyDigest from a seed byte.
pub(crate) fn make_digest(seed: u8) -> PolicyDigest {
    PolicyDigest::from_bytes([seed; 32])
}

/// Build a decision record with matching outer run and ticket run.
pub(crate) fn make_decision(run: u64, outcome: DecisionCode, layer: PolicyLayer) -> DecisionRecord {
    DecisionRecord {
        ticket: make_ticket(run, 2, 7, 5),
        admission: PinnedAdmissionPolicy::Artifact(make_digest(1)),
        current_live: vb_core::policy::CurrentPolicyIdentity::Configured {
            digest: make_digest(2),
        },
        input_digest: make_digest(3),
        taint: Taint::Secret,
        outcome,
        layer,
        emergency_influence: InfluenceState::Active,
        compat_influence: InfluenceState::NotEvaluated,
    }
}
