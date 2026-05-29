use vstd::prelude::*;

verus! {
pub open spec fn max_preview_bytes() -> nat { 65536 }
pub struct Preview { pub input_len: nat, pub shown_len: nat, pub cap: nat, pub omitted: nat }
pub open spec fn valid_preview(p: Preview) -> bool {
    0 < p.cap && p.cap <= max_preview_bytes() && p.shown_len <= p.cap && p.shown_len <= p.input_len && p.omitted == p.input_len - p.shown_len
}
pub proof fn lemma_preview_bound(input_len: nat, shown_len: nat, cap: nat)
    requires 0 < cap, cap <= max_preview_bytes(), shown_len <= cap, shown_len <= input_len
    ensures valid_preview(Preview { input_len, shown_len, cap, omitted: (input_len - shown_len) as nat })
{}
}
