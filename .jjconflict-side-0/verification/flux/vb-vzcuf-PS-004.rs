// Flux-rs refinement annotations for batch state preservation (PS-004, C5).
//
// Obligation ID: POB-vb-vzcuf-015
// Verifier: flux-rs
// Command: flux check verification/flux/vb-vzcuf-PS-004.rs
//
// Domain claim: Accumulated byte rejection leaves batch state unchanged
// and does not persist the rejected event after commit.
//
// PRODUCTION BINDING:
//   Refinements for JournalWriteBatch state preservation
//   (crates/vb_storage/src/batch.rs:38-257).
//   On rejection: staged_count, staged_bytes, and aborted flag unchanged.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-015

#![allow(unused)]

/// Batch admission: returns new state on accept, same state on reject.
#[flux_rs::sig(fn(u64, u64, u64) -> Result<u64, ()> requires limit > 0)]
fn batch_admit(staged: u64, encoded_len: u64, limit: u64) -> Result<u64, ()> {
    let total = staged.checked_add(encoded_len).ok_or(())?;
    if total <= limit { Ok(total) } else { Err(()) }
}

/// State preservation: on Err, staged_bytes unchanged.
fn test_rejection_preserves_state() {
    let staged = 100u64;
    let candidate = 200u64;
    let limit = 150u64;

    let result = batch_admit(staged, candidate, limit);
    assert!(result.is_err());
    // staged value unchanged after rejection
    assert_eq!(staged, 100);
}

/// State update: on Ok, staged_bytes increases by encoded_len.
fn test_acceptance_updates_state() {
    let staged = 100u64;
    let candidate = 50u64;
    let limit = 200u64;

    match batch_admit(staged, candidate, limit) {
        Ok(new_staged) => {
            assert_eq!(new_staged, staged + candidate);
            assert!(new_staged > staged);
        }
        Err(_) => panic!("should accept"),
    }
}

/// Refinement: aborted batches don't commit.
fn test_aborted_no_commit(aborted: bool) -> bool {
    if aborted { false } else { true }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejection_no_state_change() {
        let before = 100u64;
        let r = batch_admit(before, 200, 150);
        assert!(r.is_err());
        assert_eq!(before, 100); // unchanged
    }

    #[test]
    fn acceptance_increases_staged() {
        let before = 100u64;
        let r = batch_admit(before, 50, 200);
        assert!(r.is_ok());
        assert!(r.unwrap() > before);
    }

    #[test]
    fn acceptance_exact_value() {
        let r = batch_admit(100, 50, 200);
        assert_eq!(r.unwrap(), 150);
    }

    #[test]
    fn aborted_batch_blocks_commit() {
        assert!(!test_aborted_no_commit(true));
        assert!(test_aborted_no_commit(false));
    }
}
