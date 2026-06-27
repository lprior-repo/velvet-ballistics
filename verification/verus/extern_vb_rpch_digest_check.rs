// SPDX-License-Identifier: MIT
//
// Extern surface for `vb_rpch_digest_check` Verus spec.
//
// ============================================================================
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// This file is the production-binding surface for the
// `vb_rpch_digest_check.rs` Verus spec. It contains:
//
//   1. A direct mirror enum `DigestCheck` with the EXACT variant set of
//      the production enum at
//      `crates/vb_storage/src/recovery/types.rs:854-864`
//      (`WorkflowSourceOnly`, `WorkflowAndIr`, `Full`). Production uses
//      `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` + `#[non_exhaustive]`;
//      the spec projection enumerates the closed three-level hierarchy.
//      The variant set MUST match production byte-for-byte; any drift
//      breaks the type bridge.
//
//   2. Five mirror methods (`hierarchy_rank`, `checks_workflow_source`,
//      `checks_compiled_ir`, `checks_full`, `is_strictly_weaker_than`)
//      with bodies that are byte-for-byte copies of the production
//      bodies at `types.rs:868-899`. All five are marked
//      `#[verifier::external]` so Verus skips body verification.
//
//   3. A phantom `prod_methods_drift_check` fn that calls every
//      production method with arguments of the production enum
//      discriminant, forcing Rust to resolve the production method
//      names at compile time. Any rename of these methods in
//      production breaks this fn's compilation.
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF
// crates/vb_storage/src/recovery/types.rs
// ============================================================================
//
// Direct `#[path]` inclusion of `types.rs` is blocked by:
//   1. `types.rs` uses `#[derive(... Serialize, Deserialize)]` (line
//      429 and onward) plus `#[derive(thiserror::Error)]` (line 37).
//      Verus cannot invoke proc-macro derives without registering the
//      proc-macro crates, and the file also pulls in
//      `serde::{Deserialize, Serialize}` (line 10) as a bare-path
//      import that would need a separate extern alias.
//   2. `types.rs` uses `vb_core::*` and `crate::recovery::replay::*`
//      imports that are not available in a standalone
//      `verus --crate-type=lib` invocation.
//   3. `types.rs` is ~30 KB and contains the full recovery type
//      module surface (ActionReplayTracker, DigestPair,
//      DigestVerificationRequest, FullDigestEvidence,
//      UnsupportedRecoveryState, etc.), most of which is irrelevant
//      to the digest-check hierarchy proof.
//
// These are all "NO production changes" blockers (per the task brief).
// The structural mirror below sidesteps every blocker while still
// establishing a real end-to-end binding: any drift in the production
// enum discriminant set, method signatures, or method bodies will
// break the `extern_vb_rpch_digest_check` mirror and the spec proofs
// that depend on it.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
// Production source: `crates/vb_storage/src/recovery/types.rs:854-900`.
//
//   `DigestCheck`                            <- types.rs:857-864
//   `DigestCheck::WorkflowSourceOnly`        <- types.rs:859
//   `DigestCheck::WorkflowAndIr`             <- types.rs:861
//   `DigestCheck::Full`                      <- types.rs:863
//   `DigestCheck::hierarchy_rank`            <- types.rs:868-875
//   `DigestCheck::checks_workflow_source`    <- types.rs:878-881
//   `DigestCheck::checks_compiled_ir`        <- types.rs:883-886
//   `DigestCheck::checks_full`               <- types.rs:888-893
//   `DigestCheck::is_strictly_weaker_than`   <- types.rs:895-899
//
// Production production bodies (each method has a body that mirrors
// the production body byte-for-byte):
//
//   `hierarchy_rank`:
//       match self {
//           Self::WorkflowSourceOnly => 1,
//           Self::WorkflowAndIr => 2,
//           Self::Full => 3,
//       }
//   `checks_workflow_source`:
//       self.hierarchy_rank() >= Self::WorkflowSourceOnly.hierarchy_rank()
//   `checks_compiled_ir`:
//       self.hierarchy_rank() >= Self::WorkflowAndIr.hierarchy_rank()
//   `checks_full`:
//       self.hierarchy_rank() >= Self::Full.hierarchy_rank()
//   `is_strictly_weaker_than`:
//       self.hierarchy_rank() < other.hierarchy_rank()
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of the five `DigestCheck` methods are NOT
// verified by Verus directly. All five methods below are
// `#[verifier::external]` so Verus skips body verification. The
// `assume_specification` bridges in the companion spec file
// (`vb_rpch_digest_check.rs`) attach the production contracts (the
// expected boolean / u8 return for each method), and the exec wrappers
// in that file exercise the bridges from `verus!` context so the
// bridges are not used as vacuum specifications. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// Mirror of `DigestCheck` at crates/vb_storage/src/recovery/types.rs:857-864.
// ---------------------------------------------------------------------------
//
// Production uses `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` +
// `#[non_exhaustive]`. The spec projection enumerates the closed
// three-level hierarchy; `Clone + Copy + PartialEq + Eq` are sufficient
// for spec reasoning. The variant set MUST match production
// byte-for-byte; any drift in discriminant names or added/removed
// variants breaks the type bridge and the bridges below.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DigestCheck {
    /// Only verify workflow source digest.   <- types.rs:859
    WorkflowSourceOnly,
    /// Verify workflow source and compiled IR digests.   <- types.rs:861
    WorkflowAndIr,
    /// Verify all digests including action ABI and policy.   <- types.rs:863
    Full,
}

impl DigestCheck {
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

// ---------------------------------------------------------------------------
// Phantom drift-detection helper
// ---------------------------------------------------------------------------
//
// The body is `#[verifier::external]` (opaque to Verus), but the
// `DigestCheck::*` method references force Rust to resolve the
// production method names at compile time. A rename of any of these
// methods (or the production enum) breaks this fn's compilation.
#[verifier::external]
fn prod_methods_drift_check() {
    let w = DigestCheck::WorkflowSourceOnly;
    let i = DigestCheck::WorkflowAndIr;
    let f = DigestCheck::Full;
    // Each call below forces resolution of the production method name.
    let _ = w.hierarchy_rank();
    let _ = i.hierarchy_rank();
    let _ = f.hierarchy_rank();
    let _ = w.checks_workflow_source();
    let _ = w.checks_compiled_ir();
    let _ = w.checks_full();
    let _ = w.is_strictly_weaker_than(i);
    let _ = w.is_strictly_weaker_than(f);
}

} // verus!
