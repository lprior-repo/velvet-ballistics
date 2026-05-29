#![forbid(unsafe_code)]
//! PO-vb-7m21-033
use flux_rs::attrs::*;
#[sig(fn(u64, u64) -> bool)]
pub fn stale_snapshot(snapshot_seq: u64, tail_seq: u64) -> bool {
    snapshot_seq < tail_seq
}
