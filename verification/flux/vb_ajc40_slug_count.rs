#![allow(unused)]

// vb-ajc40 Flux artifact — PO-033.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_slug_count.rs
// Production seam targeted: compiled_slug.rs::{validate_compiled_slug_count,
// validate_compiled_slug_summary, YbBoundedSlugs::len}.

#[flux_rs::sig(fn(count: usize{count <= 65535}) -> usize{accepted: accepted <= 65535})]
fn admitted_slug_count(count: usize) -> usize {
    count
}

fn positive_slug_count_boundary() {
    let empty = admitted_slug_count(0);
    let limit = admitted_slug_count(65535);
    assert!(empty == 0);
    assert!(limit == 65535);
}
