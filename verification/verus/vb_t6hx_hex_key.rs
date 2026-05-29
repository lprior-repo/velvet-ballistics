use vstd::prelude::*;

verus! {
pub open spec fn max_hex_key_bytes() -> nat { 64 }
pub open spec fn is_hex_nybble(byte: u8) -> bool {
    (48 <= byte && byte <= 57) || (65 <= byte && byte <= 70) || (97 <= byte && byte <= 102)
}
pub open spec fn valid_hex_len(len: nat) -> bool { 0 < len && len <= max_hex_key_bytes() * 2 && len % 2 == 0 }

pub proof fn lemma_valid_hex_length_even_nonempty(len: nat)
    requires valid_hex_len(len)
    ensures len > 0, len % 2 == 0, len <= max_hex_key_bytes() * 2
{}
}
