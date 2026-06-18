//! Taint lattice proof kernel.
//!
//! Local-only taint lattice sanity kernel. This module defines its own `Taint`
//! mirror type and is not bound to `vb_core::value::Taint` or production
//! `vb_core::value::join_taint`. Verus checks in this module are therefore
//! non-proof local model checks and must not be registered as production
//! evidence unless a future pass adds a reviewed production binding.

#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

// ── Verus path: verus! block with all Verus definitions ─────────────────
#[cfg(verus_keep_ghost)]
mod verus;
#[cfg(verus_keep_ghost)]
pub use verus::*;

// ── Non-Verus path: regular Rust modules ────────────────────────────────
#[cfg(not(verus_keep_ghost))]
mod lattice;
#[cfg(not(verus_keep_ghost))]
mod properties;
#[cfg(not(verus_keep_ghost))]
mod r#type;
#[cfg(not(verus_keep_ghost))]
pub use properties::*;

// ── Shared: tests and Kani harnesses ────────────────────────────────────
mod kani;
mod tests;

// ── Non-Verus re-exports ────────────────────────────────────────────────
#[cfg(not(verus_keep_ghost))]
pub use lattice::*;
#[cfg(not(verus_keep_ghost))]
pub use r#type::*;
