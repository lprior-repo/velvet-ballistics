#![allow(unused)]

// vb-ajc40 Flux artifact — PO-041.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_empty_path_semantics.rs
// Production seams targeted: compiled_slug.rs::YbBoundedSlug and
// compiled_query.rs::YbBoundedQuery root accessor path handling.

#[flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{accepted: accepted <= 16})]
fn admitted_root_or_bounded_path(depth: usize) -> usize {
    depth
}

fn positive_empty_path_root_accessor() {
    let slug_root = admitted_root_or_bounded_path(0);
    let query_root = admitted_root_or_bounded_path(0);
    let boundary = admitted_root_or_bounded_path(16);
    assert!(slug_root == 0);
    assert!(query_root == 0);
    assert!(boundary == 16);
}
