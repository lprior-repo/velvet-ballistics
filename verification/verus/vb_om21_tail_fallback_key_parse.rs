// Obligation: PO-vb-om21-key-parse-verus
use vstd::prelude::*;

verus! {
spec fn can_decode(len: int, prefix_ok: bool) -> bool { len == 17 && prefix_ok }

proof fn proof_decode_guard(len: int, prefix_ok: bool)
    ensures can_decode(len, prefix_ok) ==> len >= 17
{
}
}
