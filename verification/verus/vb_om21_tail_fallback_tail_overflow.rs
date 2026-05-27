// Obligation: PO-vb-om21-tail-overflow-verus
use vstd::prelude::*;

verus! {
spec const MAX_U64: int = 18446744073709551615;
enum Outcome { Ok(int), TailOverflow }
spec fn tail_after(max_seq: int) -> Outcome { if max_seq == MAX_U64 { Outcome::TailOverflow } else { Outcome::Ok(max_seq + 1) } }

proof fn proof_overflow_typed()
    ensures tail_after(MAX_U64) == Outcome::TailOverflow
{
}
}
