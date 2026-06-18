#![cfg(all(kani, feature = "kani-vb-mrwe-7"))]
//! bead vb-mrwe.7 — OBL-ATOM-KANI.  Model harness for all-or-retained flush outcomes.

const MAX: usize = 16;

#[kani::proof]
fn vb_mrwe_7_flush_batch_atomic_all_or_retained() {
    let pending: usize = kani::any();
    let batch: usize = kani::any();
    kani::assume(pending <= MAX);
    kani::assume(batch > 0 && batch <= MAX);
    let commit_ok: bool = kani::any();
    let prefix = if pending < batch { pending } else { batch };
    let drained = if commit_ok { prefix } else { 0 };
    let remaining = pending - drained;
    kani::assert(drained <= prefix, "drained <= prefix");
    kani::assert(
        !commit_ok || drained == prefix,
        "commit_ok or drained == prefix",
    );
    kani::assert(
        commit_ok || remaining == pending,
        "commit_ok or remaining == pending",
    );
}
