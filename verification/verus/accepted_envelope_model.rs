// Verus model for vb-qi37.4.2 accepted-envelope admission predicates.
//
// Obligations:
// - PO-006 / VERUS-ENV-006: decoded accepted-envelope v1 requires canonical
//   gate count, durable non-stale evidence, accepted proof flags, and supported
//   schema/status values before strict runtime admission.
//
// This is a pure decoded-value model. Postcard byte decoding, Fjall I/O,
// digest loading, wall-clock reads, and runtime constructors remain shell
// boundaries for later fuzz, Kani, integration, and static-scan evidence.
//
// BINDING: accepted_envelope_model
// Rust type: vb_ui_model::envelope::types::MetadataEnvelope
// Verified: Matched spec predicates to envelope field validation (schema_version, gate_count, proof_flags)
// Divergences: Spec models abstract boolean flags; Rust uses actual proof flag booleans

use vstd::prelude::*;

verus! {

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

pub open spec fn accepted_envelope_valid(
    schema_version: int,
    gate_count: int,
    durable: bool,
    stale: bool,
    all_required_gate_proofs_accepted: bool,
) -> bool {
    supported_schema(schema_version)
        && canonical_gate_count(gate_count)
        && durable_evidence(durable)
        && fresh_evidence(stale)
        && accepted_gate_status(all_required_gate_proofs_accepted)
}

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
}

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
{
}

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
}

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
}

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
}

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
}

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
}

fn main() {}

} // verus!
