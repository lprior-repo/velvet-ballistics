#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(len: int, cap: int)]
#[invariant(0 <= len && len <= cap)]
pub struct BoundedPreview { #[field(len)] pub len: usize, #[field(cap)] pub cap: usize }
