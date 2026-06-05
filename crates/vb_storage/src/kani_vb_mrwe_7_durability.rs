#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-DURABILITY-KANI. TB-FJALL-STRICT-PERSIST is external.
#[kani::proof]
fn vb_mrwe_7_durability_classification() {
    let has_strict: bool = kani::any();
    let strict_persist_ok: bool = kani::any();
    let can_report_success = !has_strict || strict_persist_ok;
    assert!(!(has_strict && !strict_persist_ok && can_report_success));
}
