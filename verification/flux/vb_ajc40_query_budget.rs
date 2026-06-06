#![allow(unused)]

// vb-ajc40 Flux artifact — PO-017.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_query_budget.rs
// Production seam targeted: compiled_query.rs::validate_compiled_query_summary and
// YbBoundedQueries::remaining_budget.

#[flux_rs::sig(fn(
    declared_total: u64,
    recomputed_total: u64{declared_total == recomputed_total},
    max_budget: u64{recomputed_total <= max_budget}
) -> u64{remaining: remaining + recomputed_total == max_budget})]
fn query_remaining_after_validated_total(
    declared_total: u64,
    recomputed_total: u64,
    max_budget: u64,
) -> u64 {
    let _ = declared_total;
    max_budget - recomputed_total
}

fn positive_query_remaining_budget() {
    let remaining = query_remaining_after_validated_total(8, 8, 13);
    assert!(remaining == 5);
}
