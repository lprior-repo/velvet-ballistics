// Obligations: PO-005, VERUS-DIAG-004.
// Abstract proof kernel for diagnostic envelope field preservation.

use vstd::prelude::*;

verus! {

pub struct SpecDiagnosticEnvelope {
    pub operation: nat,
    pub boundary: nat,
    pub run_id: nat,
    pub has_run_id: bool,
    pub record_kind: nat,
    pub has_record_kind: bool,
    pub cause: nat,
    pub has_cause: bool,
}

pub open spec fn spec_transform_diagnostic(source: SpecDiagnosticEnvelope) -> SpecDiagnosticEnvelope {
    SpecDiagnosticEnvelope {
        operation: source.operation,
        boundary: source.boundary,
        run_id: source.run_id,
        has_run_id: source.has_run_id,
        record_kind: source.record_kind,
        has_record_kind: source.has_record_kind,
        cause: source.cause,
        has_cause: source.has_cause,
    }
}

pub open spec fn spec_diagnostic_fields_preserved(
    source: SpecDiagnosticEnvelope,
    transformed: SpecDiagnosticEnvelope,
) -> bool {
    transformed.operation == source.operation
        && transformed.boundary == source.boundary
        && transformed.run_id == source.run_id
        && transformed.has_run_id == source.has_run_id
        && transformed.record_kind == source.record_kind
        && transformed.has_record_kind == source.has_record_kind
        && transformed.cause == source.cause
        && transformed.has_cause == source.has_cause
}

pub proof fn proof_diagnostic_envelope_preserves_cause(source: SpecDiagnosticEnvelope)
    ensures
        spec_diagnostic_fields_preserved(source, spec_transform_diagnostic(source)),
        source.has_cause ==> spec_transform_diagnostic(source).has_cause,
        source.has_cause ==> spec_transform_diagnostic(source).cause == source.cause,
{
}

} // verus!

fn main() {}
