#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(stage: int)]
#[invariant(0 <= stage && stage <= 8)]
pub struct DecodeStage { #[field(stage)] pub stage: usize }

#[refined_by(ok: bool)]
pub struct IntegrityChecked { #[field(ok)] pub ok: bool }
