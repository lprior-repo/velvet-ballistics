// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for ipc_strict_admission Verus spec
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `ipc_strict_admission.rs` Verus spec. It is a hand-written structural mirror
// of the canonical production admission surface for REFINE-IPC-001 at
// `crates/vb_runtime/src/ipc_refinement.rs:21-29, 34-36, 123-134`.
//
// The substitutions relative to direct `#[path]` inclusion of the
// production source are documented in the companion extern file
// (`verification/verus/extern_ipc_strict_admission.rs`) header. In
// summary, the production `ipc_refinement.rs` source depends on
// `vb_core`, `crate::admission`, `crate::shard`, and serde derives
// that cannot be resolved in a single-file Verus unit under the "no
// installs / no production changes" constraints. The mirror preserves
// the production field names and discriminant shape so spec reasoning
// matches production semantics.
//
// DRIFT POLICY: This file MUST be regenerated from the production
// source whenever production changes. The mirror is annotated at the
// top of every section with the originating production line range so
// regeneration is mechanical.
//
// This file is included by the companion extern file
// (`verification/verus/extern_ipc_strict_admission.rs`) via `#[path]`.
// Each production method body is marked `#[verifier::external]` so the
// body is opaque to Verus while the signature participates in the
// `assume_specification` binding in the companion spec file
// `ipc_strict_admission.rs`.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `StrictAdmissionRefinement` (struct)             <- crates/vb_runtime/src/ipc_refinement.rs:21-29
//     - `artifact_digest_matches: bool`               <- crates/vb_runtime/src/ipc_refinement.rs:24
//     - `run_id_matches: bool`                        <- crates/vb_runtime/src/ipc_refinement.rs:26
//     - `policy_matches: bool`                        <- crates/vb_runtime/src/ipc_refinement.rs:28
//   - `StrictAdmissionRefinement::is_refined`         <- crates/vb_runtime/src/ipc_refinement.rs:34-36
//   - `evidence_complete_projection`                  <- derived projection
//     (collapse of `run_id_matches && policy_matches`)
//   - `strict_admission_refinement`                   <- crates/vb_runtime/src/ipc_refinement.rs:123-134
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_strict_admission.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// Mirror of production `StrictAdmissionRefinement`
// ============================================================================
//
// Mirror of production `vb_runtime::ipc_refinement::StrictAdmissionRefinement`
// at `crates/vb_runtime/src/ipc_refinement.rs:21-29`. All 3 production
// fields are mirrored verbatim:
//   - `artifact_digest_matches: bool`  (line 24)
//   - `run_id_matches: bool`           (line 26)
//   - `policy_matches: bool`           (line 28)
//
// `PartialEq` and `Eq` derives are intentionally dropped (Verus does
// not support `core::intrinsics::discriminant_value` derivation
// without `external_type_specification`); the spec only compares
// individual fields, never whole structs, so `PartialEq` is not
// exercised by the proof obligations.
#[derive(Clone, Copy)]
pub struct StrictAdmissionRefinement {
    /// Mirror of production `artifact_digest_matches: bool` (line 24).
    pub artifact_digest_matches: bool,
    /// Mirror of production `run_id_matches: bool` (line 26).
    pub run_id_matches: bool,
    /// Mirror of production `policy_matches: bool` (line 28).
    pub policy_matches: bool,
}

// ============================================================================
// Production exec wrappers — `#[verifier::external]`
// ============================================================================

/// Mirror of `StrictAdmissionRefinement::is_refined` at
/// `crates/vb_runtime/src/ipc_refinement.rs:34-36`.
///
/// Production semantics (line 35):
///   `self.artifact_digest_matches && self.run_id_matches && self.policy_matches`
#[verifier::external]
pub fn is_refined(r: &StrictAdmissionRefinement) -> bool {
    r.artifact_digest_matches && r.run_id_matches && r.policy_matches
}

/// Pure spec projection: a refinement carries "required evidence" iff
/// both the run-id and policy booleans agree. This collapses the
/// 3-boolean production surface to the 2-boolean spec surface used by
/// the existing proof fns.
#[verifier::external]
pub fn evidence_complete_projection(r: &StrictAdmissionRefinement) -> bool {
    r.run_id_matches && r.policy_matches
}

/// Mirror of `strict_admission_refinement` at
/// `crates/vb_runtime/src/ipc_refinement.rs:123-134`.
///
/// Production body (lines 129-133):
///   ```ignore
///   StrictAdmissionRefinement {
///       artifact_digest_matches: admission.artifact_digest() == expected_digest,
///       run_id_matches:          admission.run_id()          == expected_run,
///       policy_matches:          admission.policy()          == expected_policy,
///   }
///   ```
#[verifier::external]
pub fn strict_admission_refinement(
    artifact_digest_matches: bool,
    run_id_matches: bool,
    policy_matches: bool,
) -> StrictAdmissionRefinement {
    StrictAdmissionRefinement {
        artifact_digest_matches,
        run_id_matches,
        policy_matches,
    }
}
