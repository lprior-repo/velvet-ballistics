#![forbid(unsafe_code)]
//! PO-vb-7m21-003
use flux_rs::attrs::*;
#[sig(fn(u32, u32) -> bool)]
pub fn oversized(len: u32, max: u32) -> bool {
    len > max
}
