// Verus proof obligations for vb-qi37.1.6 recovery hydration contracts.
//
// Obligation: VERUS-REC-001 / PO-002.
// Contract clauses: PRE-004, PRE-006, POST-001, POST-008, INV-001,
// INV-004, INV-005, INV-006.
//
// This is a verification-only abstraction of recovery summary/frame-seed
// construction.  Fjall I/O, decoded slot bytes, snapshot metadata validation,
// and journal ordering are trusted boundaries discharged by TLA+/integration
// lanes; this file proves the Rust-local decision lattice cannot report
// runnable success without complete durable facts, exact taint, bounded
// dimensions, monotonic facts, and fail-closed typed errors.

use vstd::prelude::*;

verus! {

pub enum SpecRecoveryError {
    NoRecoveryData,
    CorruptSnapshot,
    ReplayDivergence,
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    NonIdempotentActionBlocked,
    FrameDimensionOverflow,
    InvalidRecoveryHydration,
    CollectExtraHydrationFailed,
}

pub struct SpecRecoveryInput {
    pub has_header: bool,
    pub has_required_slot: bool,
    pub has_taint: bool,
    pub secret_required: bool,
    pub recovered_secret: bool,
    pub snapshot_valid: bool,
    pub ordered: bool,
    pub tail_after_watermark: bool,
    pub workflow_source_digest_match: bool,
    pub compiled_ir_digest_match: bool,
    pub pending_action: bool,
    pub collect_extra_valid: bool,
    pub runtime_boundary_supported: bool,
    pub dimensions: int,
    pub max_dimensions: int,
    pub fact_erased: bool,
}

pub struct SpecRecoverySuccess {
    pub recovered_secret: bool,
    pub dimensions: int,
}

pub open spec fn dimensions_bounded(input: SpecRecoveryInput) -> bool {
    input.dimensions >= 0
        && input.max_dimensions >= 0
        && input.dimensions <= input.max_dimensions
}

pub open spec fn durable_facts_complete(input: SpecRecoveryInput) -> bool {
    input.has_header
        && input.has_required_slot
        && input.has_taint
        && input.snapshot_valid
        && input.ordered
        && input.tail_after_watermark
        && input.workflow_source_digest_match
        && input.compiled_ir_digest_match
        && input.collect_extra_valid
        && input.runtime_boundary_supported
        && !input.pending_action
        && !input.fact_erased
        && dimensions_bounded(input)
}

pub open spec fn taint_exact(input: SpecRecoveryInput, success: SpecRecoverySuccess) -> bool {
    success.recovered_secret == input.recovered_secret
        && (!input.secret_required || success.recovered_secret)
}

pub open spec fn recovery_decision(input: SpecRecoveryInput) -> Result<SpecRecoverySuccess, SpecRecoveryError> {
    if !input.has_header || !input.has_required_slot || !input.has_taint {
        Err(SpecRecoveryError::NoRecoveryData)
    } else if !input.snapshot_valid {
        Err(SpecRecoveryError::CorruptSnapshot)
    } else if !input.ordered || !input.tail_after_watermark || input.fact_erased {
        Err(SpecRecoveryError::ReplayDivergence)
    } else if !input.workflow_source_digest_match {
        Err(SpecRecoveryError::WorkflowSourceDigestMismatch)
    } else if !input.compiled_ir_digest_match {
        Err(SpecRecoveryError::CompiledIrDigestMismatch)
    } else if input.pending_action {
        Err(SpecRecoveryError::NonIdempotentActionBlocked)
    } else if !input.collect_extra_valid {
        Err(SpecRecoveryError::CollectExtraHydrationFailed)
    } else if !input.runtime_boundary_supported {
        Err(SpecRecoveryError::InvalidRecoveryHydration)
    } else if !dimensions_bounded(input) {
        Err(SpecRecoveryError::FrameDimensionOverflow)
    } else if input.secret_required && !input.recovered_secret {
        Err(SpecRecoveryError::InvalidRecoveryHydration)
    } else {
        Ok(SpecRecoverySuccess { recovered_secret: input.recovered_secret, dimensions: input.dimensions })
    }
}

pub proof fn proof_success_has_complete_durable_facts(input: SpecRecoveryInput)
    ensures
        recovery_decision(input).is_Ok() ==> durable_facts_complete(input),
{
    reveal(recovery_decision);
    reveal(durable_facts_complete);
    reveal(dimensions_bounded);
}

pub proof fn proof_success_has_exact_taint(input: SpecRecoveryInput)
    ensures
        recovery_decision(input).is_Ok() ==> taint_exact(input, recovery_decision(input).get_Ok_0()),
{
    reveal(recovery_decision);
    reveal(taint_exact);
}

pub proof fn proof_missing_secret_taint_fails_closed(input: SpecRecoveryInput)
    requires
        input.secret_required,
        !input.recovered_secret,
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

pub proof fn proof_dimension_overflow_fails_closed(input: SpecRecoveryInput)
    requires
        !dimensions_bounded(input),
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
    reveal(dimensions_bounded);
}

pub proof fn proof_pending_action_fails_closed(input: SpecRecoveryInput)
    requires
        input.has_header,
        input.has_required_slot,
        input.has_taint,
        input.snapshot_valid,
        input.ordered,
        input.tail_after_watermark,
        input.workflow_source_digest_match,
        input.compiled_ir_digest_match,
        input.pending_action,
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

pub proof fn proof_runtime_boundary_unsupported_fails_closed(input: SpecRecoveryInput)
    requires
        input.has_header,
        input.has_required_slot,
        input.has_taint,
        input.snapshot_valid,
        input.ordered,
        input.tail_after_watermark,
        input.workflow_source_digest_match,
        input.compiled_ir_digest_match,
        !input.pending_action,
        input.collect_extra_valid,
        !input.runtime_boundary_supported,
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

pub proof fn proof_digest_mismatch_fails_closed(input: SpecRecoveryInput)
    requires
        input.has_header,
        input.has_required_slot,
        input.has_taint,
        input.snapshot_valid,
        input.ordered,
        input.tail_after_watermark,
        !input.workflow_source_digest_match || !input.compiled_ir_digest_match,
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

pub proof fn proof_monotonic_fact_erasure_fails_closed(input: SpecRecoveryInput)
    requires
        input.fact_erased,
    ensures
        recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

pub proof fn proof_typed_error_totality(input: SpecRecoveryInput)
    ensures
        recovery_decision(input).is_Ok() || recovery_decision(input).is_Err(),
{
    reveal(recovery_decision);
}

fn main() {}

} // verus!
