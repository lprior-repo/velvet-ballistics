#![allow(unused)]

// vb-ajc40 negative Flux artifact for PO-003/008/013/017/021/025/029/033/037/041.
// Expected command and result:
//   flux --crate-type=lib --edition=2024 verification/flux/vb_ajc40_invalid_state_rejections_neg.rs
//   MUST FAIL with precondition diagnostics for each invalid-state constructor.

#[flux_rs::sig(fn(decoded: bool{decoded}) -> bool[true])]
fn require_decode_success(decoded: bool) -> bool {
    decoded
}

#[flux_rs::sig(fn(count: usize{count <= 65535}) -> usize{accepted: accepted <= 65535})]
fn admitted_count(count: usize) -> usize {
    count
}

#[flux_rs::sig(fn(depth: usize{depth <= 16}) -> usize{accepted: accepted <= 16})]
fn admitted_path_depth(depth: usize) -> usize {
    depth
}

#[flux_rs::sig(fn(declared_total: u64, recomputed_total: u64{declared_total == recomputed_total}) -> u64[recomputed_total])]
fn validated_total(declared_total: u64, recomputed_total: u64) -> u64 {
    let _ = declared_total;
    recomputed_total
}

#[flux_rs::sig(fn(a: u64, b: u64{a + b <= 18446744073709551615}) -> u64{sum: sum == a + b})]
fn checked_pair_sum(a: u64, b: u64) -> u64 {
    a + b
}

#[flux_rs::sig(fn(recomputed_total: u64, max_budget: u64{recomputed_total <= max_budget}) -> u64{remaining: remaining + recomputed_total == max_budget})]
fn remaining_budget(recomputed_total: u64, max_budget: u64) -> u64 {
    max_budget - recomputed_total
}

fn malformed_decode_is_not_admitted() {
    let _ = require_decode_success(false);
}

fn too_many_items_are_not_admitted() {
    let _ = admitted_count(65536);
}

fn too_deep_paths_are_not_admitted() {
    let _ = admitted_path_depth(17);
}

fn total_mismatch_is_not_validated() {
    let _ = validated_total(12, 13);
}

fn overflowing_pair_sum_is_not_validated() {
    let _ = checked_pair_sum(18446744073709551615, 1);
}

fn over_budget_total_has_no_remaining_budget() {
    let _ = remaining_budget(26, 25);
}
