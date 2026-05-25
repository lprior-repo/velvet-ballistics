#![allow(unused_imports)]

use vstd::prelude::*;

verus! {

/// VFR-R2-VERUS-005 / PRE-001.
/// Bridge model for hydrate_snapshot_tail_run_matches,
/// hydrate_snapshot_tail_seq_after_snapshot,
/// hydrate_snapshot_tail_has_evidence, hydrate_snapshot_tail_preconditions,
/// and hydrate_dimensions_positive.
pub type RunId = int;

pub open spec fn all_tail_runs_match(tail_runs: Seq<RunId>, run: RunId) -> bool {
    forall|i: int| 0 <= i < tail_runs.len() ==> tail_runs[i] == run
}

pub open spec fn all_tail_seq_after_snapshot(snapshot_seq: int, tail_seqs: Seq<int>) -> bool {
    forall|i: int| 0 <= i < tail_seqs.len() ==> tail_seqs[i] > snapshot_seq
}

pub open spec fn production_hydrate_snapshot_tail_run_matches(snapshot_run: RunId, tail_runs: Seq<RunId>, requested_run: RunId) -> bool {
    snapshot_run == requested_run && all_tail_runs_match(tail_runs, requested_run)
}

pub open spec fn production_hydrate_snapshot_tail_seq_after_snapshot(snapshot_seq: int, tail_seqs: Seq<int>) -> bool {
    all_tail_seq_after_snapshot(snapshot_seq, tail_seqs)
}

pub open spec fn production_hydrate_snapshot_tail_has_evidence(tail_len: int, snapshot_slots_len: int, snapshot_taint_len: int) -> bool {
    tail_len > 0 || snapshot_slots_len > 0 || snapshot_taint_len > 0
}

pub open spec fn production_hydrate_dimensions_positive(step_count: int, slot_count: int) -> bool {
    step_count > 0 && slot_count > 0
}

pub open spec fn valid_hydrate_snapshot_tail_preconditions(
    snapshot_run: RunId,
    snapshot_seq: int,
    tail_runs: Seq<RunId>,
    tail_seqs: Seq<int>,
    requested_run: RunId,
    snapshot_decodable: bool,
    has_evidence: bool,
    step_count: int,
    slot_count: int,
) -> bool {
    production_hydrate_snapshot_tail_run_matches(snapshot_run, tail_runs, requested_run)
        && tail_runs.len() == tail_seqs.len()
        && production_hydrate_snapshot_tail_seq_after_snapshot(snapshot_seq, tail_seqs)
        && snapshot_decodable
        && has_evidence
        && production_hydrate_dimensions_positive(step_count, slot_count)
        && step_count <= 65535
        && slot_count <= 65535
}

pub proof fn proof_production_preconditions_derive_valid_contract(
    snapshot_run: RunId,
    snapshot_seq: int,
    tail_runs: Seq<RunId>,
    tail_seqs: Seq<int>,
    requested_run: RunId,
    snapshot_decodable: bool,
    has_evidence: bool,
    step_count: int,
    slot_count: int,
)
    requires
        snapshot_run == requested_run,
        tail_runs.len() == tail_seqs.len(),
        all_tail_runs_match(tail_runs, requested_run),
        all_tail_seq_after_snapshot(snapshot_seq, tail_seqs),
        snapshot_decodable,
        has_evidence,
        0 < step_count <= 65535,
        0 < slot_count <= 65535,
    ensures
        valid_hydrate_snapshot_tail_preconditions(snapshot_run, snapshot_seq, tail_runs, tail_seqs, requested_run, snapshot_decodable, has_evidence, step_count, slot_count),
        production_hydrate_snapshot_tail_run_matches(snapshot_run, tail_runs, requested_run),
        production_hydrate_snapshot_tail_seq_after_snapshot(snapshot_seq, tail_seqs),
        production_hydrate_dimensions_positive(step_count, slot_count),
{}

pub proof fn proof_mismatched_snapshot_run_rejected(
    snapshot_run: RunId,
    requested_run: RunId,
    snapshot_seq: int,
    tail_runs: Seq<RunId>,
    tail_seqs: Seq<int>,
)
    requires snapshot_run != requested_run,
    ensures !production_hydrate_snapshot_tail_run_matches(snapshot_run, tail_runs, requested_run),
        !valid_hydrate_snapshot_tail_preconditions(snapshot_run, snapshot_seq, tail_runs, tail_seqs, requested_run, true, true, 1, 1),
{}

pub proof fn proof_missing_evidence_rejected(snapshot_run: RunId, requested_run: RunId, snapshot_seq: int, tail_runs: Seq<RunId>, tail_seqs: Seq<int>)
    ensures !valid_hydrate_snapshot_tail_preconditions(snapshot_run, snapshot_seq, tail_runs, tail_seqs, requested_run, true, false, 1, 1),
{}

}
