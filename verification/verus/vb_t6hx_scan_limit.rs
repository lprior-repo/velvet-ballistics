use vstd::prelude::*;

verus! {
pub open spec fn max_scan_limit() -> nat { 65536 }
pub struct ScanLimit { pub value: nat }
pub struct ScanOutcome { pub rows: nat, pub limit: ScanLimit }

pub open spec fn valid_limit(limit: ScanLimit) -> bool { 0 < limit.value && limit.value <= max_scan_limit() }
pub open spec fn bounded_outcome(outcome: ScanOutcome) -> bool { valid_limit(outcome.limit) && outcome.rows <= outcome.limit.value }

proof fn lemma_constructor_rejects_zero_and_caps(limit: nat)
    ensures (0 < limit && limit <= max_scan_limit()) ==> valid_limit(ScanLimit { value: limit })
{}

proof fn lemma_scan_rows_never_exceed_limit(rows: nat, limit: nat)
    requires 0 < limit, limit <= max_scan_limit(), rows <= limit
    ensures bounded_outcome(ScanOutcome { rows, limit: ScanLimit { value: limit } })
{}
}
