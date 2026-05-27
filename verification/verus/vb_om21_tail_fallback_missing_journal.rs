// Obligation: PO-vb-om21-missing-journal-verus
use vstd::prelude::*;

verus! {
enum Outcome { Ok(int), MissingJournal }
spec fn classify(recovery: bool, seen: bool) -> Outcome { if recovery && !seen { Outcome::MissingJournal } else { Outcome::Ok(0) } }

proof fn proof_missing_journal()
    ensures classify(true, false) == Outcome::MissingJournal
{
}
}
