// Obligation: PO-vb-om21-prefix-bound-verus
use vstd::prelude::*;

verus! {
spec const PREFIX: int = 0x11;
spec const MAX_U64: int = 18446744073709551615;

spec fn valid_u64(x: int) -> bool { 0 <= x && x <= MAX_U64 }
spec fn same_prefix(key_run: int, wanted_run: int, tag: int) -> bool {
    tag == PREFIX && key_run == wanted_run
}
spec fn parse_seq(key_run: int, wanted_run: int, tag: int, seq: int) -> Option<int> {
    if same_prefix(key_run, wanted_run, tag) { Some(seq) } else { None }
}

proof fn proof_prefix_bound(key_run: int, wanted_run: int, seq: int)
    requires valid_u64(key_run), valid_u64(wanted_run), valid_u64(seq)
    ensures parse_seq(key_run, wanted_run, PREFIX, seq).is_Some() ==> key_run == wanted_run
{
}
}
