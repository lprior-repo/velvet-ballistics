#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(effect: int)]
#[invariant(effect == 0 || effect == 1)]
pub struct DecodeMode { #[field(effect)] pub decode_effect: usize }
