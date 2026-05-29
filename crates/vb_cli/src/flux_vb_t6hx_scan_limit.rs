#![cfg(flux)]
use flux_rs::attrs::*;

#[refined_by(n: int)]
#[invariant(0 < n && n <= 65536)]
pub struct ScanLimit { #[field(n)] pub value: usize }

#[refined_by(len: int, limit: int)]
#[invariant(0 <= len && len <= limit)]
pub struct BoundedRows { #[field(len)] pub len: usize, #[field(limit)] pub limit: usize }
