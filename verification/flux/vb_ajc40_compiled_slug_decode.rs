#![allow(unused)]

// vb-ajc40 Flux artifact — PO-003.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_compiled_slug_decode.rs
// Production seam targeted: compiled_slug.rs::{from_bytes_compiled_slugs,
// validate_compiled_slugs, validate_compiled_slug_summary, YbBoundedSlugs}.

#[flux_rs::sig(fn(
    decoded: bool{decoded},
    count: usize{count <= 65535},
    max_path_depth: usize{max_path_depth <= 16},
    declared_total: u64,
    recomputed_total: u64{declared_total == recomputed_total},
    max_budget: u64{recomputed_total <= max_budget}
) -> u64{remaining: remaining + recomputed_total == max_budget})]
fn admit_decoded_slug_summary(
    decoded: bool,
    count: usize,
    max_path_depth: usize,
    declared_total: u64,
    recomputed_total: u64,
    max_budget: u64,
) -> u64 {
    let _ = decoded;
    let _ = count;
    let _ = max_path_depth;
    let _ = declared_total;
    max_budget - recomputed_total
}

fn positive_slug_decode_boundary() {
    let remaining = admit_decoded_slug_summary(true, 65535, 16, 21, 21, 34);
    assert!(remaining == 13);
}
