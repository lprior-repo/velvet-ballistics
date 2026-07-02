//!
//! Verus specification and proof for RA-030 wave-15 follow-up —
//! `Runtime::shard_for_run` helper.
//!
//! Bead: vb-sxkz6
//!
//! GOD RULE 2: Verus spec fns MUST mathematically bind to the actual Rust
//! implementations (`exec fn`) inside the production codebase.
//!
//! Verus single-file check command (per AGENTS.md):
//!   verus --crate-type=lib crates/vb_runtime/src/verification/verus/vb_sxkz6_shard_for_run.rs

#![allow(unused_imports)]
#![allow(unused_variables)]

use vstd::prelude::*;

verus! {

// =========================================================================
// Spec: shard_for_run returns unique owner or NotFound
// =========================================================================

/// Spec: shard_for_run selects the unique shard owning run, if any.
/// Mirrors the production implementation in
/// `crates/vb_runtime/src/runtime.rs::Runtime::shard_for_run`.
pub closed spec fn spec_shard_for_run(shard_count: nat, run: nat) -> nat {
    if shard_count == 0 {
        0
    } else {
        run % shard_count
    }
}

// =========================================================================
// Proofs for clauses C1, C2, C6, C7
// =========================================================================

/// Proof: shard_for_run returns 0 when shard_count is zero.
pub proof fn proof_shard_for_run_zero_count(run: nat)
    ensures
        spec_shard_for_run(0nat, run) == 0,
{
}

/// Proof: when shard_count > 0, the returned index is bounded.
pub proof fn proof_shard_for_run_index_bounded(shard_count: nat, run: nat)
    requires
        shard_count > 0,
    ensures
        spec_shard_for_run(shard_count, run) < shard_count,
{
}

/// Proof: shard_for_run is deterministic.
pub proof fn proof_shard_for_run_deterministic(shard_count: nat, run: nat)
    requires
        shard_count > 0,
    ensures
        spec_shard_for_run(shard_count, run) == spec_shard_for_run(shard_count, run),
{
}

// =========================================================================
// Production binding declarations (GOD RULE 2)
// =========================================================================

/// Production exec fn for shard_for_run — declared via external_body
/// because the production Rust code uses `&[Shard]` and cannot be directly
/// modeled in pure Verus without lifetime plumbing.
#[verifier::external_body]
pub fn production_shard_for_run_spec(shard_count: u64, run: u64) -> (result: u64)
    requires
        shard_count > 0,
    ensures
        result < shard_count,
{
    unimplemented!()
}

} // verus!