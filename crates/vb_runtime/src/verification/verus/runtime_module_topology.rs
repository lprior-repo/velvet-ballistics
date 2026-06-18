//! Verus specification and proof for runtime module topology invariants — vb-evkno.
//!
//! Production binding:
//! - `spec_shard_index` → mirrors `Runtime::shard_index` in runtime/mod.rs
//! - `spec_submit_direct_admitted` → submit_direct admission logic
//!
//! NOTE: This file was merged into runtime_facade_api.rs which contains
//! the same `spec_shard_index` spec. This file proves additional topology
//! invariants (submit_direct, inspect_run) that are not covered by
//! runtime_facade_api.rs.

use vstd::prelude::*;

verus! {

    // ===========================================================================
    // Spec: shard_index — mirrors Runtime::shard_index
    // ===========================================================================

    pub closed spec fn spec_shard_index(run_id: u64, shard_count: u64) -> u64 {
        if shard_count == 0 { 0 } else { run_id % shard_count }
    }

    // ===========================================================================
    // Spec: submit_direct admission (topology-level)
    //
    // Production returns Ok(()) when shard_for succeeds and enqueue succeeds.
    // The spec captures the admission decision: run_id routed to a valid shard.
    // ===========================================================================

    pub closed spec fn spec_submit_direct_admitted(run_id: u64, shard_count: u64) -> bool {
        if shard_count == 0 { false } else { spec_shard_index(run_id, shard_count) < shard_count }
    }

    // ===========================================================================
    // Spec: inspect_run admission (topology-level)
    //
    // Inspect uses the same shard routing as submit_direct.
    // ===========================================================================

    pub closed spec fn spec_inspect_run_admitted(run_id: u64, shard_count: u64) -> bool {
        spec_submit_direct_admitted(run_id, shard_count)
    }

    // ===========================================================================
    // Proof: submit_direct admitted when shards exist
    // ===========================================================================

    pub proof fn proof_submit_admitted_when_shards_exist(run_id: u64, shard_count: u64)
        requires
            shard_count > 0,
        ensures
            spec_submit_direct_admitted(run_id, shard_count),
    {
        assert(spec_submit_direct_admitted(run_id, shard_count)) by {
            assert(spec_shard_index(run_id, shard_count) < shard_count) by (compute);
        };
    }

    // ===========================================================================
    // Proof: submit_direct rejected when no shards
    // ===========================================================================

    pub proof fn proof_submit_rejected_no_shards(run_id: u64)
        ensures
            !spec_submit_direct_admitted(run_id, 0),
    {
        assert(!spec_submit_direct_admitted(run_id, 0)) by (compute);
    }

    // ===========================================================================
    // Proof: inspect_run admitted when submit_direct admitted
    // ===========================================================================

    pub proof fn proof_inspect_admitted_when_submit_admitted(run_id: u64, shard_count: u64)
        requires
            shard_count > 0,
        ensures
            spec_inspect_run_admitted(run_id, shard_count),
    {
        assert(spec_inspect_run_admitted(run_id, shard_count)) by {
            assert(spec_submit_direct_admitted(run_id, shard_count)) by (compute);
        };
    }

    // ===========================================================================
    // Theorem: Runtime topology invariant is sound
    //
    // For any valid configuration (shard_count > 0), all topology operations
    // have well-defined shard assignments that are bounded by shard_count.
    // ===========================================================================

    pub proof fn theorem_runtime_topology_soundness(run_id: u64, shard_count: u64)
        requires
            shard_count > 0,
        ensures
            spec_shard_index(run_id, shard_count) < shard_count
                && spec_submit_direct_admitted(run_id, shard_count)
                && spec_inspect_run_admitted(run_id, shard_count),
    {
        assert(spec_shard_index(run_id, shard_count) < shard_count) by (compute);
        assert(spec_submit_direct_admitted(run_id, shard_count)) by (compute);
        assert(spec_inspect_run_admitted(run_id, shard_count)) by (compute);
    }

} // verus!
