#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-key-parse-kani
use crate::constants::{JOURNAL_KEY_BYTES, PREFIX_RUN_EVENT};
use crate::kani_vb_om21_model::parse_seq_if_prefix;

#[kani::proof]
#[kani::unwind(18)]
fn vb_om21_key_parse_harness() {
    let bytes: [u8; JOURNAL_KEY_BYTES] = kani::any();
    let run: u64 = kani::any();
    let parsed = parse_seq_if_prefix(&bytes, run);
    kani::assert(parsed.is_none() || bytes[0] == PREFIX_RUN_EVENT,
        "key-parse: Some decodes only from prefix-matching keys (no unchecked slicing on malformed)");
    kani::cover!(bytes[0] != PREFIX_RUN_EVENT && parsed.is_none(), "malformed-prefix-rejected");
}
