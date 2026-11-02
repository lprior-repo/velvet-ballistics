#![forbid(unsafe_code)]

//! MASTER §77.10 capability grants for the storage layer.
//!
//! The lifecycle itself lives in [`vb_core::capability_grant`] and the
//! dispatch checks in `vb_runtime::capability_grant`. This module owns
//! the storage-side piece: journaling a grant before it is relied on,
//! so replay reconstructs the grant from the journal instead of from
//! any caller's claim. A journaled grant is historical evidence only;
//! replay validates its structure and never authorizes a new live
//! effect from it.

use crate::{EventSeq, error::JournalError, events::JournalEvent, journal::FjallJournal};
use vb_core::RunId;
use vb_core::capability_grant::CapabilityGrant;

/// Appends a capability grant to the run journal under strict
/// durability. Callers supply the next sequence for the run.
pub fn journal_capability_grant(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
    grant: CapabilityGrant,
) -> Result<(), JournalError> {
    journal.append_strict(&JournalEvent::CapabilityGrantEvent { run, seq, grant })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::capability::Capability;
    use vb_core::capability_grant::{GrantId, GrantIdentity, ResourceScope, RevocationEpoch};
    use vb_core::ids::{ActionId, WorkflowDigest};

    fn grant() -> CapabilityGrant {
        CapabilityGrant {
            id: GrantId::new(1),
            issuer: GrantIdentity::new("operator".to_string()).unwrap(),
            subject: GrantIdentity::new("executor".to_string()).unwrap(),
            artifact_digest: WorkflowDigest::from_bytes([7; 32]),
            run: RunId::new(3),
            action: ActionId::new(9),
            capabilities: Box::new([Capability::new("network.github".into(), ActionId::new(9))]),
            resources: ResourceScope {
                patterns: Box::new(["repo-a".into()]),
            },
            issued_at_ms: 100,
            expires_at_ms: 1_000,
            delegable: false,
            revocation_epoch: RevocationEpoch::new(2),
            break_glass: false,
            approval_receipt_digest: None,
            signature: None,
        }
    }

    #[test]
    fn grant_event_wire_round_trip() {
        let grant = grant();
        let event = JournalEvent::CapabilityGrantEvent {
            run: grant.run,
            seq: EventSeq::new(1),
            grant: grant.clone(),
        };
        assert_eq!(event.record_kind(), crate::RecordKind::CapabilityGrant);
        assert_eq!(event.seq(), EventSeq::new(1));
        let encoded = postcard::to_allocvec(&event).unwrap();
        let decoded: JournalEvent = postcard::from_bytes(&encoded).unwrap();
        assert_eq!(decoded, event);
    }

    #[test]
    fn grant_event_rejects_outer_run_mismatch() {
        let grant = grant();
        let event = JournalEvent::CapabilityGrantEvent {
            run: RunId::new(99),
            seq: EventSeq::new(1),
            grant,
        };
        assert!(postcard::to_allocvec(&event).is_err());
    }
}
