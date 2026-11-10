#![forbid(unsafe_code)]

use super::RecoveryCannotResumeState;

/// Classification priority for the cannot-resume reason tokens.
/// The first-matching rule (highest enum variant above) wins;
/// the `None` arm of the priority scan carries through to the
/// "resumable" string fallback in [`unsupported_reason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CannotResumeClass {
    SlotValues,
    SlotTaint,
    ActionPayloads,
    PendingActions,
    PendingTimers,
    PendingAsks,
    WorkflowMissing,
    StoreMissing,
    ActionAttemptsMissing,
    AdmissionMissing,
    CollectStatesMissing,
    ActionContractsMissing,
    ActionAbiDigestsMissing,
}

impl RecoveryCannotResumeState {
    /// First-half priority scan (storage-layer + pending-boundary
    /// flags 0..6). Returns the highest-priority cannot-resume reason
    /// seen in this half, or `None` if every flag in the half is
    /// false. Pure projection over the witness.
    pub(crate) const fn priority_class_first_half(self) -> Option<CannotResumeClass> {
        if self.slot_values {
            return Some(CannotResumeClass::SlotValues);
        }
        if self.slot_taint {
            return Some(CannotResumeClass::SlotTaint);
        }
        if self.action_payloads {
            return Some(CannotResumeClass::ActionPayloads);
        }
        if self.pending_actions {
            return Some(CannotResumeClass::PendingActions);
        }
        if self.pending_timers {
            return Some(CannotResumeClass::PendingTimers);
        }
        if self.pending_asks {
            return Some(CannotResumeClass::PendingAsks);
        }
        None
    }

    /// Second-half priority scan (the seven `*_missing` full-RunState
    /// flags 6..13). Returns the highest-priority cannot-resume reason
    /// seen in this half, or `None` if every flag in the half is
    /// false. Pure projection over the witness.
    pub(crate) const fn priority_class_second_half(self) -> Option<CannotResumeClass> {
        if self.workflow_missing {
            return Some(CannotResumeClass::WorkflowMissing);
        }
        if self.store_missing {
            return Some(CannotResumeClass::StoreMissing);
        }
        if self.action_attempts_missing {
            return Some(CannotResumeClass::ActionAttemptsMissing);
        }
        if self.admission_missing {
            return Some(CannotResumeClass::AdmissionMissing);
        }
        if self.collect_states_missing {
            return Some(CannotResumeClass::CollectStatesMissing);
        }
        if self.action_contracts_missing {
            return Some(CannotResumeClass::ActionContractsMissing);
        }
        if self.action_abi_digests_missing {
            return Some(CannotResumeClass::ActionAbiDigestsMissing);
        }
        None
    }
}

/// Maps a [`CannotResumeClass`] to its canonical reason token.
/// Order matches [`CannotResumeClass`] declaration; the spec proof
/// `proof_unsupported_reason_first_match_wins` in
/// `verification/verus/recovery_verification.rs` discharges the
/// priority invariant over this mapping.
#[must_use]
pub(crate) const fn priority_reason(class: CannotResumeClass) -> &'static str {
    match class {
        CannotResumeClass::SlotValues => "slot_values",
        CannotResumeClass::SlotTaint => "slot_taint",
        CannotResumeClass::ActionPayloads => "action_payloads",
        CannotResumeClass::PendingActions => "pending_actions",
        CannotResumeClass::PendingTimers => "pending_timers",
        CannotResumeClass::PendingAsks => "pending_asks",
        CannotResumeClass::WorkflowMissing => "workflow_missing",
        CannotResumeClass::StoreMissing => "store_missing",
        CannotResumeClass::ActionAttemptsMissing => "action_attempts_missing",
        CannotResumeClass::AdmissionMissing => "admission_missing",
        CannotResumeClass::CollectStatesMissing => "collect_states_missing",
        CannotResumeClass::ActionContractsMissing => "action_contracts_missing",
        CannotResumeClass::ActionAbiDigestsMissing => "action_abi_digests_missing",
    }
}
