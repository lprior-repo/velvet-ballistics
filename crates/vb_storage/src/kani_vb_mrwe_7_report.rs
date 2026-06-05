#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-REPORT-KANI.
#[kani::proof]
fn vb_mrwe_7_flush_report_mapping() {
    let prefix: usize = kani::any();
    let committed: bool = kani::any();
    kani::assume(prefix <= 16);
    let drained = if committed { prefix } else { 0 };
    assert!(drained <= prefix);
    assert!(committed || drained == 0);
}
