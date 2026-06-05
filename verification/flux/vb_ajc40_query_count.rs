#![allow(unused)]

// vb-ajc40 Flux artifact — PO-037.
// Command: flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_query_count.rs
// Production seam targeted: compiled_query.rs::{validate_compiled_query_count,
// validate_compiled_query_summary, YbBoundedQueries::len}.

#[flux_rs::sig(fn(count: usize{count <= 65535}) -> usize{accepted: accepted <= 65535})]
fn admitted_query_count(count: usize) -> usize {
    count
}

fn positive_query_count_boundary() {
    let empty = admitted_query_count(0);
    let limit = admitted_query_count(65535);
    assert!(empty == 0);
    assert!(limit == 65535);
}
