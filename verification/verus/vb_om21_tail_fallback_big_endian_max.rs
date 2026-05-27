// Obligation: PO-vb-om21-big-endian-max-verus
use vstd::prelude::*;

verus! {
spec const MAX_U64: int = 18446744073709551615;
spec fn valid_u64(x: int) -> bool { 0 <= x && x <= MAX_U64 }
spec fn be_lex_le_same_prefix(a: int, b: int) -> bool recommends valid_u64(a), valid_u64(b) { a <= b }

proof fn proof_big_endian_order(a: int, b: int)
    requires valid_u64(a), valid_u64(b)
    ensures be_lex_le_same_prefix(a, b) == (a <= b)
{
}
}
