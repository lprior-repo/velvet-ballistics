#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(len: int)]
#[invariant(0 < len && len <= 64)]
pub struct HexKey { #[field(len)] pub byte_len: usize }
