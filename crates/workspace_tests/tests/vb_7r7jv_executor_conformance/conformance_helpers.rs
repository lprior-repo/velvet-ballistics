//! Small shared helpers for the split conformance suite.

use super::*;

pub(super) fn first_pack(
    artifact: &mut vb_storage::AcceptedArtifact,
) -> Result<&mut ActionPack, Box<dyn std::error::Error>> {
    let ArtifactEvidence::Packed(evidence) = &mut artifact.verification.evidence else {
        return Err("missing packs".into());
    };
    evidence
        .packs
        .first_mut()
        .ok_or_else(|| "missing pack".into())
}

pub(super) fn ticket(seq: u64) -> ActionTicket {
    let run = RunId::new(100);
    let seq = SeqNo::new(seq);
    let action = ActionId::new(0);
    ActionTicket {
        run,
        step: StepIdx::new(1),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: vb_core::action::compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    }
}
