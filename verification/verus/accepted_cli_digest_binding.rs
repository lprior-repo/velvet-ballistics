// Verus proof obligations for vb-core-cli-accepted-path PO-002.
//
// Obligation ID: VERUS-CLI-DIGEST-002
// Verifier: verus --crate-type=lib verification/verus/accepted_cli_digest_binding.rs
// Expected evidence: Verus report shows 0 errors; spec predicates,
// assume_specification bridges, spec proofs, and exec proof witnesses
// all verified.
//
// ============================================================================
// WEAK PRODUCTION BINDING (production_inner mirror)
// ============================================================================
//
// This spec is bound through `extern_accepted_cli_digest_binding.rs` to
// handwritten structural mirrors in `production_inner/`. Those mirrors
// expose digest fields and isolated decision predicates; they do not
// include or prove the complete production admission call chain.
//
// The pre-binding spec defined a 5-element `digest_binding_total`
// predicate as a pure-math chained equality over 5 abstract `int`
// parameters and proved 3 structural properties of that predicate.
// That is a VACUUM proof: production never constructs a 5-tuple of
// digests, and no production code path was discharged — renaming any
// field on `RunAdmission` or `AcceptedArtifact` would not have
// surfaced as a Verus error.
//
// This rewrite attaches small mirror predicates to production-shaped digest
// positions. The actual admission path is
// `admit_artifact_run_with_certificate_floor` (chunk_005_admit_core.rs:91-113),
// which dispatches to `admit_strict_artifact_run` (lines 115-130), then
// `verify_admission_digest` (lines 156-173) and
// `admission_from_artifact` (lines 208-224). Those helpers are not proved
// as one end-to-end path by this mirror.
//
// `verify_admission_digest` accepts a request matching either
// `artifact.digest` or `artifact.source_digest`, and separately requires
// `artifact.verification.digest == artifact.digest`. Therefore the
// production condition does not imply that the requested/header digest
// equals `artifact.digest` when only `source_digest` matches. The 5-way
// equality predicate below is a stronger model predicate, not an established
// production post-condition.
//
// ============================================================================
// PRODUCTION BINDING LEDGER (mirrors extern_accepted_cli_digest_binding.rs)
// ============================================================================
//   - SpecWorkflowDigest            <- extern_accepted_cli_digest_binding.rs
//                                       (mirror of vb_core::ids::WorkflowDigest
//                                       at crates/vb_core/src/ids/parts/chunk_001_custom_types.rs:251-254)
//   - SpecAcceptedArtifact          <- extern_accepted_cli_digest_binding.rs
//                                       (mirror of
//                                       vb_storage::admission::AcceptedArtifact
//                                       at
//                                       crates/vb_storage/src/admission.rs:41-77)
//   - SpecRunAdmission              <- extern_accepted_cli_digest_binding.rs
//                                       (mirror of
//                                       vb_runtime::admission::RunAdmission
//                                       at crates/vb_runtime/src/admission/parts/chunk_002_records.rs:6-34)
//   - production_artifact_digest_eq_header
//                                   <- extern_accepted_cli_digest_binding.rs
//                                       (isolated mirror of the digest
//                                       acceptance condition in
//                                       `verify_admission_digest` at
//                                       chunk_005_admit_core.rs:156-165)
//   - production_proof_digest_eq_artifact
//                                   <- extern_accepted_cli_digest_binding.rs
//                                       (isolated mirror of the proof-digest
//                                       check in `verify_admission_digest` at
//                                       chunk_005_admit_core.rs:166-173)
//   - production_run_admission_new_digest
//                                   <- extern_accepted_cli_digest_binding.rs
//                                       (mirror of RunAdmission::new at
//                                       chunk_002_records.rs:51-69)
//   - production_run_admission_artifact_digest
//                                   <- extern_accepted_cli_digest_binding.rs
//                                       (mirror of RunAdmission::artifact_digest
//                                       at chunk_002_records.rs:161-165)
//
// ============================================================================
// UPGRADE FROM PREVIOUS (VACUUM) FORM
// ============================================================================
// The previous `accepted_cli_digest_binding.rs` defined
// `digest_binding_total(source, artifact, header, event, admission)`
// as a 5-way pure-math equality and proved 3 lemmas over it
// vacuously. There was no production binding: the 5 `int` parameters
// had no connection to any production field, type, or fn signature.
//
// The current proofs use handwritten SpecWorkflowDigest,
// SpecAcceptedArtifact and SpecRunAdmission mirror types. Their external
// wrappers carry trusted assume_specification contracts. The lemmas and
// witnesses reason about those mirror contracts; they neither execute
// production admission nor discharge its end-to-end behavior.
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The external mirror bodies are not verified. Production source
// correspondence is checked separately by binding and drift audits;
// verifier success here does not establish production-path proof closure.
//
// ============================================================================
// Obligations:
// - VERUS-CLI-DIGEST-002: the abstract 5-digest equality predicate
//   implies transitivity and rejects mismatched model inputs.
// - The mirror witnesses verify assumed predicate contracts. They do
//   not establish 5-way equality or mismatch rejection for production
//   admission; the source-digest alternative prevents that inference.
//
// Verifier command:
//   verus --crate-type=lib verification/verus/accepted_cli_digest_binding.rs
use vstd::prelude::*;

verus! {

// ============================================================================
// Production extern surface — `#[path]`-bound mirror of admission.rs
// ============================================================================
#[path = "extern_accepted_cli_digest_binding.rs"]
mod production;

// Re-export the mirror types and exec wrappers so the spec proofs
// below reference them as `production::SpecAcceptedArtifact`,
// `production::production_artifact_digest_eq_header`, etc.
pub use production::{
    SpecAcceptedArtifact,
    SpecRunAdmission,
    SpecVerificationProof,
    SpecWorkflowDigest,
    production_artifact_digest_eq_header,
    production_proof_digest_eq_artifact,
    production_run_admission_artifact_digest,
    production_run_admission_new_digest,
};

// ============================================================================
// Spec predicate: abstract 5-digest binding equality
// ============================================================================
/// Spec view: the abstract 5-digest chain is equal —
/// source == artifact == header == event == admission.
///
/// This predicate is stronger than the production digest check:
/// `verify_admission_digest` permits a source-digest match without
/// requiring `source_digest == artifact.digest`. This spec predicate is
/// not a production post-condition and is not established for every
/// successful production admission.
pub open spec fn digest_binding_total(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
) -> bool {
    &&& source_digest == artifact_digest
    &&& artifact_digest == header_digest
    &&& header_digest == event_digest
    &&& event_digest == admission_digest
}

/// Spec view: the 5-digest chain is NOT bound (any pairwise mismatch).
pub open spec fn digest_mismatch_rejects(
    source_digest: int,
    artifact_digest: int,
    header_digest: int,
    event_digest: int,
    admission_digest: int,
) -> bool {
    !digest_binding_total(
        source_digest,
        artifact_digest,
        header_digest,
        event_digest,
        admission_digest,
    )
}

// ============================================================================
// Mirror-bound exec wrappers (call the external mirror functions)
// ============================================================================
/// Production-shaped mirror of the artifact/request condition in
/// `verify_admission_digest` at chunk_005_admit_core.rs:156-165.
pub fn production_artifact_digest_eq_header_exec(
    artifact: &production::SpecAcceptedArtifact,
    header_digest: production::SpecWorkflowDigest,
) -> bool {
    production::production_artifact_digest_eq_header(artifact, header_digest)
}

/// Production-shaped mirror of the verification-digest condition in
/// `verify_admission_digest` at chunk_005_admit_core.rs:166-173.
pub fn production_proof_digest_eq_artifact_exec(
    artifact: &production::SpecAcceptedArtifact,
) -> bool {
    production::production_proof_digest_eq_artifact(artifact)
}

/// Production-shaped mirror of `RunAdmission::new` at
/// chunk_002_records.rs:51-69.
pub fn production_run_admission_new_digest_exec(
    digest: production::SpecWorkflowDigest,
) -> production::SpecRunAdmission {
    production::production_run_admission_new_digest(digest)
}

/// Production-shaped mirror of `RunAdmission::artifact_digest` at
/// chunk_002_records.rs:161-165.
pub fn production_run_admission_artifact_digest_exec(
    admission: &production::SpecRunAdmission,
) -> production::SpecWorkflowDigest {
    production::production_run_admission_artifact_digest(admission)
}

// ============================================================================
// assume_specification bridges — trusted mirror contract surface
// ============================================================================
//
// These bridges attach spec contracts to the mirror-bound exec
// wrappers in `extern_accepted_cli_digest_binding.rs`. The body of
// each extern fn is opaque to Verus (`#[verifier::external]`); the
// spec proofs below exercise the contracts via the exec wrappers in
// the "Mirror-bound exec wrappers" section.
// --------------------------------------------------------------------------
// Bridge: `production_artifact_digest_eq_header` returns true iff
// INV-002 holds — the artifact's `digest` OR `source_digest` matches
// the requested header digest.
// --------------------------------------------------------------------------
// Mirrors the artifact/request condition in `verify_admission_digest`
// at chunk_005_admit_core.rs:156-165:
//
//     if artifact.digest != artifact_digest && artifact.source_digest != artifact_digest {
//         return Err(AdmissionError::ArtifactDigestMismatch { ... });
//     }
//
// The check passes (returns Ok / true) iff `artifact.digest ==
// artifact_digest || artifact.source_digest == artifact_digest`.
pub assume_specification[ production::production_artifact_digest_eq_header ](
    artifact: &production::SpecAcceptedArtifact,
    header_digest: production::SpecWorkflowDigest,
) -> (r: bool)
    ensures
        r == (artifact.digest == header_digest || artifact.source_digest == header_digest),
;

// --------------------------------------------------------------------------
// Bridge: `production_proof_digest_eq_artifact` returns true iff
// INV-003 holds — the verification proof's `digest` matches the
// artifact's `digest`.
// --------------------------------------------------------------------------
// Mirrors the verification-digest condition in `verify_admission_digest`
// at chunk_005_admit_core.rs:166-173:
//
//     if artifact.verification.digest != artifact.digest {
//         return Err(AdmissionError::ArtifactDigestMismatch { ... });
//     }
//
// The check passes (returns Ok / true) iff
// `artifact.verification.digest == artifact.digest`.
pub assume_specification[ production::production_proof_digest_eq_artifact ](
    artifact: &production::SpecAcceptedArtifact,
) -> (r: bool)
    ensures
        r == (artifact.verification.digest == artifact.digest),
;

// --------------------------------------------------------------------------
// Bridge: `production_run_admission_new_digest` returns a
// `SpecRunAdmission` whose `artifact_digest` field equals the input
// digest.
// --------------------------------------------------------------------------
// Mirrors `RunAdmission::new` at chunk_002_records.rs:51-69; the
// constructor sets the record's artifact_digest field from the supplied digest.
pub assume_specification[ production::production_run_admission_new_digest ](
    digest: production::SpecWorkflowDigest,
) -> (admission: production::SpecRunAdmission)
    ensures
        admission.artifact_digest == digest,
;

// --------------------------------------------------------------------------
// Bridge: `production_run_admission_artifact_digest` returns the
// `artifact_digest` field of the admission record.
// --------------------------------------------------------------------------
// Mirrors `RunAdmission::artifact_digest` at chunk_002_records.rs:161-165,
// whose body returns `self.artifact_digest`.
pub assume_specification[ production::production_run_admission_artifact_digest ](
    admission: &production::SpecRunAdmission,
) -> (r: production::SpecWorkflowDigest)
    ensures
        r == admission.artifact_digest,
;

// ============================================================================
// Abstract mirror proofs of equality and mismatch predicates.
// ============================================================================
// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — total binding implies all pairwise
// equalities.
// --------------------------------------------------------------------------
// Reveals the abstract conjunction and derives equalities over the
// mirror fields. Production source drift requires a separate binding
// audit; these handwritten mirror fields do not track source changes automatically.
pub proof fn proof_total_binding_implies_all_equal(
    artifact: production::SpecAcceptedArtifact,
    header_digest: production::SpecWorkflowDigest,
    admission: production::SpecRunAdmission,
)
    requires
        digest_binding_total(
            artifact.source_digest.0 as int,
            artifact.digest.0 as int,
            header_digest.0 as int,
            artifact.verification.digest.0 as int,
            admission.artifact_digest.0 as int,
        ),
    ensures
        artifact.source_digest.0 as int == artifact.digest.0 as int,
        artifact.source_digest.0 as int == header_digest.0 as int,
        artifact.source_digest.0 as int == artifact.verification.digest.0 as int,
        artifact.source_digest.0 as int == admission.artifact_digest.0 as int,
{
    reveal(digest_binding_total);
    // digest_binding_total is a chained conjunction of equalities
    // between adjacent pairs in the chain (source-artifact,
    // artifact-header, header-event, event-admission). Each conjunct
    // is an equality on int; transitivity yields
    // source == artifact == header == event == admission.
    assert(artifact.source_digest.0 as int == artifact.digest.0 as int);
    assert(artifact.digest.0 as int == header_digest.0 as int);
    assert(header_digest.0 as int == artifact.verification.digest.0 as int);
    assert(artifact.verification.digest.0 as int == admission.artifact_digest.0 as int);
    // Re-derive each requested equality by transitivity.
    assert(artifact.source_digest.0 as int == header_digest.0 as int);
    assert(artifact.source_digest.0 as int == artifact.verification.digest.0 as int);
    assert(artifact.source_digest.0 as int == admission.artifact_digest.0 as int);
}

// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — any pairwise mismatch rejects (denies the
// total binding).
// --------------------------------------------------------------------------
// The abstract mirror disjunction precondition is
// exactly the negation of `digest_binding_total`'s conjuncts. Reveal
// the chained conjunction and observe at least one conjunct must be
// false, which forces the conjunction to be false.
pub proof fn proof_any_pairwise_mismatch_rejects(
    artifact: production::SpecAcceptedArtifact,
    header_digest: production::SpecWorkflowDigest,
    admission: production::SpecRunAdmission,
)
    requires
        artifact.source_digest.0 != artifact.digest.0 || artifact.digest.0 != header_digest.0
            || header_digest.0 != artifact.verification.digest.0 || artifact.verification.digest.0
            != admission.artifact_digest.0,
    ensures
        digest_mismatch_rejects(
            artifact.source_digest.0 as int,
            artifact.digest.0 as int,
            header_digest.0 as int,
            artifact.verification.digest.0 as int,
            admission.artifact_digest.0 as int,
        ),
{
    reveal(digest_mismatch_rejects);
    reveal(digest_binding_total);
    let valid = digest_binding_total(
        artifact.source_digest.0 as int,
        artifact.digest.0 as int,
        header_digest.0 as int,
        artifact.verification.digest.0 as int,
        admission.artifact_digest.0 as int,
    );
    // digest_binding_total is a chained conjunction; each conjunct is
    // an equality between adjacent digests in the 5-tuple. The
    // precondition states at least one of these equalities is false,
    // so the conjunction is false.
    assert(!valid);
}

// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — admission equals artifact and verification
// digests under the stated mirror assumptions; header equality is not ensured.
// --------------------------------------------------------------------------
// The theorem assumes the admission/artifact equality and the two
// isolated digest conditions. It proves only the equalities listed in
// ensures. Its historical function name does not imply equality with
// the requested header when only artifact.source_digest matches.
pub proof fn proof_admitted_digest_matches_run_header(
    artifact: production::SpecAcceptedArtifact,
    header_digest: production::SpecWorkflowDigest,
    admission: production::SpecRunAdmission,
)
    requires
// The isolated production-shaped predicates cover:
//   - the artifact/source digest acceptance condition in
//     `verify_admission_digest` (chunk_005_admit_core.rs:156-165).
//   - equality between verification.digest and artifact.digest in
//     `verify_admission_digest` (chunk_005_admit_core.rs:166-173).
// They do not prove that source_digest equals artifact.digest or that
// the entire strict-admission call chain satisfies the stronger 5-way
// equality predicate in every accepted case.

        artifact.digest == header_digest || artifact.source_digest == header_digest,
        artifact.verification.digest == artifact.digest,
        admission.artifact_digest == artifact.digest,
    ensures
        admission.artifact_digest.0 as int == artifact.digest.0 as int,
        admission.artifact_digest.0 as int == artifact.verification.digest.0 as int,
{
    // Direct: admission.artifact_digest == artifact.digest (from
    // precondition, line 3 of requires) and artifact.verification.digest
    // == artifact.digest (from precondition, line 2). Transitivity
    // yields admission.artifact_digest == artifact.verification.digest.
    assert(admission.artifact_digest == artifact.digest);
    assert(artifact.verification.digest == artifact.digest);
    assert(admission.artifact_digest == artifact.verification.digest);
}

// ============================================================================
// Mirror exec witnesses — check contracts on constructed mirror values
// using the existing trusted assume_specification boundaries
// ============================================================================
//
// Each witness constructs SpecAcceptedArtifact, SpecWorkflowDigest and
// SpecRunAdmission mirror values, invokes an external mirror wrapper,
// and checks the assumed contract. These witnesses do not execute or
// prove the complete production admission path.
// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — exec witness for the 5-digest happy path.
// --------------------------------------------------------------------------
// Constructs an artifact + header + admission where all 5 digest
// positions hold the same canonical digest, and asserts that the
// mirror exec wrappers return the expected values
// (INV-002 passes, INV-003 passes, RunAdmission::artifact_digest
// returns the canonical digest).
pub fn exec_witness_digest_binding_happy_path(canonical: production::SpecWorkflowDigest) -> (result:
    bool)
    ensures
        result == true,
{
    // Construct an artifact with all 3 digests equal to `canonical`.
    let artifact = production::SpecAcceptedArtifact {
        digest: canonical,
        source_digest: canonical,
        policy_digest: canonical,
        verification: production::SpecVerificationProof { digest: canonical },
    };
    // Strict-policy INV-002: artifact.digest == header (canonical).
    let inv2 = production::production_artifact_digest_eq_header(&artifact, canonical);
    // Strict-policy INV-003: artifact.verification.digest == artifact.digest.
    let inv3 = production::production_proof_digest_eq_artifact(&artifact);
    // RunAdmission::new(canonical).
    let admission = production::production_run_admission_new_digest(canonical);
    // RunAdmission::artifact_digest returns canonical.
    let adm_digest = production::production_run_admission_artifact_digest(&admission);
    assert(inv2 == true);
    assert(inv3 == true);
    assert(adm_digest == canonical);
    true
}

// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — exec witness for digest mismatch rejection.
// --------------------------------------------------------------------------
// Constructs an artifact where artifact.verification.digest does NOT
// equal artifact.digest, and asserts that the mirror-bound exec
// wrapper `production_proof_digest_eq_artifact` returns false for these
// mirror values. This witness does not establish a production error outcome.
pub fn exec_witness_digest_mismatch_rejected(
    artifact_digest: production::SpecWorkflowDigest,
    proof_digest: production::SpecWorkflowDigest,
) -> (result: bool)
    requires
        artifact_digest != proof_digest,
    ensures
        result == false,
{
    let artifact = production::SpecAcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest: artifact_digest,
        verification: production::SpecVerificationProof { digest: proof_digest },
    };
    // INV-003 fails because proof_digest != artifact_digest.
    let r = production::production_proof_digest_eq_artifact(&artifact);
    assert(r == false);
    r
}

// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — exec witness for header mismatch rejection.
// --------------------------------------------------------------------------
// Constructs an artifact whose `digest` AND `source_digest` differ
// from the requested header digest, and asserts that INV-002 fails.
pub fn exec_witness_header_mismatch_rejected(
    artifact_digest: production::SpecWorkflowDigest,
    source_digest: production::SpecWorkflowDigest,
    header_digest: production::SpecWorkflowDigest,
) -> (result: bool)
    requires
        artifact_digest != header_digest,
        source_digest != header_digest,
    ensures
        result == false,
{
    let artifact = production::SpecAcceptedArtifact {
        digest: artifact_digest,
        source_digest,
        policy_digest: artifact_digest,
        verification: production::SpecVerificationProof { digest: artifact_digest },
    };
    // INV-002 fails because neither artifact.digest nor
    // artifact.source_digest equals header_digest.
    let r = production::production_artifact_digest_eq_header(&artifact, header_digest);
    assert(r == false);
    r
}

// --------------------------------------------------------------------------
// VERUS-CLI-DIGEST-002 — exec witness for the round-trip:
// production_artifact_digest_eq_header & production_proof_digest_eq_artifact
// both pass with the same canonical digest at every position.
// --------------------------------------------------------------------------
pub fn exec_witness_round_trip_binding(canonical: production::SpecWorkflowDigest) -> (result: bool)
    ensures
        result == true,
{
    let artifact = production::SpecAcceptedArtifact {
        digest: canonical,
        source_digest: canonical,
        policy_digest: canonical,
        verification: production::SpecVerificationProof { digest: canonical },
    };
    let admission = production::production_run_admission_new_digest(canonical);
    let inv2 = production::production_artifact_digest_eq_header(&artifact, canonical);
    let inv3 = production::production_proof_digest_eq_artifact(&artifact);
    let adm_digest = production::production_run_admission_artifact_digest(&admission);
    assert(inv2 == true);
    assert(inv3 == true);
    assert(adm_digest == canonical);
    true
}

fn main() {
}

} // verus!
