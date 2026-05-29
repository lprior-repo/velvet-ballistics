use vstd::prelude::*;

verus! {
pub enum DecodeMode { SkipDecode, DecodeHeader, DecodePayload }
pub open spec fn decode_effect_allowed(mode: DecodeMode) -> bool { !(mode is SkipDecode) }
pub proof fn lemma_skip_decode_has_no_payload_decode_effect()
    ensures !decode_effect_allowed(DecodeMode::SkipDecode)
{}
}
