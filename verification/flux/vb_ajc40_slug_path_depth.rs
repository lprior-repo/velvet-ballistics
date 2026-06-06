#![allow(unused)]

// vb-ajc40 Flux artifact — PO-025.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_slug_path_depth.rs
// Production seam targeted: compiled_slug.rs::{YbBoundedSlug::path_depth,
// YbBoundedSlug::is_path_too_deep, validate_compiled_slug_summary}.

#[flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{accepted: accepted <= 16})]
fn admitted_slug_path_depth(depth: usize) -> usize {
    depth
}

fn positive_slug_path_depth_boundary() {
    let root = admitted_slug_path_depth(0);
    let limit = admitted_slug_path_depth(16);
    assert!(root == 0);
    assert!(limit == 16);
}
