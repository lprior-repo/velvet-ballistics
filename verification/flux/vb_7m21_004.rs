#![forbid(unsafe_code)]
//! PO-vb-7m21-019
use flux_rs::attrs::*;
#[sig(fn(bool, bool) -> bool)]
pub fn missing_side_index(event: bool, index: bool) -> bool {
    event && !index
}
