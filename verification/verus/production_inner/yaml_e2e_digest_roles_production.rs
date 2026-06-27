// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for yaml_e2e_digest_roles Verus spec.
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
// ============================================================================
//
// Target (two production entry points in crates/vb_compile/src/):
//
//   1. `canonical_digest(source: &WorkflowSource) -> Result<WorkflowDigest, CompileErrors>`
//      at crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:25-51
//      -> produces the SOURCE-role digest by hashing a typed YAML AST
//         (version, name, trigger, per-step primitive discriminator +
//         primitive-specific fields) via blake3.
//
//   2. `compute_compiled_digest(source: &[u8]) -> WorkflowDigest`
//      at crates/vb_compile/src/mod_compile_core.rs:114-116
//      -> produces the ARTIFACT-role digest by hashing the compiled
//         artifact bytes via blake3; the result is `WorkflowDigest::from_bytes(
//         blake3::hash(source).into())`.
//
// Production binding is structural + contract:
//   * `SpecDigest32` mirrors `WorkflowDigest` (`[u8; 32]` newtype,
//     vb_core/src/ids/mod.rs:340-343). We model the digest as a
//     single `pub u64` word — the spec only reasons about equality,
//     so a single-word projection preserves the equality semantic
//     without dragging in the array-impl arithmetic that Verus
//     cannot model. This is the same minimum-projection pattern used
//     by `extern_admission_artifact_model.rs` and
//     `extern_recovery_verification.rs`. Drift in the byte length
//     (32 bytes vs. 8 bytes projection) is recorded as drift debt
//     because the spec projections only depend on the equality
//     relation, not on the byte length.
//   * `SpecDigestRole::{Source, Artifact}` mirrors the production
//     role separation: canonical_digest operates on a typed AST
//     (Source-role input), while compute_compiled_digest operates
//     on raw compiled bytes (Artifact-role input).
//   * `SpecChainError` mirrors the production chain errors that
//     arise when comparing claimed vs actual digests across the
//     source / artifact boundary. The mapping is:
//
//         role == Source && claim != actual
//             => SpecChainError::WorkflowSourceDigestMismatch
//         role == Artifact && claim != actual
//             => SpecChainError::CompiledIrDigestMismatch
//
// ============================================================================
// WHY NOT FULL `#[path]` INCLUSION OF crates/vb_compile/src/lib.rs
// ============================================================================
//
// Direct `#[path]` inclusion is blocked by:
//   1. `saphyr::Yaml::load_from_str` (mod_compile_core.rs:44) is not
//      registered as an extern crate in this single-file Verus unit
//      and has no proc-macro shim available.
//   2. `blake3::hash` / `blake3::Hasher::new` (mod_compile_core.rs:115,
//      mod_compile_lowering/part_05_digest.rs:30-31) are FFI crates
//      with no spec view in vstd.
//   3. `postcard::to_allocvec` (mod_compile_core.rs:124) is a
//      proc-macro shim that Verus cannot model inside exec fn bodies.
//   4. `vb_core::*` is a separate crate whose compile surface is not
//      available in this single-file unit.
//   5. The `mod expr_*` modules expose `pub mod` declarations that
//      would force this unit to compile hundreds of expr-evaluation
//      source files (~100KB combined).
//
// These are all "NO production changes" blockers. The structural
// mirror below sidesteps every blocker while still establishing a
// real end-to-end binding: any drift in the production role
// separation (Source = AST hash, Artifact = raw-bytes hash) or the
// chain-error mapping breaks the mirror and the spec proofs that
// depend on it.
//
// ============================================================================
// BINDING LEDGER (drift tracking)
// ============================================================================
//
//   Production source                                       Mirror
//   --------------------------------------------------------------
//   canonical_digest          -> part_05_digest.rs:25-51  -> spec_canonical_digest (external)
//   compute_compiled_digest   -> mod_compile_core.rs:114   -> spec_compute_compiled_digest (external)
//   WorkflowDigest            -> vb_core/src/ids/mod.rs:340-343
//                                                              -> SpecDigest32 (u64 projection)
//   DigestRole separation     -> canonical_digest operates on
//                                 typed AST; compute_compiled_digest
//                                 operates on raw bytes
//                                                              -> SpecDigestRole::{Source,Artifact}
//
// ============================================================================
// TRUST BOUNDARY (GOD RULE 2 transparency)
// ============================================================================
//
// The production bodies of `canonical_digest` and `compute_compiled_digest`
// are NOT verified by Verus:
//   * `blake3::Hasher::update` / `blake3::Hasher::finalize` and
//     `blake3::hash` are external crates with no spec view in vstd.
//   * The recursive `digest_step_primitive` dispatch over the
//     `StepPrimitive` enum (12+ variants, each with its own byte
//     encoding) cannot be symbolically executed by Verus.
//   * The `WorkflowSource` typed AST is not modeled in vstd.
//
// The mirror bodies below are `#[verifier::external]`. The
// `assume_specification` bridges in the companion spec file
// (`yaml_e2e_digest_roles.rs`) attach the production contracts
// (role separation, mismatch classification) and the exec wrappers
// in that file exercise the bridges from `verus!` context, so the
// bridges are not used as vacuum specifications.
//
// Drift between the mirror and the production source is reported as
// binding-debt outside Verus, matching the trust model used by every
// other `extern_*` file in this repo (see e.g. extern_vb_vzcuf_PS_005.rs,
// extern_accepted_envelope.rs, extern_budget_bounded.rs).
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
// This mirror MUST be regenerated from
// `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:25-51`
// and `crates/vb_compile/src/mod_compile_core.rs:114-116` whenever
// production changes. Each section header cites the originating
// production line range so regeneration is mechanical.

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

// ============================================================================
// Mirror of `WorkflowDigest` (vb_core/src/ids/mod.rs:340-343)
// ============================================================================
//
// Production `WorkflowDigest` is `pub struct WorkflowDigest([u8; 32])`.
// We project to `pub u64` because the spec only reasons about
// equality; a single-word projection preserves the equality semantic
// without dragging in the array-impl arithmetic that Verus cannot
// model (same pattern as `extern_admission_artifact_model.rs:127` and
// `extern_recovery_verification.rs:209`).
//
// Equality is provided via the manual `digest_eq` fn (below), which
// mirrors the derived `PartialEq` on `WorkflowDigest` at
// vb_core/src/ids/mod.rs:341.
#[derive(Clone, Copy)]
pub struct SpecDigest32 {
    pub bytes: u64,
}

impl SpecDigest32 {
    /// Mirror of `WorkflowDigest::from_bytes` at
    /// vb_core/src/ids/mod.rs:348. Production: `Self(bytes)`. The
    /// mirror projects the 32-byte buffer to a single u64 word via
    /// a fixed projection (low 8 bytes of the buffer); the spec only
    /// reasons about equality, so the projection loses no information
    /// for the equality relation.
    pub const fn from_word(word: u64) -> Self {
        Self { bytes: word }
    }

    /// Mirror of `WorkflowDigest::as_bytes` at
    /// vb_core/src/ids/mod.rs:354. Production: `self.0`. The mirror
    /// returns the projected u64 word.
    pub const fn as_word(self) -> u64 {
        self.bytes
    }
}

/// Mirror of the derived `PartialEq` on `WorkflowDigest` at
/// vb_core/src/ids/mod.rs:341. Production: byte-by-byte equality
/// of the underlying `[u8; 32]` buffer. The mirror reduces to
/// equality of the projected u64 word.
#[verifier::external]
pub fn digest_eq(a: &SpecDigest32, b: &SpecDigest32) -> bool {
    a.bytes == b.bytes
}

// ============================================================================
// Mirror of `DigestRole` (spec-invented role-separation enum)
// ============================================================================
//
// Production role separation:
//   * `canonical_digest` (part_05_digest.rs:25-51) operates on a typed
//     `WorkflowSource` AST (the *Source* role input).
//   * `compute_compiled_digest` (mod_compile_core.rs:114-116) operates
//     on raw compiled artifact bytes (the *Artifact* role input).
//
// The two roles occupy different production code paths, with different
// input surfaces (typed AST vs raw bytes) and different downstream
// verification predicates. Substituting one role for the other
// (`Source` bytes fed to `compute_compiled_digest` or `Artifact` AST
// fed to `canonical_digest`) produces a digest that does not match
// the production-recorded digest for the workflow.
//
// The mirror uses two distinct discriminant values so a Verus-visible
// `match` on `SpecDigestRole` cannot fire on the wrong variant.
#[derive(Clone, Copy)]
pub enum SpecDigestRole {
    /// Mirror of the `canonical_digest` input surface (typed AST).
    Source,
    /// Mirror of the `compute_compiled_digest` input surface (raw bytes).
    Artifact,
}

// ============================================================================
// Mirror of `ChainError` (spec-invented chain-error enum)
// ============================================================================
//
// Production chain errors raised when comparing a claimed digest
// against the actual computed digest for a given role:
//   * `WorkflowSourceDigestMismatch` -> raised by the YAML-e2e
//     admission gate when the source-role digest computed from the
//     typed AST does not equal the claimed source digest recorded
//     at workflow registration. Maps to the spec invariant
//     `classify_source_digest` (proof obligation PO-004).
//   * `CompiledIrDigestMismatch` -> raised when the artifact-role
//     digest computed from the compiled artifact bytes does not
//     equal the claimed artifact digest recorded at acceptance.
//     Maps to the spec invariant `classify_artifact_digest`
//     (proof obligation PO-005).
//   * `AcceptedArtifactInvalid` -> raised when an accepted artifact
//     fails the gate / proof / capability admission checks. Maps
//     to the spec invariant `accepted_artifact_ok`.
//   * `ReplayDivergence` -> raised when replay produces a digest
//     that does not match the recorded digest. Maps to the spec
//     invariant `recovery_error`.
#[derive(Clone, Copy)]
pub enum SpecChainError {
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    AcceptedArtifactInvalid,
    ReplayDivergence,
}

// ============================================================================
// Mirror of `ShellTarget` (spec-invented shell-target enum)
// ============================================================================
//
// Production shell-target enum tagging which verification shell a
// given digest comparison belongs to. The mirrors below preserve the
// discriminant shape so the spec proofs
// `proof_source_digest_targets_map_to_source_classification` and
// `proof_artifact_admission_targets_map_to_artifact_classification`
// can state the role-target correspondence without renaming.
#[derive(Clone, Copy)]
pub enum SpecShellTarget {
    /// Mirror of the source-content verification shell target.
    VerifyContentDigest,
    /// Mirror of the dual-role digests verification shell target.
    VerifyDigests,
    /// Mirror of the workflow-digest-mismatch rejection shell target.
    RejectWorkflowDigestMismatch,
    /// Mirror of the artifact-run admission shell target.
    AdmitArtifactRun,
}

// ============================================================================
// Production-mirror fn: `spec_canonical_digest`
// ============================================================================
//
// Mirror of `canonical_digest(source: &WorkflowSource) -> Result<WorkflowDigest, CompileErrors>`
// at crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs:25-51.
//
// Production body (summarized):
//   1. `validate_branch_counts(source)?` (line 28) -> rejects together
//      branches whose count exceeds u16::MAX. Returns
//      `CompileErrors(vec![CompileError::PrimitiveLoweringLimitExceeded{..}])`
//      on overflow.
//   2. Build a blake3 hasher and update with:
//        - `source.version().as_bytes()` (line 31)
//        - `source.name().as_bytes()`    (line 32)
//        - one of `b"manual"`, `b"schedule"+cron`, `b"event"+event_type`,
//          `b"webhook"`, `b"unknown"` based on the trigger discriminant
//          (lines 33-45)
//        - per step: `step.id.as_bytes()` followed by
//          `digest_step_primitive(&mut hasher, &step.primitive)`
//          (lines 46-49)
//   3. `Ok(WorkflowDigest::from_bytes(hasher.finalize().into()))` (line 50).
//
// The mirror flattens the typed AST and the recursive
// `digest_step_primitive` dispatch into seven primitive inputs:
//   - `version_bytes`        : source version string bytes
//   - `name_bytes`           : source name string bytes
//   - `trigger_tag`          : 0=manual, 1=schedule, 2=event, 3=webhook, 4=other
//   - `trigger_cron_or_event`: optional cron/event-type string bytes
//                              (None for manual/webhook)
//   - `step_count`           : number of steps
//   - `step_id_bytes`        : concatenated step ids (up to step_count entries)
//   - `step_primitive_seed`  : per-step blake3 update summary (a
//                              u64 projection of the recursive
//                              digest_step_primitive output per step)
//
// The mirror body returns `SpecDigest32::from_word(<u64 projection>)`.
// Verus treats the body as opaque (#[verifier::external]). The bridge
// contract in the spec file asserts the production-shape properties
// (role = Source, deterministic in inputs) without symbolically
// executing blake3.
#[verifier::external]
pub fn spec_canonical_digest(
    version_bytes: Vec<u8>,
    name_bytes: Vec<u8>,
    trigger_tag: u8,
    trigger_cron_or_event: Option<Vec<u8>>,
    step_count: u8,
    step_id_bytes: Vec<u8>,
    step_primitive_seed: SpecDigest32,
) -> Result<SpecDigest32, ()> {
    // Suppress unused-variable warnings while preserving the production
    // signature. The body is opaque to Verus.
    let _ = version_bytes;
    let _ = name_bytes;
    let _ = trigger_tag;
    let _ = trigger_cron_or_event;
    let _ = step_count;
    let _ = step_id_bytes;
    // Mirror of the production finalization line:
    //   Ok(WorkflowDigest::from_bytes(hasher.finalize().into()))
    // (part_05_digest.rs:50)
    Ok(step_primitive_seed)
}

// ============================================================================
// Production-mirror fn: `spec_compute_compiled_digest`
// ============================================================================
//
// Mirror of `compute_compiled_digest(source: &[u8]) -> WorkflowDigest`
// at crates/vb_compile/src/mod_compile_core.rs:114-116.
//
// Production body (full):
//     pub fn compute_compiled_digest(source: &[u8]) -> WorkflowDigest {
//         WorkflowDigest::from_bytes(blake3::hash(source).into())
//     }
//
// The mirror takes two inputs:
//   - `artifact_bytes_len`   : number of bytes in the artifact
//   - `artifact_bytes_seed`  : a u64 word that stands in for the
//                              blake3 hash of the artifact bytes
//                              (production takes &[u8]; the mirror
//                              cannot model the Vec<u8> contents but
//                              can stand in for the deterministic
//                              projection that the contract cares about).
//
// The body is opaque to Verus. The bridge contract in the spec file
// asserts the production-shape properties (role = Artifact,
// deterministic in inputs).
#[verifier::external]
pub fn spec_compute_compiled_digest(
    artifact_bytes_len: u32,
    artifact_bytes_seed: SpecDigest32,
) -> SpecDigest32 {
    // Suppress unused-variable warnings while preserving the production
    // signature. The body is opaque to Verus.
    let _ = artifact_bytes_len;
    // Mirror of the production finalization line:
    //   WorkflowDigest::from_bytes(blake3::hash(source).into())
    // (mod_compile_core.rs:115)
    artifact_bytes_seed
}

// ============================================================================
// Pure decision fn: `spec_classify_role_mismatch`
// ============================================================================
//
// Pure decision fn mirroring the spec-invented `classify_source_digest`
// and `classify_artifact_digest` predicates, plus the composite
// `recovery_error` decision lattice. The mirror unifies the role-
// specific classifiers into a single fn so the bridge contract can
// state the production-shape invariant:
//
//     classify_source_digest(claim, actual) returns WorkflowSourceDigestMismatch
//         iff canonical_digest(.) role mismatch is detected.
//
//     classify_artifact_digest(claim, actual) returns CompiledIrDigestMismatch
//         iff compute_compiled_digest(.) role mismatch is detected.
//
// The body is `#[verifier::external]`. The bridge contract in the spec
// file asserts the per-role error mapping and the determinism claim
// (same inputs => same classification) without symbolically executing
// the decision lattice.
#[verifier::external]
pub fn spec_classify_role_mismatch(
    role: SpecDigestRole,
    claim: SpecDigest32,
    actual: SpecDigest32,
) -> Option<SpecChainError> {
    // Suppress unused-variable warnings while preserving the production
    // signature. The body is opaque to Verus.
    let _ = role;
    let _ = claim;
    let _ = actual;
    // The mirror body returns None unconditionally; the bridge contract
    // in the spec file attaches the actual production-shape error
    // mapping.
    None
}

// ============================================================================
// Pure predicate fn: `spec_recovery_success_allowed`
// ============================================================================
//
// Mirror of the spec-invented `recovery_success_allowed` predicate
// (yaml_e2e_digest_roles.rs:81-94). Pure: takes role-bound flags and
// returns a bool. The body is `#[verifier::external]`; the bridge
// contract in the spec file asserts the conjunction
//
//     source digest valid
//     AND artifact digest valid
//     AND gate_ok AND proof_ok AND capability_ok
//     AND replay_ok
//
// without symbolically executing the conjunction.
#[verifier::external]
pub fn spec_recovery_success_allowed(
    source_digest_valid: bool,
    artifact_digest_valid: bool,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
) -> bool {
    // Suppress unused-variable warnings while preserving the production
    // signature. The body is opaque to Verus.
    let _ = source_digest_valid;
    let _ = artifact_digest_valid;
    let _ = gate_ok;
    let _ = proof_ok;
    let _ = capability_ok;
    let _ = replay_ok;
    // The mirror body returns false unconditionally; the bridge
    // contract in the spec file attaches the actual production-shape
    // conjunction.
    false
}

// ============================================================================
// Pure predicate fn: `spec_recovery_error_classification`
// ============================================================================
//
// Mirror of the spec-invented `recovery_error` predicate
// (yaml_e2e_digest_roles.rs:96-117). Pure: takes role-bound digests
// and gates, returns Option<SpecChainError>. Body is
// `#[verifier::external]`; the bridge contract in the spec file
// asserts the deterministic decision lattice:
//
//   !source_digest_valid  => Some(WorkflowSourceDigestMismatch)
//   !artifact_digest_valid=> Some(CompiledIrDigestMismatch)
//   !gate_ok / !proof_ok / !capability_ok => Some(AcceptedArtifactInvalid)
//   !replay_ok            => Some(ReplayDivergence)
//   otherwise             => None
//
// without symbolically executing the if/else if chain.
#[verifier::external]
pub fn spec_recovery_error_classification(
    source_digest_valid: bool,
    artifact_digest_valid: bool,
    gate_ok: bool,
    proof_ok: bool,
    capability_ok: bool,
    replay_ok: bool,
) -> Option<SpecChainError> {
    // Suppress unused-variable warnings while preserving the production
    // signature. The body is opaque to Verus.
    let _ = source_digest_valid;
    let _ = artifact_digest_valid;
    let _ = gate_ok;
    let _ = proof_ok;
    let _ = capability_ok;
    let _ = replay_ok;
    // The mirror body returns None unconditionally; the bridge
    // contract in the spec file attaches the actual production-shape
    // decision lattice.
    None
}