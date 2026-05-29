#![forbid(unsafe_code)]
//! PO-vb-7m21-008
use flux_rs::attrs::*;
#[sig(fn(u16) -> bool)]
pub fn future_schema(version: u16) -> bool {
    version > 1
}
