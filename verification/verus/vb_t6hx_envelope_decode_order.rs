use vstd::prelude::*;

verus! {
pub enum DecodeStage { HeaderLen, Magic, Schema, Family, PayloadLen, HeaderCrc, Availability, Digest, Postcard, Error }
pub open spec fn can_postcard(len_ok: bool, crc_ok: bool, avail_ok: bool, digest_ok: bool) -> bool {
    len_ok && crc_ok && avail_ok && digest_ok
}
pub proof fn lemma_postcard_only_after_integrity(len_ok: bool, crc_ok: bool, avail_ok: bool, digest_ok: bool)
    requires can_postcard(len_ok, crc_ok, avail_ok, digest_ok)
    ensures len_ok, crc_ok, avail_ok, digest_ok
{}
}
