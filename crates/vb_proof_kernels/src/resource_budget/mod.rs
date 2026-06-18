//! Resource budget proof kernel.
//!
//! Local-only resource budget sanity kernel. The Verus layer models a mirror
//! `Budget` type with mathematical `nat` fields; it is not bound to production
//! `vb_core` resource-budget types or methods. Successful Verus checking of
//! this module is local model evidence only, not production deductive evidence.
//!
//! # Module layout
//!
//! | Module       | Contents                                                    |
//! |--------------|-------------------------------------------------------------|
//! | `budget`     | `Budget` type (Verus `nat` / Cargo `u64` dual compilation) |
//! | `spec`       | Verus spec + exec functions (`spec_sequential_add`, etc.)  |
//! | `lemmas`     | Verus proof lemmas (commutativity, associativity, …)       |
//! | `policy`     | `Policy` type + `within` violation check (cargo only)      |
//! | `combinator` | Standalone `*_compose` functions + tests (cargo only)      |

// ── Always-available sub-modules ────────────────────────────────────────────

pub mod budget;
pub mod combinator;

// ── Verus-only sub-modules (depend on vstd / verus!) ────────────────────────

#[cfg(verus_keep_ghost)]
pub mod lemmas;

#[cfg(verus_keep_ghost)]
pub mod spec;

// ── Cargo-only sub-modules (depend on u64 / Vec) ────────────────────────────

#[cfg(not(verus_keep_ghost))]
pub mod policy;

// ── Re-exports ──────────────────────────────────────────────────────────────

// Core Budget type (available in both modes)
pub use budget::Budget;

// Verus spec / exec functions — only in verus mode
#[cfg(verus_keep_ghost)]
pub use spec::{
    branch_max, loop_mul, sequential_add, spec_branch_max, spec_is_zero_budget, spec_loop_mul,
    spec_sequential_add,
};

// Verus lemmas — only in verus mode
#[cfg(verus_keep_ghost)]
pub use lemmas::*;

// Policy — cargo only
#[cfg(not(verus_keep_ghost))]
pub use policy::Policy;

// Combinators — cargo only
#[cfg(not(verus_keep_ghost))]
pub use combinator::{branch_compose, loop_compose, sequential_compose};

// ── Retired Kani notice ────────────────────────────────────────────────────
//
// Retired by vb-dzibx: the Kani harness modules contain fixed budget shapes
// and are not production-bound proof evidence. Keep the source files for
// future repair, but do not compile/register them as active Kani harnesses.
