use vb_core::workflow::compiled_query::{
    validate_compiled_query_count, validate_compiled_query_summary,
};
use vb_core::workflow::compiled_slug::{
    validate_compiled_slug_count, validate_compiled_slug_summary,
};

/// Maximum slug/query count per workflow (both = 65_535).
pub const MAX_COUNT: usize = 65_535;
/// Maximum path depth per slug/query (both = 16).
pub const MAX_DEPTH: usize = 16;

// ── Flux contracts (legacy, kept for backward compat) ──

#[cfg_attr(flux, flux_rs::sig(fn(count: usize{count <= 65535}) -> usize{v: v <= 65535}))]
pub fn validated_slug_count(count: usize) -> usize {
    let _production = validate_compiled_slug_count(count);
    count
}

#[cfg_attr(flux, flux_rs::sig(fn(count: usize{count <= 65535}) -> usize{v: v <= 65535}))]
pub fn validated_query_count(count: usize) -> usize {
    let _production = validate_compiled_query_count(count);
    count
}

#[cfg_attr(flux, flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{v: v <= 16}))]
pub fn validated_slug_path_depth(depth: usize) -> usize {
    let _production = validate_compiled_slug_summary(0, 0, 0, depth, 0);
    depth
}

#[cfg_attr(flux, flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{v: v <= 16}))]
pub fn validated_query_path_depth(depth: usize) -> usize {
    let _production = validate_compiled_query_summary(0, 0, 0, depth, 0);
    depth
}

#[cfg_attr(flux, flux_rs::sig(fn(declared: u64, recomputed: u64{declared == recomputed}) -> u64{v: v == declared}))]
pub fn validated_total(declared: u64, recomputed: u64) -> u64 {
    let _slug = validate_compiled_slug_summary(0, recomputed, declared, 0, recomputed);
    let _query = validate_compiled_query_summary(0, recomputed, declared, 0, recomputed);
    recomputed
}

#[cfg_attr(flux, flux_rs::sig(fn(a: u64, b: u64{a + b <= 18446744073709551615}) -> u64{v: v == a + b}))]
pub fn checked_pair_sum(a: u64, b: u64) -> u64 {
    a + b
}

#[cfg_attr(flux, flux_rs::sig(fn(
    count: usize{count <= 65535},
    recomputed_total: u64,
    declared_total: u64{declared_total == recomputed_total},
    max_path_depth: usize{max_path_depth <= 16},
    max_budget: u64{recomputed_total <= max_budget}
) -> u64{remaining: remaining + recomputed_total == max_budget}))]
pub fn admitted_slug_summary(
    count: usize,
    recomputed_total: u64,
    declared_total: u64,
    max_path_depth: usize,
    max_budget: u64,
) -> u64 {
    let _production = validate_compiled_slug_summary(
        count,
        recomputed_total,
        declared_total,
        max_path_depth,
        max_budget,
    );
    max_budget - recomputed_total
}

#[cfg_attr(flux, flux_rs::sig(fn(
    count: usize{count <= 65535},
    recomputed_total: u64,
    declared_total: u64{declared_total == recomputed_total},
    max_path_depth: usize{max_path_depth <= 16},
    max_budget: u64{recomputed_total <= max_budget}
) -> u64{remaining: remaining + recomputed_total == max_budget}))]
pub fn admitted_query_summary(
    count: usize,
    recomputed_total: u64,
    declared_total: u64,
    max_path_depth: usize,
    max_budget: u64,
) -> u64 {
    let _production = validate_compiled_query_summary(
        count,
        recomputed_total,
        declared_total,
        max_path_depth,
        max_budget,
    );
    max_budget - recomputed_total
}

/// Witness that all positive contracts are simultaneously satisfiable.
pub fn positive_vb_ajc40_refinement_witness() {
    let slug_count = validated_slug_count(65535);
    let query_count = validated_query_count(65535);
    let slug_depth = validated_slug_path_depth(16);
    let query_depth = validated_query_path_depth(16);
    let total = validated_total(21, 21);
    let pair = checked_pair_sum(9, 12);
    let slug_remaining = admitted_slug_summary(slug_count, total, pair, slug_depth, 34);
    let query_remaining = admitted_query_summary(query_count, total, pair, query_depth, 34);
    assert!(slug_remaining == 13);
    assert!(query_remaining == 13);
}
