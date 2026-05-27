// Obligation: PO-vb-om21-replay-parity-verus
use vstd::prelude::*;

verus! {
spec fn replay_accept(request_run: int, event_run: int, expected_seq: int, event_seq: int) -> bool {
    request_run == event_run && expected_seq == event_seq
}

proof fn proof_replay_parity(request_run: int, event_run: int, expected_seq: int, event_seq: int)
    ensures replay_accept(request_run, event_run, expected_seq, event_seq) ==> request_run == event_run && expected_seq == event_seq
{
}
}
