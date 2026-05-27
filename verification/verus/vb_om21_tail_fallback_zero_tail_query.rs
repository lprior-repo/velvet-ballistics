// Obligation: PO-vb-om21-zero-tail-query-verus
use vstd::prelude::*;

verus! {
spec fn query_tail(seen: bool) -> int { if seen { 1 } else { 0 } }

proof fn proof_zero_tail_query()
    ensures query_tail(false) == 0
{
}
}
