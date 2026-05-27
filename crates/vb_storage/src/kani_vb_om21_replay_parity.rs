#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-replay-parity-kani

#[kani::proof]
fn vb_om21_replay_parity_harness() {
    let requested_run: u64 = kani::any();
    let event_run: u64 = kani::any();
    let expected_seq: u64 = kani::any();
    let event_seq: u64 = kani::any();
    let accepted = event_run == requested_run && event_seq == expected_seq;
    kani::assert(!accepted || (event_run == requested_run && event_seq == expected_seq),
        "replay-parity: accepted events match requested run and expected sequence");
    kani::assert(accepted || event_run != requested_run || event_seq != expected_seq,
        "replay-parity: rejected events have run or sequence mismatch");
    kani::cover!(accepted, "replay-parity-accepted");
    kani::cover!(!accepted, "replay-parity-rejected");
}
