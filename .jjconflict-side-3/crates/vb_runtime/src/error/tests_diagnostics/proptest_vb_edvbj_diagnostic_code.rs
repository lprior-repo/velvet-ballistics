// SPDX-License-Identifier: MIT
//
// Proptest: RuntimeError::UnsupportedRuntimeJournalEventMapping diagnostic
// codes are invariant across event kinds. Every kind maps to the core
// internal-invariant code and carries no runtime code.

#![cfg(test)]

use crate::error::RuntimeError;
use proptest::prelude::*;
use vb_core::DiagnosticCode;
use vb_core::errors::CoreError;

const EVENT_KINDS: [&str; 24] = [
    "RunSubmitted",
    "RunAdmission",
    "CompatibilityDecision",
    "TenantAdmission",
    "RetryCheckpoint",
    "RunFinished",
    "RunFailed",
    "RunCancelled",
    "RunKilled",
    "ActionScheduled",
    "ActionCompleted",
    "ActionScheduledTicket",
    "ActionCompletedEnvelope",
    "ActionFailed",
    "ActionAbandoned",
    "WaitScheduled",
    "WaitResolved",
    "AskScheduled",
    "AskAnswered",
    "AskTimedOut",
    "SlotWritten",
    "StepStarted",
    "StepSucceeded",
    "Resumed",
];

fn assert_unmapped_mapping_code_is_stable(kind: usize) {
    let error = RuntimeError::UnsupportedRuntimeJournalEventMapping {
        event_kind: EVENT_KINDS[kind],
    };
    assert_eq!(error.diagnostic_code(), CoreError::INTERNAL_INVARIANT_CODE);
    assert_eq!(error.diagnostic_code(), DiagnosticCode::new(0x1309));
    assert_eq!(error.runtime_code(), None);
}

proptest! {
    #[test]
    fn unmapped_diagnostic_codes_invariant(kind in 0usize..EVENT_KINDS.len()) {
        assert_unmapped_mapping_code_is_stable(kind);
    }
}

#[test]
fn every_event_kind_maps_to_the_internal_invariant_code() {
    for kind in 0..EVENT_KINDS.len() {
        assert_unmapped_mapping_code_is_stable(kind);
    }
}
