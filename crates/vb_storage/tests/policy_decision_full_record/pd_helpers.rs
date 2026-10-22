#![forbid(unsafe_code)]

use vb_core::action::ActionTicket;
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};
use vb_core::policy::{
    DecisionCode, DecisionRecord, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::value::Taint;

/// Build an ActionTicket with all 8 fields distinct and non-trivial.
pub(crate) fn make_ticket(run: u64, step: u16, seq: u64, action: u16) -> ActionTicket {
    ActionTicket {
        run: RunId::new(run),
        step: StepIdx::new(step),
        seq: SeqNo::new(seq),
        action: ActionId::new(action),
        run_attempt: 1,
        attempt: 1,
        idempotency_key: 0,
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