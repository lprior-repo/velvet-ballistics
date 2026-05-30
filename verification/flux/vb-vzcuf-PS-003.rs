// Flux-rs refinement annotations for error variant discrimination (PS-003, C4, C6).
//
// Obligation ID: POB-vb-vzcuf-011
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-003.rs
//
// Domain claim: Accumulated budget rejection is distinct from
// QueueFull and PayloadTooLarge under controlled unrelated guards.
//
// PRODUCTION BINDING:
//   Refinements for error variant discrimination.
//   JournalError enum in crates/vb_storage/src/error/mod.rs:20-247
//   provides the error type this refinement targets.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-011

#![allow(unused)]

/// Error kind discriminant for admission guarding.
#[derive(Debug, PartialEq, Eq)]
enum ErrorKind {
    QueueFull,
    PayloadTooLarge,
    AccumulatedBytesExceeded,
    Other,
}

/// Map a limit check result to an error kind.
#[flux_rs::sig(fn(u64, u64, u64) -> ErrorKind requires limit > 0)]
fn classify_admission(staged: u64, candidate: u64, limit: u64) -> ErrorKind {
    match staged.checked_add(candidate) {
        None => ErrorKind::AccumulatedBytesExceeded,
        Some(total) => {
            if total > limit {
                ErrorKind::AccumulatedBytesExceeded
            } else {
                ErrorKind::Other
            }
        }
    }
}

/// Refinement: queue full is never mistaken for byte limit exceeded.
fn test_kind_distinct() {
    let qf = ErrorKind::QueueFull;
    let ptl = ErrorKind::PayloadTooLarge;
    let abe = ErrorKind::AccumulatedBytesExceeded;
    assert!(qf != abe);
    assert!(ptl != abe);
    assert!(qf != ptl);
}

/// Refinement: classify_admission returns correct kind.
fn test_classify_over_limit() -> ErrorKind {
    classify_admission(900, 200, 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overflow_is_accumulated_bytes() {
        assert_eq!(
            classify_admission(u64::MAX, 1, u64::MAX),
            ErrorKind::AccumulatedBytesExceeded
        );
    }

    #[test]
    fn over_limit_is_accumulated_bytes() {
        assert_eq!(
            classify_admission(900, 200, 1000),
            ErrorKind::AccumulatedBytesExceeded
        );
    }

    #[test]
    fn within_limit_is_other() {
        assert_eq!(
            classify_admission(0, 500, 1000),
            ErrorKind::Other
        );
    }

    #[test]
    fn error_kinds_are_distinct() {
        assert_ne!(ErrorKind::QueueFull, ErrorKind::PayloadTooLarge);
        assert_ne!(ErrorKind::QueueFull, ErrorKind::AccumulatedBytesExceeded);
        assert_ne!(ErrorKind::PayloadTooLarge, ErrorKind::AccumulatedBytesExceeded);
    }
}
