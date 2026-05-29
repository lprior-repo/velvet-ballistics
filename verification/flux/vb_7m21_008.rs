#![forbid(unsafe_code)]
//! PO-vb-7m21-038
use flux_rs::attrs::*;
#[sig(fn(u8, u8) -> bool)]
pub fn missing_keyspace(declared: u8, present: u8) -> bool {
    declared & !present != 0
}
