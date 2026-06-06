#![allow(unused)]

// vb-ajc40 Flux artifact — PO-029.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_query_path_depth.rs
// Production seam targeted: compiled_query.rs::{YbBoundedQuery::path_depth,
// YbBoundedQuery::is_path_too_deep, validate_compiled_query_summary}.

#[flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{accepted: accepted <= 16})]
fn admitted_query_path_depth(depth: usize) -> usize {
    depth
}

fn positive_query_path_depth_boundary() {
    let root = admitted_query_path_depth(0);
    let limit = admitted_query_path_depth(16);
    assert!(root == 0);
    assert!(limit == 16);
}
