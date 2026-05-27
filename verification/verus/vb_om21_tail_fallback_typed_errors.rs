// Obligation: PO-vb-om21-typed-errors-verus
use vstd::prelude::*;

verus! {
enum Outcome { Ok(int), MissingJournal, TailMismatch, TailOverflow }
spec fn classify(recovery: bool, seen: bool, stale: bool, overflow: bool) -> Outcome {
    if recovery && !seen { Outcome::MissingJournal }
    else if overflow { Outcome::TailOverflow }
    else if stale { Outcome::TailMismatch }
    else { Outcome::Ok(0) }
}

proof fn proof_typed_error_separation()
    ensures classify(true, false, false, false) == Outcome::MissingJournal,
            classify(true, true, true, false) == Outcome::TailMismatch,
            classify(true, true, false, true) == Outcome::TailOverflow
{
}
}
