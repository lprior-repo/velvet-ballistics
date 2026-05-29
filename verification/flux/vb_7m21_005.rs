#![forbid(unsafe_code)]
//! PO-vb-7m21-024
use flux_rs::attrs::*;
#[sig(fn(u64, u64) -> bool)]
pub fn sequence_gap(expected: u64, actual: u64) -> bool {
    expected != actual
}
