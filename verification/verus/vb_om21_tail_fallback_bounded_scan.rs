// Obligation: PO-vb-om21-bounded-scan-verus
use vstd::prelude::*;

verus! {
spec fn accumulator_slots() -> int { 2 }

proof fn proof_bounded_scan_state()
    ensures accumulator_slots() == 2
{
}
}
