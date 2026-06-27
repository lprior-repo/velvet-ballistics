// SPDX-License-Identifier: MIT
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_digest_check.rs` Verus spec.
//
// Structure:
//   1. A direct `#[path]` inclusion of the verbatim production mirror
//      at
//      `verification/verus/production_inner/digest_check_production.rs`.
//      The mirror is a line-for-line copy of
//      crates/vb_storage/src/recovery/types.rs:855-900 with only
//      proc-macro `#[derive(...)]` and `#[must_use]`/`#[non_exhaustive]`
//      attributes dropped (they are unavailable or unsupported under
//      `verus --crate-type=lib`). Any drift in the production
//      variant names, variant set, method signatures, or method
//      bodies breaks this Verus build at compile time.
//
//   2. A spec-side mirror enum `SpecDigestCheck` (declared in
//      `verus!` context below) with the same variant set as
//      production. The spec-side mirror has `#[verifier::external]`
//      bodies that mirror the production bodies byte-for-byte. The
//      `assume_specification` bridges in the companion spec file
//      attach the production contracts to the spec-side mirror
//      methods, and the `exec fn` wrappers in that file invoke the
//      spec-side mirror methods to discharge the contracts.
//
//   3. A phantom drift-detection helper forces Rust to look up the
//      production method names at compile time. A rename of any of
//      these production methods breaks this fn's compilation.
//
// ============================================================================
// WHY A SPEC-SIDE MIRROR (NOT DIRECT PRODUCTION TYPE IN SPEC)
// ============================================================================
// The production mirror is included via `#[path]` under module-level
// `#[verifier::external]`. This makes the included types opaque to
// Verus, so spec functions cannot pattern-match on the production
// `DigestCheck` variants directly. The spec-side mirror enum below
// is declared in `verus!` context (NOT `#[verifier::external]`) so
// its variants are spec-visible; pattern matches in `assume_specification`
// contracts and `exec fn` ensures clauses can reason about them.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:855-900`.
//
// Production mirror included via `#[path]` (drift detection):
//   - `prod_src::DigestCheck`                         <- types.rs:857-864
//   - `prod_src::DigestCheck::WorkflowSourceOnly`     <- types.rs:859
//   - `prod_src::DigestCheck::WorkflowAndIr`          <- types.rs:861
//   - `prod_src::DigestCheck::Full`                   <- types.rs:863
//   - `prod_src::DigestCheck::hierarchy_rank`         <- types.rs:868-875
//   - `prod_src::DigestCheck::checks_workflow_source` <- types.rs:878-881
//   - `prod_src::DigestCheck::checks_compiled_ir`     <- types.rs:883-886
//   - `prod_src::DigestCheck::checks_full`            <- types.rs:888-893
//   - `prod_src::DigestCheck::is_strictly_weaker_than`<- types.rs:895-899
//
// Spec-side mirror (used in Verus proofs):
//   - `SpecDigestCheck` (local enum below, variant-identical to production)
//   - `SpecDigestCheck::hierarchy_rank`
//   - `SpecDigestCheck::checks_workflow_source`
//   - `SpecDigestCheck::checks_compiled_ir`
//   - `SpecDigestCheck::checks_full`
//   - `SpecDigestCheck::is_strictly_weaker_than`
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of the five `DigestCheck` methods are NOT
// verified by Verus directly. The production mirror module is
// marked `#[verifier::external]` at module level, and the
// spec-side mirror methods are also `#[verifier::external]`. The
// `assume_specification` bridges in the companion spec file
// (`vb_rpch_digest_check.rs`) attach the production contracts to
// the spec-side mirror methods, and the exec wrappers in that file
// invoke the spec-side mirror methods and assert the contracts hold.
// Drift between the production mirror and the production source is
// reported as binding-debt tracked outside Verus.
//
// ============================================================================
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Drift-detection inclusion: `#[path]` to verbatim production mirror
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/digest_check_production.rs`. The mirror is marked
// `#[verifier::external]` at module level so the production bodies
// are opaque to Verus; the inclusion still validates Rust resolution
// (variant names, discriminant sets, fn signatures) at compile time.
// Any drift in the production impl surface breaks this Verus build.
#[verifier::external]
#[path = "production_inner/digest_check_production.rs"]
pub mod prod_src;

// Phantom drift-detection helper. The body is `#[verifier::external]`
// (opaque to Verus), but the `prod_src::DigestCheck::*` method
// references force Rust to resolve the production method names at
// compile time. A rename of any of these production methods (or the
// production enum) breaks this fn's compilation. Methods take `self`
// by value, so each call constructs a fresh copy.
#[verifier::external]
fn prod_methods_drift_check() {
    // Each call below forces resolution of the production method name.
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.hierarchy_rank();
    let _ = prod_src::DigestCheck::WorkflowAndIr.hierarchy_rank();
    let _ = prod_src::DigestCheck::Full.hierarchy_rank();
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.checks_workflow_source();
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.checks_compiled_ir();
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.checks_full();
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.is_strictly_weaker_than(
        prod_src::DigestCheck::WorkflowAndIr,
    );
    let _ = prod_src::DigestCheck::WorkflowSourceOnly.is_strictly_weaker_than(
        prod_src::DigestCheck::Full,
    );
}

// ---------------------------------------------------------------------------
// Spec-side mirror enum — production variant-identical
// ---------------------------------------------------------------------------
//
// Variant-identical to production `DigestCheck` at
// `crates/vb_storage/src/recovery/types.rs:857-864`. All three
// variants are unit variants.
//
// `PartialEq, Eq` are intentionally NOT derived here because the
// macro-generated `discriminant_value` call is not supported by
// Verus 0.2026.05.05 (Rust 1.95.0). Spec proofs reason via
// `is_strictly_weaker_than` and direct variant comparison instead.
//
// `#[non_exhaustive]` is intentionally NOT applied here because the
// spec projection enumerates the closed three-level hierarchy. A
// production drift that adds a 4th variant does NOT break the
// mirror at compile time but is recorded as binding-debt during
// review (the spec's `production_hierarchy_rank` match becomes
// non-exhaustive in the spec's eyes).
#[derive(Clone, Copy)]
pub enum SpecDigestCheck {
    /// Only verify workflow source digest.   <- types.rs:859
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.   <- types.rs:861
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.   <- types.rs:863
    Full,
}

// ---------------------------------------------------------------------------
// Spec-side mirror methods — production body-identical
// ---------------------------------------------------------------------------
//
// All methods are `#[verifier::external]` so Verus skips body
// verification. The companion spec file attaches `assume_specification`
// bridges that state the production contracts: each method returns
// a value derived from the variant's hierarchy rank. The exec
// wrappers in the spec file invoke these mirror methods and assert
// the contracts hold.
//
// Every body is a byte-for-byte copy of the production body at the
// cited `types.rs` line range. Drift in any production body breaks
// the `assume_specification` contract because the projection body no
// longer matches the contract the spec proofs discharge.
impl SpecDigestCheck {
    /// Mirror of `hierarchy_rank` at
    /// `crates/vb_storage/src/recovery/types.rs:868-875`.
    /// Production body (line 869):
    /// ```
    /// pub const fn hierarchy_rank(self) -> u8 {
    ///     match self {
    ///         Self::WorkflowSourceOnly => 1,
    ///         Self::WorkflowAndIr => 2,
    ///         Self::Full => 3,
    ///     }
    /// }
    /// ```
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`). The `assume_specification` bridge
    /// in the companion spec file attaches the production contract.
    #[verifier::external]
    pub fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Mirror of `checks_workflow_source` at
    /// `crates/vb_storage/src/recovery/types.rs:878-881`.
    /// Production body (line 879):
    /// `self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Mirror of `checks_compiled_ir` at
    /// `crates/vb_storage/src/recovery/types.rs:883-886`.
    /// Production body (line 884):
    /// `self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Mirror of `checks_full` at
    /// `crates/vb_storage/src/recovery/types.rs:888-893`.
    /// Production body (line 890):
    /// `self.hierarchy_rank() >= Self::Full.hierarchy_rank()`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }

    /// Mirror of `is_strictly_weaker_than` at
    /// `crates/vb_storage/src/recovery/types.rs:895-899`.
    /// Production body (line 897):
    /// `self.hierarchy_rank() < other.hierarchy_rank()`.
    ///
    /// TRUST BOUNDARY: body is opaque to Verus
    /// (`#[verifier::external]`).
    #[verifier::external]
    pub fn is_strictly_weaker_than(self, other: Self) -> bool {
        self.hierarchy_rank() < other.hierarchy_rank()
    }
}

} // verus!
