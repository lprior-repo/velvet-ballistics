#![forbid(unsafe_code)]
//! Duplicate retry decision logic.

use crate::events::JournalEvent;
use crate::journal::append::intent::{Mrwe6EventClass, mrwe6_event_class};
use crate::journal::append::mrwe6_kernel::Mrwe6DuplicateRetryDecision;

#[cfg(kani)]
#[allow(dead_code)]
pub(crate) type VerificationDuplicateRetryDecision = Mrwe6DuplicateRetryDecision;

#[cfg(kani)]
#[allow(dead_code)]
pub(crate) fn verification_duplicate_retry_decision(
    existing: &JournalEvent,
    retry: &JournalEvent,
    index_marker_present: bool,
) -> VerificationDuplicateRetryDecision {
    mrwe6_duplicate_retry_decision(existing, retry, index_marker_present)
}

#[must_use]
pub fn mrwe6_duplicate_retry_decision(
    existing: &JournalEvent,
    retry: &JournalEvent,
    index_marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    mrwe6_duplicate_retry_decision_from_facts(
        existing == retry,
        mrwe6_event_class(retry),
        index_marker_present,
    )
}

#[must_use]
pub fn mrwe6_duplicate_retry_decision_from_facts(
    equal_payload: bool,
    retry_class: Mrwe6EventClass,
    index_marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    crate::journal::append::mrwe6_kernel::duplicate_retry_decision_from_facts(
        equal_payload,
        retry_class,
        index_marker_present,
    )
}

pub fn mrwe6_idempotent_duplicate_retry_from_facts(
    equal_payload: bool,
    retry_class: Mrwe6EventClass,
    index_marker_present: bool,
) -> Result<Mrwe6DuplicateRetryDecision, crate::journal::append::intent::Mrwe6SeamError> {
    let decision =
        mrwe6_duplicate_retry_decision_from_facts(equal_payload, retry_class, index_marker_present);
    if decision == Mrwe6DuplicateRetryDecision::IdempotentEqualRetry {
        Ok(decision)
    } else {
        Err(crate::journal::append::intent::Mrwe6SeamError::DuplicateRetryNotIdempotent)
    }
}
