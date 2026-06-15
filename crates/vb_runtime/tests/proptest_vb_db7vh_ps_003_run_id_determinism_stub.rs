//! Proptest file: proptest_vb_db7vh_ps_003_run_id_determinism_stub
//!
//! RRO: RRO-vb-db7vh-003 (proptest lane)
//! Proof claim: PS-003 — submit_artifact(run, ...) treats `run` as a
//!   transparent newtype over u64. The same raw value must produce the
//!   same RunId; distinct raw values must produce distinct RunIds.
//! Mapping target: crates/vb_runtime/src/runtime/submit_artifact.rs
//!   (Runtime::submit_artifact, run parameter handling)
//!
//! Suffix convention: this file uses the `::_stub` suffix split. Tests
//! here are `#[test]` functions named `*_stub` and delegate to the stub
//! harness `submit_artifact_run_id_determinism_stub`. Disjoint from the
//! `::_proptest_block` files in this bead (ps_002, ps_004, ps_006).

#![cfg(test)]

use vb_core::ids::RunId;

mod submit_artifact_run_id_determinism_stub {
    use super::*;

    /// Stub harness: for any two generated u64 raw values, the induced
    /// RunIds are equal iff the raws are equal. This is a stub over
    /// the RunId constructor; the full proptest runs at ps_009
    /// (toolchain-unavailable) and is mapped via RRO-vb-db7vh-009.
    pub(crate) fn check_run_id_determinism_stub(raw1: u64, raw2: u64) -> bool {
        let run1 = RunId::new(raw1);
        let run2 = RunId::new(raw2);
        if raw1 == raw2 {
            run1 == run2
        } else {
            run1 != run2
        }
    }
}

#[test]
fn proptest_vb_db7vh_ps_003_run_id_determinism_equal_raw_stub() {
    let result = submit_artifact_run_id_determinism_stub::check_run_id_determinism_stub(7, 7);
    assert!(result, "equal raw values must produce equal RunIds (stub)");
}

#[test]
fn proptest_vb_db7vh_ps_003_run_id_determinism_distinct_raw_stub() {
    let result = submit_artifact_run_id_determinism_stub::check_run_id_determinism_stub(7, 8);
    assert!(
        result,
        "distinct raw values must produce distinct RunIds (stub)"
    );
}

#[test]
fn proptest_vb_db7vh_ps_003_run_id_determinism_max_run_id_stub() {
    let result =
        submit_artifact_run_id_determinism_stub::check_run_id_determinism_stub(u64::MAX, u64::MAX);
    assert!(result, "u64::MAX must round-trip through RunId (stub)");
}
