#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-big-endian-max-kani
use crate::kani_vb_om21_model::{encode_run_event_key, lex_le_run_event, parse_seq_if_prefix};

#[kani::proof]
#[kani::unwind(18)]
fn vb_om21_big_endian_max_harness() {
    let run: u64 = kani::any();
    let a: u64 = kani::any();
    let b: u64 = kani::any();
    let ka = encode_run_event_key(run, a);
    let kb = encode_run_event_key(run, b);
    kani::assert(parse_seq_if_prefix(&ka, run) == Some(a),
        "big-endian-max: key-a roundtrips sequence a");
    kani::assert(parse_seq_if_prefix(&kb, run) == Some(b),
        "big-endian-max: key-b roundtrips sequence b");
    kani::assert(lex_le_run_event(&ka, &kb) == (a <= b),
        "big-endian-max: lexicographic order over final 8 BE bytes matches numeric EventSeq order");
    kani::cover!(a < b, "big-endian-strictly-less");
    kani::cover!(a == u64::MAX, "big-endian-max-edge");
}
