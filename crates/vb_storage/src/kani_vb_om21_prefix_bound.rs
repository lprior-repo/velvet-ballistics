#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-prefix-bound-kani
use crate::kani_vb_om21_model::{encode_run_event_key, has_prefix, parse_seq_if_prefix};

#[kani::proof]
#[kani::unwind(18)]
fn vb_om21_prefix_bound_harness() {
    let run: u64 = kani::any();
    let other_run: u64 = kani::any();
    let seq: u64 = kani::any();
    let key = encode_run_event_key(run, seq);
    kani::assert(has_prefix(&key, run),
        "prefix-bound: key encodes correct run prefix");
    kani::assert(parse_seq_if_prefix(&key, run) == Some(seq),
        "prefix-bound: correct run parses encoded sequence");
    kani::assert(other_run == run || parse_seq_if_prefix(&key, other_run).is_none(),
        "prefix-bound: mismatched run prefix rejected (no cross-run leak)");
    kani::cover!(seq == 0, "prefix-bound-zero-seq");
    kani::cover!(seq == u64::MAX, "prefix-bound-max-seq");
}
