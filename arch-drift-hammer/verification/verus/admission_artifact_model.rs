// Verus model for vb-qi37.4 accepted-artifact admission proof obligations.
//
// Obligations:
// - VERUS-GATE-004: strict admission requires runtime gate count and all proof flags.
// - VERUS-DIGEST-005: successful admission preserves one workflow digest through the
//   accepted artifact, compiled IR record, run header, and RunAdmission record.
//
// This is a pure finite model. Postcard decoding, Fjall I/O, digest construction,
// and production struct extraction are trusted shell boundaries that require the
// integration/Kani/fuzz evidence recorded in the bead plan.
//
// BINDING: admission_artifact_model
// Rust type: vb_storage::admission::AcceptedArtifact
// Verified: Matched spec gate_count and proof_flags to Rust AcceptedArtifact struct fields
// Divergences: Spec models abstract booleans; Rust uses actual proof flag types

use vstd::prelude::*;

verus! {

pub open spec fn required_gate_count() -> int {
    15
}

pub open spec fn proof_flags_complete(
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
) -> bool {
    bounded && taint_safe && retry_safe && durable && replayable
}

pub open spec fn gate_schema_valid(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
) -> bool {
    gate_count == required_gate_count()
        && proof_flags_complete(bounded, taint_safe, retry_safe, durable, replayable)
}

pub open spec fn digest_binding_valid(
    accepted_digest: int,
    compiled_digest: int,
    header_digest: int,
    admission_digest: int,
) -> bool {
    accepted_digest == compiled_digest
        && compiled_digest == header_digest
        && header_digest == admission_digest
}

pub open spec fn strict_admission_valid(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
    accepted_digest: int,
    compiled_digest: int,
    header_digest: int,
    admission_digest: int,
) -> bool {
    gate_schema_valid(gate_count, bounded, taint_safe, retry_safe, durable, replayable)
        && digest_binding_valid(accepted_digest, compiled_digest, header_digest, admission_digest)
}

pub proof fn proof_success_requires_runtime_gate_count(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
)
    requires
        gate_schema_valid(gate_count, bounded, taint_safe, retry_safe, durable, replayable),
    ensures
        gate_count == required_gate_count(),
        bounded,
        taint_safe,
        retry_safe,
        durable,
        replayable,
{
}

pub proof fn proof_wrong_gate_count_denies(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
)
    requires
        gate_count != required_gate_count(),
    ensures
        !gate_schema_valid(gate_count, bounded, taint_safe, retry_safe, durable, replayable),
{
}

pub proof fn proof_false_required_flag_denies(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
)
    requires
        !bounded || !taint_safe || !retry_safe || !durable || !replayable,
    ensures
        !gate_schema_valid(gate_count, bounded, taint_safe, retry_safe, durable, replayable),
{
}

pub proof fn proof_success_preserves_digest_binding(
    gate_count: int,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
    accepted_digest: int,
    compiled_digest: int,
    header_digest: int,
    admission_digest: int,
)
    requires
        strict_admission_valid(
            gate_count,
            bounded,
            taint_safe,
            retry_safe,
            durable,
            replayable,
            accepted_digest,
            compiled_digest,
            header_digest,
            admission_digest,
        ),
    ensures
        accepted_digest == compiled_digest,
        accepted_digest == header_digest,
        accepted_digest == admission_digest,
{
}

pub proof fn proof_digest_mismatch_denies(
    accepted_digest: int,
    compiled_digest: int,
    header_digest: int,
    admission_digest: int,
)
    requires
        accepted_digest != compiled_digest
            || compiled_digest != header_digest
            || header_digest != admission_digest,
    ensures
        !digest_binding_valid(accepted_digest, compiled_digest, header_digest, admission_digest),
{
}

fn main() {}

} // verus!
