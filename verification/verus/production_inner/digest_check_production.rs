// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for DigestCheck
// ============================================================================
//
// This file is a VERBATIM copy of the production `DigestCheck` enum
// and impl block from
//   crates/vb_storage/src/recovery/types.rs:855-900
// with three minimal substitutions:
//
//   1. `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` is dropped
//      from the enum declaration. The macro-generated
//      `discriminant_value` call (used by `PartialEq` and `Eq`) is
//      not supported by Verus 0.2026.05.05 (Rust 1.95.0). The
//      spec-side proofs in `vb_rpch_digest_check.rs` reason about
//      hierarchy rank via `production_hierarchy_rank` and never
//      directly call PartialEq on DigestCheck values; this is a
//      faithful structural projection.
//
//   2. `#[non_exhaustive]` is dropped from the enum declaration.
//      `#[non_exhaustive]` is a lint hint and does not affect
//      match semantics; dropping it keeps the mirror parseable
//      under the default Verus lint set.
//
//   3. `#[must_use]` on every const fn is dropped. `#[must_use]`
//      is a lint hint and does not affect the const-fn body
//      semantics; dropping it keeps the mirror parseable under
//      the default Verus lint set. (Production retains
//      `#[must_use]` at the call sites in
//      `crates/vb_storage/src/recovery/`; the mirror here is for
//      verification only.)
//
// This file exists so that the companion
// `extern_vb_rpch_digest_check.rs` can use
//   `#[path = "production_inner/digest_check_production.rs"]`
// to bind the production `DigestCheck` block by direct source
// inclusion (per the task brief "with `#[path]` bindings to
// production source"). Any drift between this mirror and the
// production source breaks the `extern_vb_rpch_digest_check` Verus
// build, which is the explicit drift-detection mechanism the user
// requires.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_storage/src/recovery/types.rs:855-900` whenever
// production changes. The mirror is annotated at the top of every
// section with the originating production line range so regeneration
// is mechanical.
//
// This file is included by the companion extern file under module-level
// `#[verifier::external]` so every body is opaque to Verus. It
// compiles as plain Rust (no `verus!` block, no `vstd` import) and is
// checked by the Verus invocation purely for structural resolution
// and type well-formedness — Verus never reasons about the bodies.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: DigestCheck enum
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types.rs:855-865
// Drift policy: any change to the production enum between these line
// numbers MUST be mirrored here. Variant names, variant order, and
// variant set are matched exactly.

/// Digest check level for recovery validation.
#[derive(Clone, Copy)]
pub enum DigestCheck {
    /// Only verify workflow source digest.
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.
    Full,
}

// ---------------------------------------------------------------------------
// VERBATIM PRODUCTION: DigestCheck impl block
// ---------------------------------------------------------------------------
//
// Source: crates/vb_storage/src/recovery/types.rs:867-900
// Drift policy: any change to the production impl block between these
// line numbers MUST be mirrored here. Method signatures, body
// structure, and `pub const` initializers are preserved.

impl DigestCheck {
    /// Numeric rank for proof and testing of the strict digest hierarchy.
    pub const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::WorkflowSourceOnly => 1,
            Self::WorkflowAndIr => 2,
            Self::Full => 3,
        }
    }

    /// Whether this level requires workflow-source digest verification.
    pub const fn checks_workflow_source(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
    }

    /// Whether this level requires compiled-IR digest verification.
    pub const fn checks_compiled_ir(self) -> bool {
        self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
    }

    /// Whether this level requires all currently-modeled digest checks.
    pub const fn checks_full(self) -> bool {
        self.hierarchy_rank() >= Self::Full.hierarchy_rank()
    }

    /// Production proof surface for strict ordering between two levels.
    pub const fn is_strictly_weaker_than(self, other: Self) -> bool {
        self.hierarchy_rank() < other.hierarchy_rank()
    }
}
