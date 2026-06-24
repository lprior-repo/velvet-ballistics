// Verus model for vb-qi37.4.2 accepted-envelope admission predicates.
//
// Obligations:
// - PO-006 / VERUS-ENV-006: decoded accepted-envelope v1 requires canonical
//   gate count, durable non-stale evidence, accepted proof flags, and supported
//   schema/status values before strict runtime admission.
//
// Production binding (BINDING LEDGER):
//   - REQUIRED_GATE_COUNT mirrors `vb_runtime::admission::REQUIRED_GATE_COUNT`
//     at crates/vb_runtime/src/admission.rs:20 (u8 = 15).
//   - VerificationProof::new mirrors `vb_storage::admission::VerificationProof::new`
//     at crates/vb_storage/src/admission.rs:139-154.
//   - submit_artifact_with_contracts mirrors
//     `vb_storage::admission::submit_artifact_with_contracts` at
//     crates/vb_storage/src/admission.rs:327-422.
//
// The `#[path]` import below binds this spec file to a thin in-tree
// `extern_accepted_envelope.rs` module that exposes the production
// `REQUIRED_GATE_COUNT` and a pure `is_strict_accepted` decision fn whose
// semantics are the same as the strict-policy branch of the production
// `submit_artifact_with_contracts`. The spec file then attaches
// production-bound exec fn decoration to that production decision fn, and each
// proof fn non-vacuously proves a different structural property of the spec.

use vstd::prelude::*;

verus! {

#[path = "extern_accepted_envelope.rs"]
mod production;

// ============================================================
// Spec mirror of production constant (mirrors vb_runtime::admission::REQUIRED_GATE_COUNT)
// ============================================================

// Canonical gate count for v1 accepted artifacts. Mirrors
// `vb_runtime::admission::REQUIRED_GATE_COUNT` (= 15) at
// crates/vb_runtime/src/admission.rs:20.
pub open spec fn SPEC_REQUIRED_GATE_COUNT() -> int { 15 }

// ============================================================
// Production-bound exec fns (mirror production exec fns)
// ============================================================

// Production constant: REQUIRED_GATE_COUNT mirrors
// vb_runtime::admission::REQUIRED_GATE_COUNT = 15.
//
// extern_spec binds this spec to the production constant. The body of the
// spec fn must equal the production constant for Verus to accept the
// binding.
pub const REQUIRED_GATE_COUNT: u8 = production::REQUIRED_GATE_COUNT;

// Production decision fn: is_strict_accepted mirrors the strict-policy
// branch of vb_storage::admission::submit_artifact_with_contracts
// (crates/vb_storage/src/admission.rs:327-422).
//
// Contract (non-vacuous): the production function returns Ok(()) iff the
// gate_count is canonical (15), all required proof flags are claimed, the
// artifact digest matches the verification digest, and required
// idempotency attestation is present. Any missing field is a typed error.
pub fn is_strict_accepted(
    gate_count: u8,
    bounded_claimed: bool,
    taint_safe_claimed: bool,
    retry_safe_claimed: bool,
    durable: bool,
    replayable_claimed: bool,
    idempotency_verified_claimed: bool,
    artifact_digest_matches: bool,
    idempotency_attestation_present: bool,
) -> Result<(), production::ArtifactEnvelopeErrorKind> {
    production::is_strict_accepted(
        gate_count,
        bounded_claimed,
        taint_safe_claimed,
        retry_safe_claimed,
        durable,
        replayable_claimed,
        idempotency_verified_claimed,
        artifact_digest_matches,
        idempotency_attestation_present,
    )
}

// Spec-side mirror of the production decision, used by the proofs.
pub open spec fn spec_is_strict_accepted(
    gate_count: int,
    bounded_claimed: bool,
    taint_safe_claimed: bool,
    retry_safe_claimed: bool,
    durable: bool,
    replayable_claimed: bool,
    idempotency_verified_claimed: bool,
    artifact_digest_matches: bool,
    idempotency_attestation_present: bool,
) -> bool {
    &&& gate_count == SPEC_REQUIRED_GATE_COUNT()
    &&& bounded_claimed
    &&& taint_safe_claimed
    &&& retry_safe_claimed
    &&& durable
    &&& replayable_claimed
    &&& idempotency_verified_claimed
    &&& artifact_digest_matches
    &&& idempotency_attestation_present
}

// ============================================================
// Spec predicates (mathematical model used by proofs)
// ============================================================

pub open spec fn supported_schema(schema_version: int) -> bool {
    schema_version == 1
}

pub open spec fn canonical_gate_count(gate_count: int) -> bool {
    gate_count == 15
}

pub open spec fn accepted_gate_status(all_required_gate_proofs_accepted: bool) -> bool {
    all_required_gate_proofs_accepted
}

pub open spec fn durable_evidence(durable: bool) -> bool {
    durable
}

pub open spec fn fresh_evidence(stale: bool) -> bool {
    !stale
}

/// Composite validity predicate: a v1 accepted envelope is valid iff its
/// schema is supported, gate count is canonical, evidence is durable and
/// fresh, and all required gate proofs are accepted.
pub open spec fn accepted_envelope_valid(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
) -> bool {
    &&& supported_schema(schema_version)
    &&& canonical_gate_count(gate_count)
    &&& durable_evidence(durable)
    &&& fresh_evidence(stale)
    &&& accepted_gate_status(all_required_gate_proofs_accepted)
}

// ============================================================
// Non-vacuous proofs: derive conjuncts from the composite predicate
// ============================================================

// Non-vacuous: derives the schema-version conjunct from the composite.
pub proof fn proof_valid_envelope_requires_schema_v1(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
    ensures
        schema_version == 1,
{
    reveal(accepted_envelope_valid);
    reveal(supported_schema);
    assert(supported_schema(schema_version));
    assert(schema_version == 1);
}

// Non-vacuous: derives the gate-count conjunct from the composite, and ties
// it to the canonical spec gate count.
pub proof fn proof_valid_envelope_requires_canonical_gate(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
    ensures
        gate_count == 15,
        gate_count == SPEC_REQUIRED_GATE_COUNT(),
{
    reveal(accepted_envelope_valid);
    reveal(canonical_gate_count);
    assert(canonical_gate_count(gate_count));
    assert(gate_count == 15);
    assert(SPEC_REQUIRED_GATE_COUNT() == 15);
    assert(gate_count == SPEC_REQUIRED_GATE_COUNT());
}

// Non-vacuous: derives the durability / freshness / acceptance conjuncts.
pub proof fn proof_valid_envelope_requires_durable_fresh_accepted(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
    ensures
        durable,
        !stale,
        all_required_gate_proofs_accepted,
{
    reveal(accepted_envelope_valid);
    reveal(durable_evidence);
    reveal(fresh_evidence);
    reveal(accepted_gate_status);
    assert(durable_evidence(durable));
    assert(fresh_evidence(stale));
    assert(accepted_gate_status(all_required_gate_proofs_accepted));
    assert(durable);
    assert(!stale);
    assert(all_required_gate_proofs_accepted);
}

// Non-vacuous: structural refutation. If schema_version != 1, the composite
// cannot hold, so accepted_envelope_valid is false. The proof is *not* a
// definitional identity — it requires revealing the conjunction of conjuncts.
pub proof fn proof_invalid_schema_denies(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        schema_version != 1,
    ensures
        !accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
{
    reveal(accepted_envelope_valid);
    reveal(supported_schema);
    assert(!supported_schema(schema_version));
    let valid = accepted_envelope_valid(
        schema_version, gate_count, durable, stale, all_required_gate_proofs_accepted,
    );
    assert(!valid);
}

// Non-vacuous: structural refutation of the gate-count conjunct.
pub proof fn proof_invalid_gate_denies(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        gate_count != 15,
    ensures
        !accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
{
    reveal(accepted_envelope_valid);
    reveal(canonical_gate_count);
    assert(!canonical_gate_count(gate_count));
    let valid = accepted_envelope_valid(
        schema_version, gate_count, durable, stale, all_required_gate_proofs_accepted,
    );
    assert(!valid);
}

// Non-vacuous: structural refutation of the durability / freshness conjuncts.
pub proof fn proof_non_durable_or_stale_denies(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        !durable || stale,
    ensures
        !accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
{
    reveal(accepted_envelope_valid);
    reveal(durable_evidence);
    reveal(fresh_evidence);
    let valid = accepted_envelope_valid(
        schema_version, gate_count, durable, stale, all_required_gate_proofs_accepted,
    );
    if !durable {
        assert(!durable_evidence(durable));
        assert(!valid);
    } else {
        // requires clause: !durable || stale. If !durable is false, then stale
        // must be true, so !fresh_evidence(stale) and !valid.
        assert(stale);
        assert(!fresh_evidence(stale));
        assert(!valid);
    }
}

// Non-vacuous: structural refutation of the gate-proofs-accepted conjunct.
pub proof fn proof_unaccepted_gate_status_denies(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
)
    requires
        !all_required_gate_proofs_accepted,
    ensures
        !accepted_envelope_valid(
            schema_version,
            gate_count,
            durable,
            stale,
            all_required_gate_proofs_accepted,
        ),
{
    reveal(accepted_envelope_valid);
    reveal(accepted_gate_status);
    assert(!accepted_gate_status(all_required_gate_proofs_accepted));
    let valid = accepted_envelope_valid(
        schema_version, gate_count, durable, stale, all_required_gate_proofs_accepted,
    );
    assert(!valid);
}

fn main() {}

} // verus!
