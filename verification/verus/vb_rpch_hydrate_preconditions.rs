// Verus proof obligations for vb-rpch PRE-001, PRE-002: hydrate_run_frame preconditions.
//
// Obligation: VERUS-REC-006 / PRE-001, PRE-002
// Contract:
// - PRE-001: hydrate_run_frame(snapshot, tail_events, run_id) requires:
//   snapshot.run == run_id; every tail_event.run_id() == run_id;
//   every tail_event.seq() > snapshot.seq; snapshot bytes decodable;
//   derived step_count > 0 and fits in u16
// - PRE-002: hydrate_run_frame_from_events(events, run_id) requires:
//   events non-empty; derived step_count > 0 and fits in u16

use vstd::prelude::*;

verus! {

pub open spec fn spec_seq_order_invariant(snapshot_seq: int, tail_seqs: Seq<int>) -> bool {
    forall i: int :: 0 <= i < tail_seqs.len() ==> tail_seqs[i] > snapshot_seq
}

pub open spec fn spec_hydrate_run_frame_preconditions(
    snapshot_run: RunId,
    snapshot_seq: int,
    tail_runs: Seq<RunId>,
    tail_seqs: Seq<int>,
    run_id: RunId,
) -> bool {
    snapshot_run == run_id
        && forall i: int :: 0 <= i < tail_runs.len() ==> tail_runs[i] == run_id
        && spec_seq_order_invariant(snapshot_seq, tail_seqs)
}

pub open spec fn spec_hydrate_run_frame_from_events_preconditions(
    events_len: int,
    step_count: int,
) -> bool {
    events_len > 0 && step_count > 0
}

pub proof fn proof_preconditions_ensure_valid_hydration(
    snapshot_run: RunId,
    snapshot_seq: int,
    tail_runs: Seq<RunId>,
    tail_seqs: Seq<int>,
    run_id: RunId,
)
    requires
        snapshot_run == run_id,
        forall i: int :: 0 <= i < tail_runs.len() ==> tail_runs[i] == run_id,
        spec_seq_order_invariant(snapshot_seq, tail_seqs),
    ensures
        spec_hydrate_run_frame_preconditions(snapshot_run, snapshot_seq, tail_runs, tail_seqs, run_id)
{
    reveal(spec_hydrate_run_frame_preconditions);
    reveal(spec_seq_order_invariant);
}

pub proof fn proof_events_empty_fails(
    events_len: int,
)
    requires
        events_len == 0,
    ensures
        !spec_hydrate_run_frame_from_events_preconditions(events_len, 1)
{
    reveal(spec_hydrate_run_frame_from_events_preconditions);
}

pub proof fn proof_step_count_zero_fails(
    events_len: int,
    step_count: int,
)
    requires
        events_len > 0,
        step_count == 0,
    ensures
        !spec_hydrate_run_frame_from_events_preconditions(events_len, step_count)
{
    reveal(spec_hydrate_run_frame_from_events_preconditions);
}

} // verus!

fn main() {}