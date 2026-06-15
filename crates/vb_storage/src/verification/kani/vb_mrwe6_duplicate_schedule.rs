#![cfg(kani)]

//! Kani harness for obl-vb-in8ib-duplicate-kani.

use crate::mrwe6_seams::{
    Mrwe6DuplicateRetryDecision, Mrwe6EventClass, mrwe6_duplicate_retry_decision_from_facts,
};

fn generated_retry_class() -> Mrwe6EventClass {
    match kani::any::<u8>() % 3 {
        0 => Mrwe6EventClass::Scheduled,
        1 => Mrwe6EventClass::Resolution,
        _ => Mrwe6EventClass::Unrelated,
    }
}

fn expected_equal_payload_decision(
    retry_class: Mrwe6EventClass,
    marker_present: bool,
) -> Mrwe6DuplicateRetryDecision {
    match (retry_class, marker_present) {
        (Mrwe6EventClass::Scheduled, true) => Mrwe6DuplicateRetryDecision::IdempotentEqualRetry,
        (Mrwe6EventClass::Scheduled, false) => {
            Mrwe6DuplicateRetryDecision::MissingExpectedIndexState
        }
        (Mrwe6EventClass::Resolution | Mrwe6EventClass::Unrelated, _) => {
            Mrwe6DuplicateRetryDecision::UnsupportedDuplicateClassRejected
        }
    }
}

fn unsupported_never_idempotent(
    retry_class: Mrwe6EventClass,
    decision: Mrwe6DuplicateRetryDecision,
) -> bool {
    matches!(retry_class, Mrwe6EventClass::Scheduled)
        || decision != Mrwe6DuplicateRetryDecision::IdempotentEqualRetry
}

#[kani::proof]
fn vb_mrwe6_duplicate_arbitrary_facts() {
    let retry_class = generated_retry_class();
    let equal_payload = kani::any::<bool>();
    let marker_present = kani::any::<bool>();
    let decision =
        mrwe6_duplicate_retry_decision_from_facts(equal_payload, retry_class, marker_present);

    if equal_payload {
        kani::assert_eq!(decision,
            expected_equal_payload_decision(retry_class, marker_present));
    } else {
        kani::assert_eq!(decision,
            Mrwe6DuplicateRetryDecision::DivergentDuplicateConflict);
    }
    kani::assert(unsupported_never_idempotent(retry_class, decision));
}
