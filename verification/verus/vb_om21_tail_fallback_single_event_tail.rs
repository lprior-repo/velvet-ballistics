// Obligation: PO-vb-om21-single-event-tail-verus
use vstd::prelude::*;

verus! {
proof fn proof_single_event_tail()
    ensures 0int + 1int == 1int
{
}
}
