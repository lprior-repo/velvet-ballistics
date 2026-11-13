//! Test-only grants for local register regression tests.

use super::RegisterGrant;
use vb_core::action_pack::{ActionPackDigest, CapabilityScopeSpec};
use vb_core::{
    ActionId, ActionTicket, RunId, SeqNo, StepIdx, WorkflowDigest,
    action::compute_action_idempotency_key,
};

pub(super) fn scratch_grant() -> RegisterGrant {
    let run = RunId::new(1);
    let seq = SeqNo::new(1);
    let action = ActionId::new(0);
    let ticket = ActionTicket {
        run,
        step: StepIdx::new(1),
        seq,
        action,
        run_attempt: 1,
        attempt: 1,
        idempotency_key: compute_action_idempotency_key(run, 1, seq, action),
        capacity: 2,
        fencing_token: 0,
    };
    RegisterGrant {
        artifact: WorkflowDigest::from_bytes([1; 32]),
        pack: ActionPackDigest::from_digest(WorkflowDigest::from_bytes([1; 32])),
        ticket,
        scope: CapabilityScopeSpec {
            namespace: 1,
            key: 1,
        },
        timeout_ms: 60_000,
    }
}
