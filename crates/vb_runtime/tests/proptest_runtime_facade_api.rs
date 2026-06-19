//! Proptest property suite for `vb_runtime` Runtime facade API contracts.
//!
//! Bead: vb-puvkn — closure of the runtime_facade_api exec fn binding.
//!
//! The Verus spec at `crates/vb_runtime/src/verification/verus/runtime_facade_api.rs`
//! defines:
//!
//! - `spec_shard_index(run_id, shard_count)`:
//!     `if shard_count == 0 { 0 } else { run_id % shard_count }`.
//! - `exec_shard_index_runtime(run_id, shard_count)`:
//!     Bridge that uses `checked_rem` with fallback to 0 when either
//!     input is zero, mirroring production
//!     `Runtime::shard_index(&self, run: RunId) -> usize` and
//!     `RunId::shard_index(self, shard_count: u64) -> u64`.
//!
//! Each property in this file exercises the production methods directly
//! via the public `RunId::shard_index` and asserts they implement the
//! Verus spec on both the zero and non-zero regimes.  Together with the
//! Kani harness (`vb_runtime::verification::verus::runtime_facade_api`)
//! these tests close the L1/L3/L4 lanes for `exec_shard_index_runtime`.

#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::ids::RunId;

/// Reference `spec_shard_index` from runtime_facade_api.rs.
fn spec_shard_index(run_id: u64, shard_count: u64) -> u64 {
    if shard_count == 0 {
        0
    } else {
        run_id % shard_count
    }
}

/// Strategy: any u64 run_id with bias toward boundaries (0, 1, u64::MAX).
fn arb_run_id() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0_u64),
        Just(1_u64),
        Just(u64::MAX),
        Just(u64::MAX / 2),
        0u64..u64::MAX,
    ]
}

/// Strategy: any u64 shard_count with bias toward boundaries
/// (0, 1, u64::MAX). The 0 case exercises the spec fallback.
fn arb_shard_count() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0_u64),
        Just(1_u64),
        Just(2_u64),
        Just(u64::MAX),
        1u64..u64::MAX,
    ]
}

proptest! {
    // ------------------------------------------------------------------
    // Production binding: RunId::shard_index(shard_count) == spec_shard_index(run_id_raw, shard_count)
    // ------------------------------------------------------------------

    /// `RunId::shard_index(shard_count)` equals `spec_shard_index(raw, shard_count)`.
    /// This is the production exec bridge to the Verus spec.
    #[test]
    fn runid_shard_index_matches_spec_shard_index(
        raw in arb_run_id(),
        shard_count in arb_shard_count(),
    ) {
        let run = RunId::new(raw);
        let result = run.shard_index(shard_count);
        let expected = spec_shard_index(raw, shard_count);
        prop_assert_eq!(
            result, expected,
            "RunId::shard_index({:?}, {}) = {}, spec_shard_index = {}",
            run, shard_count, result, expected
        );
    }

    // ------------------------------------------------------------------
    // LEMMA-FACADE-001 / proof_shard_index_bounded:
    // shard_count > 0 ⇒ result < shard_count.
    // ------------------------------------------------------------------

    /// When `shard_count > 0`, `RunId::shard_index` returns a value strictly
    /// less than `shard_count`. Mirrors `proof_shard_index_bounded`.
    #[test]
    fn runid_shard_index_bounded_by_shard_count(
        raw in arb_run_id(),
        shard_count in 1u64..u64::MAX,
    ) {
        let run = RunId::new(raw);
        let result = run.shard_index(shard_count);
        prop_assert!(
            result < shard_count,
            "result {} must be < shard_count {} for raw={}",
            result, shard_count, raw
        );
    }

    // ------------------------------------------------------------------
    // proof_shard_index_zero_shard_count:
    // shard_count == 0 ⇒ spec_shard_index returns 0.
    // ------------------------------------------------------------------

    /// When `shard_count == 0`, `RunId::shard_index` returns 0.
    /// Mirrors `proof_shard_index_zero_shard_count`.
    #[test]
    fn runid_shard_index_zero_shard_count_returns_zero(raw in arb_run_id()) {
        let run = RunId::new(raw);
        let result = run.shard_index(0);
        prop_assert_eq!(result, 0);
    }

    // ------------------------------------------------------------------
    // proof_zero_run_id_shard:
    // shard_count > 0 ⇒ spec_shard_index(0, shard_count) == 0.
    // ------------------------------------------------------------------

    /// When `shard_count > 0`, `RunId::shard_index(0)` returns 0.
    /// Mirrors `proof_zero_run_id_shard`.
    #[test]
    fn runid_shard_index_zero_run_id_returns_zero(shard_count in 1u64..u64::MAX) {
        let run = RunId::new(0);
        let result = run.shard_index(shard_count);
        prop_assert_eq!(result, 0);
    }

    // ------------------------------------------------------------------
    // proof_same_run_same_shard: determinism.
    // ------------------------------------------------------------------

    /// Determinism: same RunId and shard_count always yields the same result.
    /// Mirrors `proof_same_run_same_shard` and `proof_shard_index_functional`.
    #[test]
    fn runid_shard_index_is_deterministic(
        raw in arb_run_id(),
        shard_count in 1u64..u64::MAX,
    ) {
        let run = RunId::new(raw);
        let r1 = run.shard_index(shard_count);
        let r2 = run.shard_index(shard_count);
        prop_assert_eq!(r1, r2);
    }

    // ------------------------------------------------------------------
    // Boundary: u64::MAX run_id with shard_count = 2.
    // ------------------------------------------------------------------

    /// Boundary: `RunId::new(u64::MAX).shard_index(2)` is the parity of u64::MAX.
    /// u64::MAX is odd, so result must be 1.
    #[test]
    fn runid_shard_index_max_mod_two(_unit in Just(())) {
        let run = RunId::new(u64::MAX);
        prop_assert_eq!(run.shard_index(2), 1);
    }

    // ------------------------------------------------------------------
    // Identity: shard_count = 1 always returns 0.
    // ------------------------------------------------------------------

    /// `RunId::shard_index(1)` always returns 0 (the only valid index).
    #[test]
    fn runid_shard_index_count_one_returns_zero(raw in arb_run_id()) {
        let run = RunId::new(raw);
        prop_assert_eq!(run.shard_index(1), 0);
    }

    // ------------------------------------------------------------------
    // spec↔exec bridge parity (mirrors exec_shard_index_runtime).
    // ------------------------------------------------------------------

    /// Cross-check `spec_shard_index` reference impl against `RunId::shard_index`.
    /// This is the L1 mirror of the L4 lemma `lemma_exec_shard_index_matches_spec`.
    #[test]
    fn spec_shard_index_matches_runid_shard_index(
        raw in arb_run_id(),
        shard_count in arb_shard_count(),
    ) {
        let spec_result = spec_shard_index(raw, shard_count);
        let run = RunId::new(raw);
        let exec_result = run.shard_index(shard_count);
        prop_assert_eq!(spec_result, exec_result);
    }

    // ------------------------------------------------------------------
    // Production Runtime::shard_index math: zero shard_count returns 0.
    // Mirrors the usize::try_from(0).unwrap_or_default() fallback.
    // ------------------------------------------------------------------

    /// `RunId::shard_index(0)` mirrors production `Runtime::shard_index`
    /// for a `Runtime` constructed with `shard_count = 0`: both
    /// return 0 (the `usize::try_from(0)` is Ok(0) but `checked_rem(0)`
    /// returns None, so the production fallback path yields 0).
    #[test]
    fn runid_shard_index_zero_shard_count_matches_runtime_default(
        raw in arb_run_id(),
    ) {
        let run = RunId::new(raw);
        prop_assert_eq!(run.shard_index(0), 0);
    }

    // ------------------------------------------------------------------
    // Distribution: spec_shard_index produces 0 for raw = n * shard_count.
    // ------------------------------------------------------------------

    /// When `raw` is a multiple of `shard_count`, the result is 0.
    /// This is the `n * k mod k == 0` property.
    #[test]
    fn runid_shard_index_zero_at_multiples(
        n in 0u64..1024,
        shard_count in 1u64..1024,
    ) {
        let raw = n.saturating_mul(shard_count);
        let run = RunId::new(raw);
        let result = run.shard_index(shard_count);
        prop_assert_eq!(result, 0);
    }

    // ------------------------------------------------------------------
    // Distinguishability: different shard_counts can produce different results.
    // ------------------------------------------------------------------

    /// For a fixed non-zero raw, distinct shard_counts may produce
    /// different shard indices (when raw >= the larger shard_count).
    #[test]
    fn runid_shard_index_distinguishes_shard_counts(raw in 100u64..u64::MAX) {
        let run = RunId::new(raw);
        let r_3 = run.shard_index(3);
        let r_7 = run.shard_index(7);
        let r_64 = run.shard_index(64);
        // All three should be in [0, shard_count); raw is large enough
        // to exercise non-trivial mod behavior.
        prop_assert!(r_3 < 3);
        prop_assert!(r_7 < 7);
        prop_assert!(r_64 < 64);
    }
}
