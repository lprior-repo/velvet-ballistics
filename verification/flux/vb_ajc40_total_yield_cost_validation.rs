#![allow(unused)]

// vb-ajc40 Flux artifact — PO-021.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_total_yield_cost_validation.rs
// Production seams targeted: compiled_slug.rs::checked_total_yield_cost,
// validate_compiled_slug_summary; compiled_query.rs::checked_total_yield_cost,
// validate_compiled_query_summary.

#[flux_rs::sig(fn(a: u64, b: u64{a + b <= 18446744073709551615}) -> u64{sum: sum == a + b})]
fn checked_pair_sum(a: u64, b: u64) -> u64 {
    a + b
}

#[flux_rs::sig(fn(declared_total: u64, recomputed_total: u64{declared_total == recomputed_total}) -> u64[recomputed_total])]
fn validated_total_from_recomputed_sum(declared_total: u64, recomputed_total: u64) -> u64 {
    let _ = declared_total;
    recomputed_total
}

fn positive_validated_total() {
    let recomputed = checked_pair_sum(9, 13);
    let total = validated_total_from_recomputed_sum(22, recomputed);
    assert!(total == 22);
}
