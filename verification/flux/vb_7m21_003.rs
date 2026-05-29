#![forbid(unsafe_code)]
//! PO-vb-7m21-013
use flux_rs::attrs::*;
#[sig(fn(usize) -> bool)]
pub fn truncated_header(len: usize) -> bool {
    len < 60
}
