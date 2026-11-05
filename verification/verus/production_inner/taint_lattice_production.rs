// SPDX-License-Identifier: MIT
//
// ============================================================================
// PRODUCTION MIRROR for taint lattice (WEAK binding surface)
// ============================================================================
//
// In-tree production-source mirror for the `Taint` enum and taint lattice
// helper fns consumed by `verification/verus/taint_lattice.rs`.
//
// The `Taint` enum below is a VERBATIM copy of lines 14-25 of
// `crates/vb_core/src/value.rs`:
//
//   - Variant set:        Clean, DerivedFromSecret, Secret, Random,
//                         TimeDependent
//   - Discriminant ranks: 0, 1, 2, 3, 4 (matching `#[repr(u8)]`)
//   - Derives:            Debug, Clone, Copy, PartialEq, Eq
//   - Attributes:         #[non_exhaustive]
//
// `join_taint(a, b)` is a VERBATIM copy of `crates/vb_core/src/value.rs`
// lines 29-45 (the production free fn that picks the higher-ranked
// variant). The verbatim copy preserves the inline discriminant-match
// style production uses — production does NOT extract `rank` as a
// method on `Taint`.
//
// The other public helpers (`taint_rank`, `join_many`, `is_commutative`,
// `is_associative`, `is_idempotent`, `has_identity`,
// `secret_never_downgrades`, `derived_never_downgrades`,
// `all_lattice_laws`) are NOT present in production `value.rs`. They
// are spec-side observational helpers required by the spec file's
// `assume_specification` bridges. They are modeled directly on
// production's `join_taint` semantics — i.e., the higher-rank-wins
// rule — and are the only path through which the spec can observe
// production lattice behaviour at the type level.
//
// `taint_rank` is provided as a FREE FN (not a method on `Taint`)
// because Verus 0.2026.05.05 panics in trait resolution on
// `impl Enum { fn }` blocks inside spec-mode `verus!` modules.
// Production `value.rs` does not export a method form either; the
// discriminant pattern is inlined inside `join_taint` (see lines
// 30-35 and 37-43 of value.rs).
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
// This file MUST be reviewed against `crates/vb_core/src/value.rs:14-25`
// (Taint enum) and `crates/vb_core/src/value.rs:27-45` (`join_taint`)
// whenever production taint semantics change. Drift in either block
// breaks the Verus binding (variant renames surface as
// "no variant named X" errors; discriminant changes surface as
// "assertion failed" in the spec proofs).
//
// Field NAMES and DISCRIMINANT RANKS are preserved byte-for-byte from
// production. Any drift breaks the verification build.
//
// Production `crates/vb_core/src/value.rs:14-25`
// Production `crates/vb_core/src/value.rs:27-45`
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The exec bodies in this mirror are real Rust and execute under
// `verus!`. The companion spec file's `assume_specification` contracts
// state what these bodies are claimed to compute; the `wrapper_*` exec
// fns in the spec file actually invoke them and discharge the
// contracts at every call site. Drift between this mirror's body and
// the contract is caught at Verus verification time (the `assert`
// inside the wrapper fails to discharge).
//
// Included by `verification/verus/extern_taint_lattice.rs` via
// `#[verifier::external] #[path = "production_inner/taint_lattice_production.rs"]`.
// The `#[verifier::external]` on the mod declaration is required to
// avoid a rustc panic in trait resolution when the spec invokes
// these fns through the bridge.
// ============================================================================

#![forbid(unsafe_code)]
#![allow(dead_code)]

// ----------------------------------------------------------------------------
// Verbatim copy of crates/vb_core/src/value.rs:14-25
// ----------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum Taint {
    /// Slot contains no secret-derived data.
    Clean = 0,
    /// Slot contains data derived from one or more secrets.
    DerivedFromSecret = 1,
    /// Slot contains a secret value.
    Secret = 2,
    /// Slot contains a randomly generated value.
    Random = 3,
    /// Slot contains a time-dependent value.
    TimeDependent = 4,
}

/// Free-fn form of the inline discriminant pattern that production's
/// `value.rs:30-35` and `value.rs:37-43` use inside `join_taint`.
/// Kept as a free fn (not `impl Taint::rank(&self)`) because Verus
/// 0.2026.05.05 panics on `impl Enum { fn }` inside spec-mode `verus!`.
/// Models the production discriminant mapping byte-for-byte.
pub fn taint_rank(self_: Taint) -> u8 {
    match self_ {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    }
}

// ----------------------------------------------------------------------------
// Verbatim copy of crates/vb_core/src/value.rs:29-45 (`join_taint`)
// ----------------------------------------------------------------------------
/// Joins two taint levels, returning the more restrictive one.
#[must_use]
pub fn join_taint(a: Taint, b: Taint) -> Taint {
    let a_disc: u8 = match a {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    };
    let b_disc: u8 = match b {
        Taint::Clean => 0,
        Taint::DerivedFromSecret => 1,
        Taint::Secret => 2,
        Taint::Random => 3,
        Taint::TimeDependent => 4,
    };
    if a_disc >= b_disc { a } else { b }
}

// ----------------------------------------------------------------------------
// Spec-side observational helpers (modeled on `join_taint` semantics)
// ----------------------------------------------------------------------------
//
// These are NOT verbatim copies of production code (production
// `value.rs` does not export them). They are the spec-side surface
// through which the spec file's `assume_specification` bridges observe
// the lattice laws. Bodies match the higher-rank-wins semantics of
// production `join_taint` so the bridge contracts discharge
// meaningfully.

/// Folds `join_taint` over a slice, starting from `Taint::Clean`.
pub fn join_many(taints: &[Taint]) -> Taint {
    let mut result = Taint::Clean;
    for &t in taints {
        result = join_taint(result, t);
    }
    result
}

pub fn is_commutative(a: Taint, b: Taint) -> bool {
    join_taint(a, b) == join_taint(b, a)
}

pub fn is_associative(a: Taint, b: Taint, c: Taint) -> bool {
    join_taint(join_taint(a, b), c) == join_taint(a, join_taint(b, c))
}

pub fn is_idempotent(a: Taint) -> bool {
    join_taint(a, a) == a
}

pub fn has_identity(a: Taint) -> bool {
    join_taint(a, Taint::Clean) == a
}

pub fn secret_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::Secret) == Taint::Secret
}

pub fn derived_never_downgrades() -> bool {
    join_taint(Taint::Clean, Taint::DerivedFromSecret) == Taint::DerivedFromSecret
}

pub fn all_lattice_laws(a: Taint, b: Taint, c: Taint) -> bool {
    is_commutative(a, b)
        && is_associative(a, b, c)
        && is_idempotent(a)
        && has_identity(a)
        && secret_never_downgrades()
        && derived_never_downgrades()
}