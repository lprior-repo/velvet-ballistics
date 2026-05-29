#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(state: int)]
#[invariant(state == 0)]
pub struct ReadOnlyStorage { #[field(0)] marker: usize }

#[sig(fn(&ReadOnlyStorage) -> bool[false])]
pub fn can_write(_: &ReadOnlyStorage) -> bool { false }
