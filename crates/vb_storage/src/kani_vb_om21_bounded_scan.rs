#![cfg(kani)]
#![forbid(unsafe_code)]
// Obligation: PO-vb-om21-bounded-scan-kani

#[kani::proof]
#[kani::unwind(5)]
fn vb_om21_bounded_scan_harness() {
    let seqs: [u64; 4] = kani::any();
    let mut max_seq = 0_u64;
    let mut seen = false;
    let mut idx = 0_usize;
    while idx < 4 {
        let seq = seqs[idx];
        if !seen || seq > max_seq {
            max_seq = seq;
        }
        seen = true;
        idx += 1;
    }
    assert!(seen);
    assert!(max_seq == seqs[0] || max_seq == seqs[1] || max_seq == seqs[2] || max_seq == seqs[3]);
    assert!(max_seq >= seqs[0] && max_seq >= seqs[1] && max_seq >= seqs[2] && max_seq >= seqs[3]);
}
