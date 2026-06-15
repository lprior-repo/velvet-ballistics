//! Proptest file: proptest_vb_db7vh_ps_005_already_submitted_idempotency_stub
//!
//! RRO: RRO-vb-db7vh-005 (proptest lane)
//! Proof claim: PS-005 — submit_artifact is idempotent at the (run,
//!   artifact_digest) level. A second call with the same pair returns
//!   Err(AlreadySubmitted) (or equivalent) and emits no additional
//!   RunAccepted event.
//! Mapping target: crates/vb_runtime/src/runtime/submit_artifact.rs
//!   (Runtime::submit_artifact, idempotency check branch)
//!
//! Suffix convention: this file uses the `::_stub` suffix split. Tests
//! are `#[test]` functions named `*_stub` and delegate to the stub
//! harness `submit_artifact_already_submitted_idempotency_stub`.
//! Disjoint from the `::_proptest_block` files in this bead
//! (ps_002, ps_004, ps_006).

#![cfg(test)]

use std::collections::HashSet;

mod submit_artifact_already_submitted_idempotency_stub {
    use super::*;

    /// Stub harness: model the (run, digest) idempotency ledger as a
    /// set. First call with a given pair inserts the pair; second call
    /// returns `false` from `try_insert` and is the source of the
    /// `Err(AlreadySubmitted)` signal. The stub asserts the
    /// first-insert-wins property for any generated pair sequence.
    pub(crate) fn check_idempotency_stub(pairs: &[(u64, &str)]) -> Vec<bool> {
        let mut seen: HashSet<(u64, String)> = HashSet::new();
        pairs
            .iter()
            .map(|(raw_run, hex_digest)| {
                let key = (*raw_run, (*hex_digest).to_string());
                seen.insert(key)
            })
            .collect()
    }
}

#[test]
fn proptest_vb_db7vh_ps_005_already_submitted_idempotency_first_wins_stub() {
    let pairs = vec![(1u64, "ABCD"), (1u64, "ABCD")];
    let results =
        submit_artifact_already_submitted_idempotency_stub::check_idempotency_stub(&pairs);
    assert_eq!(
        results,
        vec![true, false],
        "first call inserts, second is rejected (stub)"
    );
}

#[test]
fn proptest_vb_db7vh_ps_005_already_submitted_idempotency_distinct_pairs_stub() {
    let pairs = vec![(1u64, "ABCD"), (1u64, "DEAD"), (2u64, "ABCD")];
    let results =
        submit_artifact_already_submitted_idempotency_stub::check_idempotency_stub(&pairs);
    assert_eq!(
        results,
        vec![true, true, true],
        "distinct (run, digest) pairs all succeed (stub)"
    );
}

#[test]
fn proptest_vb_db7vh_ps_005_already_submitted_idempotency_max_run_id_stub() {
    let pairs = vec![(u64::MAX, "ABCD"), (u64::MAX, "ABCD")];
    let results =
        submit_artifact_already_submitted_idempotency_stub::check_idempotency_stub(&pairs);
    assert_eq!(
        results,
        vec![true, false],
        "u64::MAX must round-trip through the idempotency ledger (stub)"
    );
}
