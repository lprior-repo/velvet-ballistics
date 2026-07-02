#![forbid(unsafe_code)]
#![allow(dead_code)]
//! Single-byte tag projections for runtime-policy and taint discriminants.
//!
//! These are pure byte-projection helpers shared by the observation
//! encoder ([`super::encode`]) and the action observer
//! ([`super::action`]). Kept in their own module so the byte-tag
//! contract is reviewed in one place.

use vb_core::{RuntimePolicy, Taint};

/// Capture the policy discriminant into a single byte.
#[must_use]
pub(crate) const fn policy_tag(policy: RuntimePolicy) -> u8 {
    match policy {
        RuntimePolicy::Strict => 1,
        RuntimePolicy::Journaled => 2,
        RuntimePolicy::Relaxed => 3,
        // Future-proof: unknown policy discriminants get a sentinel distinct
        // from the known tags so divergence tests detect them.
        _ => 0xFF,
    }
}

/// Capture the `repr(u8)` discriminant of `vb_core::Taint` for stable encoding.
///
/// `vb_core::Taint` is declared `#[repr(u8)]` with explicit `= 0..=4`
/// discriminants, so the cast is the documented contract rather than an
/// implicit truncation. The cast is the only way to project the type into
/// the observation byte layout.
#[must_use]
pub(crate) const fn taint_tag_value(taint: Taint) -> u8 {
    // `vb_core::Taint` is `#[repr(u8)]` with explicit discriminants;
    // this is a documented byte projection, not a numeric conversion.
    #[allow(clippy::as_conversions)]
    let tag = taint as u8;
    tag
}
