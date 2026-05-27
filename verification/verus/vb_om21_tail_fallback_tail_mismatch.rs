// Obligation: PO-vb-om21-tail-mismatch-verus
use vstd::prelude::*;

verus! {
enum Outcome { Ok(int), TailMismatch }
spec fn classify(seen: bool, stale_metadata: bool, tail: int) -> Outcome { if seen && stale_metadata { Outcome::TailMismatch } else { Outcome::Ok(tail) } }

proof fn proof_tail_mismatch(tail: int)
    ensures classify(true, true, tail) == Outcome::TailMismatch
{
}
}
