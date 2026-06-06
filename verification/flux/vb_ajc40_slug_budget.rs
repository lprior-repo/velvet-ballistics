#![allow(unused)]

// vb-ajc40 Flux artifact — PO-013.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_slug_budget.rs
// Production seam targeted: compiled_slug.rs::validate_compiled_slug_summary and
// YbBoundedSlugs::remaining_budget.

#[flux_rs::sig(fn(
    declared_total: u64,
    recomputed_total: u64{declared_total == recomputed_total},
    max_budget: u64{recomputed_total <= max_budget}
) -> u64{remaining: remaining + recomputed_total == max_budget})]
fn slug_remaining_after_validated_total(
    declared_total: u64,
    recomputed_total: u64,
    max_budget: u64,
) -> u64 {
    let _ = declared_total;
    max_budget - recomputed_total
}

fn positive_slug_remaining_budget() {
    let remaining = slug_remaining_after_validated_total(8, 8, 13);
    assert!(remaining == 5);
}
