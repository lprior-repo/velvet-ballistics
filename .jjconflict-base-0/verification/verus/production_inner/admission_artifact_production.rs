// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for admission artifact model
// ============================================================================
//
// This file is a VERBATIM mirror of the production artifact admission
// types and decision logic. The mirror contains the same field names,
// discriminant sets, and decision-fn semantics as the production
// source so any drift breaks the companion
// `extern_admission_artifact_model.rs` Verus build at compile time,
// which is the explicit drift-detection mechanism.
//
// Production sources mirrored:
//   - `vb_core::ids::WorkflowDigest`             (crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:252-269)
// Production `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:252-269`
//   - `vb_core::ids::ActionId`                   (crates/vb_core/src/ids/mod.rs:73)
//   - `vb_storage::admission::VerificationWarning`
//                                                 (crates/vb_storage/src/admission/proof.rs:8-15)
//   - `vb_storage::admission::VerificationProof` (crates/vb_storage/src/admission/proof.rs:55-80)
//   - `vb_storage::admission::AcceptedArtifact`   (crates/vb_storage/src/admission.rs:37-73)
//   - `vb_storage::admission::verification_proof_core`
//                                                 (crates/vb_storage/src/admission/proof.rs:103-118)
//   - `vb_storage::admission::prepare_with_profile` proof construction
//                                                 (crates/vb_storage/src/admission/verification.rs:39-42)
//   - `vb_core::artifact::ObligationKind`        (crates/vb_core/src/artifact.rs:9-30)
//   - `vb_core::artifact::AssumptionId`          (crates/vb_core/src/artifact.rs:34-43)
//   - `vb_core::artifact::ObligationClosure`     (crates/vb_core/src/artifact.rs:47-65)
//   - `vb_core::artifact::VerificationLedgerEntry`
//                                                 (crates/vb_core/src/artifact.rs:69-76)
//   - `vb_core::artifact::ArtifactVerificationEvidence`
//                                                 (crates/vb_core/src/artifact.rs:80-97)
//   - `vb_core::artifact::ArtifactEvidence`      (crates/vb_core/src/artifact.rs:101-108)
//   - `vb_core::action_pack::PackedArtifactEvidence`
//                                                 (crates/vb_core/src/action_pack.rs:133-138)
//   - `vb_core::policy::RuntimePolicy`           (crates/vb_core/src/policy.rs:194-201)
//
// Stub substitutions: the production `WorkflowDigest` is a newtype
// over `[u8; 32]`; we model it as `pub struct WorkflowDigest(pub u64)`
// for spec-mode equality reasoning. The production `AcceptedArtifact`
// carries opaque payload fields (`ir: Vec<u8>`, `accepted_at_seq:
// EventSeq`, `required_capabilities: Box<[Capability]>`) which are
// not part of the prepared-proof-shape decision and are NOT mirrored here.
// The production `VerificationProof` collection fields are `Box<[ActionId]>`
// and `Vec<VerificationWarning>`; the `Box<[...]>` shape cannot be
// replicated inside the extern file without invoking `alloc`, which Verus
// forbids in `#[verifier::external]` bodies, so the slices are mirrored
// as `Vec<ActionId>` (constructible via `vec![]` in exec fns, measurable
// via `@.len()` in spec fns). The production `VerificationWarning::message`
// is `Box<str>`; it is mirrored as `String`, the same class of projection
// as the digest. The production `PackedArtifactEvidence::packs` is
// `Box<[ActionPack]>`; `ActionPack` itself carries ten fields over nine
// further digest types, so it is mirrored as an opaque unit marker (the
// budget-mirror `Boxed*` precedent) and the packs as `Vec<ActionPack>`;
// only the variant and field shape matter for the prepared-proof-shape
// decision, never the pack contents.
//
// DRIFT POLICY: This file MUST be regenerated whenever the production
// source changes. The mirror is annotated with the originating
// production line ranges so regeneration is mechanical. The companion
// extern file `extern_admission_artifact_model.rs` includes this file
// via `#[path = "production_inner/admission_artifact_production.rs"]`.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ---------------------------------------------------------------------------
// Production mirror: vb_core::ids::WorkflowDigest
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_core::ids::WorkflowDigest` at
// `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:252-269`. Production is
// `pub struct WorkflowDigest([u8; 32])` with a derived `PartialEq`
// impl. We model the digest as `pub struct WorkflowDigest(pub u64)`
// for spec-mode equality reasoning — equality reduces to a u64
// comparison and the proof kernel only reasons about identity, not
// byte-level content.
#[derive(Clone, Copy)]
pub struct WorkflowDigest(pub u64);

// ---------------------------------------------------------------------------
// Production mirror: vb_core::ids::ActionId
// ---------------------------------------------------------------------------
//
// Verbatim mirror of the `numeric_id!(ActionId, u16, get)` invocation at
// `crates/vb_core/src/ids/mod.rs:73` (macro at mod.rs:24-55). Production is
// a transparent u16 newtype with `new`/`get`; the mirror carries the same
// representation. Only the `get` projection matters here: the prepared
// shape reasons about the idempotency slices by length, never by value.
#[derive(Clone, Copy)]
pub struct ActionId(pub u16);

// ---------------------------------------------------------------------------
// Production mirror: vb_storage::admission::VerificationWarning
// ---------------------------------------------------------------------------
//
// Production `crates/vb_storage/src/admission/proof.rs:8-15`
//
// Verbatim mirror of `vb_storage::admission::VerificationWarning` at
// `crates/vb_storage/src/admission/proof.rs:8-15`. Production `message` is
// `Box<str>`; the mirror carries `String`, the same class of projection
// as the digest. The prepared shape reasons about the warnings by count
// only; no mirror fn reads a message.
#[derive(Clone)]
pub struct VerificationWarning {
    pub code: u32,
    pub message: String,
    pub gate: u8,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::policy::RuntimePolicy
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/policy.rs:194-201`
//
// Verbatim mirror of `vb_core::policy::RuntimePolicy` at
// `crates/vb_core/src/policy.rs:194-201`. Production carries
// `#[non_exhaustive]`; the attribute is substituted out (the drift gate
// normalizes attributes away). All three variants are mirrored in
// production order.
#[derive(Clone, Copy)]
pub enum RuntimePolicy {
    Strict,
    Journaled,
    Relaxed,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::ObligationKind
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:9-30`
//
// Verbatim mirror of `vb_core::artifact::ObligationKind` at
// `crates/vb_core/src/artifact.rs:9-30`. All ten obligations in
// production order; the prepared evidence ledger carries them in this
// canonical order.
#[derive(Clone, Copy)]
pub enum ObligationKind {
    Determinism,
    EffectSafety,
    CapabilityNonMinting,
    Boundedness,
    ReplayEffectSeparation,
    ArtifactValidity,
    IdempotencyClassification,
    RecoveryClassification,
    TaintHandling,
    PolicyBinding,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::AssumptionId
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:34-43`
//
// Verbatim mirror of `vb_core::artifact::AssumptionId` at
// `crates/vb_core/src/artifact.rs:34-43`. All four assumptions in
// production order.
#[derive(Clone, Copy)]
pub enum AssumptionId {
    ExecutorHonorsActionContracts,
    RuntimeReplayEffectSeparation,
    RuntimeRecoverySemantics,
    RuntimeTaintEnforcement,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::ObligationClosure
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:47-65`
//
// Verbatim mirror of `vb_core::artifact::ObligationClosure` at
// `crates/vb_core/src/artifact.rs:47-65`. All five closures in production
// order with their digest fields.
#[derive(Clone, Copy)]
pub enum ObligationClosure {
    StaticProof {
        proof_digest: WorkflowDigest,
    },
    RuntimeValidated {
        verifier_digest: WorkflowDigest,
    },
    RuntimeEvidenced {
        journal_schema_digest: WorkflowDigest,
    },
    OperatorAttested {
        approval_policy_digest: WorkflowDigest,
    },
    ExternalAssumption {
        assumption_id: AssumptionId,
        evidence_digest: WorkflowDigest,
    },
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::VerificationLedgerEntry
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:69-76`
//
// Verbatim mirror of `vb_core::artifact::VerificationLedgerEntry` at
// `crates/vb_core/src/artifact.rs:69-76`.
#[derive(Clone, Copy)]
pub struct VerificationLedgerEntry {
    pub obligation: ObligationKind,
    pub closure: ObligationClosure,
    pub policy_digest: WorkflowDigest,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::ArtifactVerificationEvidence
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:80-97`
//
// Verbatim mirror of `vb_core::artifact::ArtifactVerificationEvidence` at
// `crates/vb_core/src/artifact.rs:80-97`. The `ledger` carries all ten
// obligations in canonical order with no omitted or duplicate entries.
#[derive(Clone)]
pub struct ArtifactVerificationEvidence {
    pub schema_version: u16,
    pub payload_digest: WorkflowDigest,
    pub workflow_digest: WorkflowDigest,
    pub action_contract_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub admission_policy: RuntimePolicy,
    pub verifier_digest: WorkflowDigest,
    pub ledger: [VerificationLedgerEntry; 10],
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::artifact::ArtifactEvidence
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/artifact.rs:101-108`
//
// Verbatim mirror of `vb_core::artifact::ArtifactEvidence` at
// `crates/vb_core/src/artifact.rs:101-108`. All three variants in
// production order. Only the `Verified` variant can pass accepted-artifact
// verification; `Unverified` and `Packed` are rejected by
// `checked_evidence_schema` at
// `crates/vb_storage/src/admission/verification.rs:106-117`.
#[derive(Clone)]
pub enum ArtifactEvidence {
    Unverified,
    Verified(Box<ArtifactVerificationEvidence>),
    Packed(Box<PackedArtifactEvidence>),
}

// ---------------------------------------------------------------------------
// Production mirror: vb_core::action_pack::ActionPack (opaque marker)
// ---------------------------------------------------------------------------
//
// Production `ActionPack` (crates/vb_core/src/action_pack.rs:108-129)
// declares ten fields over nine further digest types
// (`ManifestDigest`, `ExecutorIdentity`, `ExecutorVersionDigest`,
// `CapabilityScopeSpec`, `ReceiptSchemaDigest`, `TimeoutPolicy`,
// `IdempotencyClass`, `RecoveryClass`, `ConformanceReportDigest`).
// Mirroring the full struct would pull those nine types into this
// mirror; the prepared-proof-shape decision never inspects pack
// contents, so the struct is mirrored as an opaque unit marker, the
// budget-mirror `Boxed*` precedent: the type name matches production
// for signature drift detection, the contents are out of scope.
#[derive(Clone, Copy)]
pub struct ActionPack(pub ());

// ---------------------------------------------------------------------------
// Production mirror: vb_core::action_pack::PackedArtifactEvidence
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/action_pack.rs:133-138`
//
// Verbatim mirror of `vb_core::action_pack::PackedArtifactEvidence` at
// `crates/vb_core/src/action_pack.rs:133-138`. Production `packs` is
// `Box<[ActionPack]>`; the mirror carries `Vec<ActionPack>` for the
// same reason the proof slices are `Vec` (see the header note).
#[derive(Clone)]
pub struct PackedArtifactEvidence {
    pub base: ArtifactVerificationEvidence,
    pub packs: Vec<ActionPack>,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_storage::admission::VerificationProof
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_storage::admission::VerificationProof` at
// `crates/vb_storage/src/admission/proof.rs:55-80`. All 12 production fields
// are mirrored: digest, gate_count, durable, bounded_claimed,
// taint_safe_claimed, retry_safe_claimed,
// idempotency_verified_claimed, replayable_claimed, idempotency_keyed,
// idempotency_attested, warnings, evidence.
pub struct VerificationProof {
    pub digest: WorkflowDigest,
    pub gate_count: u8,
    pub durable: bool,
    pub bounded_claimed: bool,
    pub taint_safe_claimed: bool,
    pub retry_safe_claimed: bool,
    pub idempotency_verified_claimed: bool,
    pub replayable_claimed: bool,
    pub idempotency_keyed: Vec<ActionId>,
    pub idempotency_attested: Vec<ActionId>,
    pub warnings: Vec<VerificationWarning>,
    pub evidence: ArtifactEvidence,
}

// ---------------------------------------------------------------------------
// Production mirror: vb_storage::admission::AcceptedArtifact
// ---------------------------------------------------------------------------
//
// Verbatim mirror of `vb_storage::admission::AcceptedArtifact` at
// `crates/vb_storage/src/admission.rs:37-73`. The 4 digest fields
// + embedded `VerificationProof` are mirrored because those are the
// only fields the spec reasons about. Opaque payload fields are
// omitted.
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub source_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub verification: VerificationProof,
}

// ---------------------------------------------------------------------------
// Production decision fn: proof_matches_prepared_shape
// ---------------------------------------------------------------------------
//
// Mirrors the prepared-proof shape production constructs for every
// admitted artifact: `verification_proof_core` at
// `crates/vb_storage/src/admission/proof.rs:103-118` forces gate_count 0,
// durable false, and every proof flag false (supplied counts and
// durability are intentionally ignored by `VerificationProof::new` at
// admission/proof.rs:125-141, which also leaves the idempotency slices and
// warnings empty and the evidence `Unverified`), and
// `prepare_with_profile` at
// `crates/vb_storage/src/admission/verification.rs:39-42` then sets
// exactly `bounded_claimed` and `idempotency_verified_claimed` true
// and wraps the rebuilt evidence as `Verified` (Legacy profile; the
// Registered profile wraps as `Packed` instead).
// `submit_artifact_with_contracts` (admission.rs:92-112) performs no
// gate-count or proof-flag validation of its own; the shape is
// enforced by `verify_accepted_artifact` at
// `crates/vb_storage/src/admission/verification.rs:60-90`, which
// recomputes the expected artifact and rejects any deviation with
// `ArtifactEvidenceInvalid` (see the gate-count and proof-flag tests
// at `crates/vb_runtime/src/admission/artifact_envelope_tests.rs:205-260`),
// after `checked_evidence_schema` at verification.rs:104-117 has
// required `Verified` evidence at schema version `EVIDENCE_VERSION`.
//
// Returns `true` iff the proof carries exactly the prepared shape:
// gate_count 0, durable false, `bounded_claimed` and
// `idempotency_verified_claimed` true, `taint_safe_claimed`,
// `retry_safe_claimed`, and `replayable_claimed` false, the
// idempotency slices and warnings empty, and the evidence `Verified`
// at schema version `EVIDENCE_VERSION`. A proof with
// the legacy 15-gate count and all flags true (the shape the
// always-present store fixture builds at
// `crates/vb_runtime/src/admission/parts/chunk_003_stores.rs:74-91`)
// does NOT match: it is not an admitted artifact, and the real
// validation rejects it.
//
// Marked `#[verifier::external]` so Verus skips body verification;
// the companion spec file attaches the production contract via
// `assume_specification[ production::proof_matches_prepared_shape ]`.
#[verifier::external]
pub fn proof_matches_prepared_shape(proof: &VerificationProof) -> bool {
    proof.gate_count == 0
        && !proof.durable
        && proof.bounded_claimed
        && !proof.taint_safe_claimed
        && !proof.retry_safe_claimed
        && proof.idempotency_verified_claimed
        && !proof.replayable_claimed
        && proof.idempotency_keyed.is_empty()
        && proof.idempotency_attested.is_empty()
        && proof.warnings.is_empty()
        && evidence_is_verified_at_current_schema(&proof.evidence)
}

// ---------------------------------------------------------------------------
// Production decision fn: evidence_is_verified_at_current_schema
// ---------------------------------------------------------------------------
//
// Helper for `proof_matches_prepared_shape`: true iff the evidence is
// the `Verified` variant carrying schema version `EVIDENCE_VERSION`.
// Production pins this in `checked_evidence_schema` at
// `crates/vb_storage/src/admission/verification.rs:104-117`, which
// rejects `Unverified`, `Packed`, and any `Verified` evidence at a
// stale schema version before `verify_accepted_artifact` recomputes
// the expected artifact. The `3` below is the `EVIDENCE_VERSION`
// constant at `crates/vb_storage/src/admission/verification.rs:10`,
// spelled as a literal because a `const` in this `#[path]` mirror
// module trips a Verus internal error (erasure context); the spec
// file carries the same value as `PREPARED_EVIDENCE_VERSION`.
fn evidence_is_verified_at_current_schema(evidence: &ArtifactEvidence) -> bool {
    match evidence {
        ArtifactEvidence::Verified(inner) => inner.schema_version == 3,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Production decision fn: digest_eq
// ---------------------------------------------------------------------------
//
// Mirrors the `PartialEq` impl derived on `vb_core::ids::WorkflowDigest`
// at `crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:239`.
#[verifier::external]
pub fn digest_eq(a: &WorkflowDigest, b: &WorkflowDigest) -> bool {
    a.0 == b.0
}

// ---------------------------------------------------------------------------
// Production decision fn: artifact_digest_bound
// ---------------------------------------------------------------------------
//
// Mirrors `vb_storage::admission::bind_artifact_digest` at
// `crates/vb_storage/src/admission/digest.rs:29-34`. Returns `true` iff
// the artifact's top-level digest equals the verification proof's
// digest.
#[verifier::external]
pub fn artifact_digest_bound(artifact: &AcceptedArtifact) -> bool {
    digest_eq(&artifact.digest, &artifact.verification.digest)
}
