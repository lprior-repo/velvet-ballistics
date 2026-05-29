#![forbid(unsafe_code)]

extern crate flux_rs;

use flux_rs::attrs::*;

// PO-VB-DYBJ-005
// Flux-supported standalone refinement artifact for the accepted digest shape.
// The production API `WorkflowDigest::from_bytes` accepts only `[u8; 32]`; this
// wrapper records the same fixed-length shape as a Flux-refined verification
// type without editing production Rust.

#[refined_by(len: int)]
pub struct DigestShape {
    #[field([u8; 32])]
    bytes: [u8; 32],
}

#[sig(fn(bytes: [u8; 32]) -> DigestShape[32])]
pub fn digest_shape_is_exactly_32_bytes(bytes: [u8; 32]) -> DigestShape {
    DigestShape { bytes }
}

#[sig(fn(shape: DigestShape[32]) -> [u8; 32])]
pub fn digest_shape_returns_exactly_32_bytes(shape: DigestShape) -> [u8; 32] {
    shape.bytes
}

#[sig(fn() -> DigestShape[32])]
pub fn digest_shape_selected_pattern() -> DigestShape {
    digest_shape_is_exactly_32_bytes([0xA5_u8; 32])
}
