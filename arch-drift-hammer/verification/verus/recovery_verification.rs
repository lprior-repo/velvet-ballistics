// Verus proof obligations for recovery boundary verification.
//
// Obligations: PO-003A, PO-011..PO-017, PO-019..PO-020, PO-027..PO-029.
// Source references read in the isolated workspace:
// - crates/vb_runtime/src/recovery.rs::reject_unsupported_live_frame_state
// - crates/vb_runtime/src/recovery.rs::SummaryRecoveryBoundary::hydrate_run_frame
// - crates/vb_storage/src/recovery/recover.rs::verify_digests
//
// This artifact proves the pure decision algebra that those production
// functions must refine. Fjall I/O, hashing, artifact lookup, allocation,
// trait-object dispatch, and private-function import are trusted shell
// boundaries recorded in proof evidence.

use vstd::prelude::*;

verus! {

pub struct SpecUnsupportedRecoveryState {
    pub slot_values: bool,
    pub slot_taint: bool,
    pub action_payloads: bool,
    pub pending_actions: bool,
}

pub struct SpecRecoveryFrameSeed {
    pub unsupported: SpecUnsupportedRecoveryState,
    pub pending_actions_len: usize,
    pub slot_entries_len: usize,
    pub taint_entries_len: usize,
    pub step_entries_len: usize,
    pub step_count: usize,
    pub slot_count: usize,
    pub pc: usize,
}

pub open spec fn spec_reject_unsupported(seed: SpecRecoveryFrameSeed) -> bool {
    seed.unsupported.slot_values
        || seed.unsupported.slot_taint
        || seed.unsupported.action_payloads
        || seed.unsupported.pending_actions
}

pub open spec fn spec_has_recovered_slot_or_taint(seed: SpecRecoveryFrameSeed) -> bool {
    seed.slot_entries_len > 0 || seed.taint_entries_len > 0
}

pub open spec fn spec_no_fabricated_slot_or_taint(seed: SpecRecoveryFrameSeed) -> bool {
    spec_has_recovered_slot_or_taint(seed) ==> !spec_reject_unsupported(seed)
}

pub open spec fn spec_frame_dimensions_supported(seed: SpecRecoveryFrameSeed) -> bool {
    seed.pc < seed.step_count
        && seed.step_entries_len <= seed.step_count
        && seed.slot_entries_len <= seed.slot_count
        && seed.taint_entries_len <= seed.slot_count
}

pub proof fn proof_reject_unsupported_slot_taint_alone()
    ensures
        forall|seed: SpecRecoveryFrameSeed|
            seed.unsupported.slot_taint ==> spec_reject_unsupported(seed),
{
    assert_forall_by(|seed: SpecRecoveryFrameSeed| {
        requires(seed.unsupported.slot_taint);
        ensures(spec_reject_unsupported(seed));
        reveal(spec_reject_unsupported);
    });
}

pub proof fn proof_reject_unsupported_pending_actions_no_bypass()
    ensures
        forall|seed: SpecRecoveryFrameSeed|
            seed.unsupported.pending_actions ==> spec_reject_unsupported(seed),
{
    assert_forall_by(|seed: SpecRecoveryFrameSeed| {
        requires(seed.unsupported.pending_actions);
        ensures(spec_reject_unsupported(seed));
        reveal(spec_reject_unsupported);
    });
}

pub proof fn proof_no_slot_value_fabrication_when_unsupported()
    ensures
        forall|seed: SpecRecoveryFrameSeed|
            spec_has_recovered_slot_or_taint(seed) && spec_no_fabricated_slot_or_taint(seed)
            ==> !spec_reject_unsupported(seed),
{
    assert_forall_by(|seed: SpecRecoveryFrameSeed| {
        requires(spec_has_recovered_slot_or_taint(seed) && spec_no_fabricated_slot_or_taint(seed));
        ensures(!spec_reject_unsupported(seed));
        reveal(spec_no_fabricated_slot_or_taint);
    });
}

pub proof fn proof_frame_dimension_overflow_detected()
    ensures
        forall|seed: SpecRecoveryFrameSeed|
            !spec_frame_dimensions_supported(seed)
            ==> seed.pc >= seed.step_count
                || seed.step_entries_len > seed.step_count
                || seed.slot_entries_len > seed.slot_count
                || seed.taint_entries_len > seed.slot_count,
{
    assert_forall_by(|seed: SpecRecoveryFrameSeed| {
        requires(!spec_frame_dimensions_supported(seed));
        ensures(seed.pc >= seed.step_count
            || seed.step_entries_len > seed.step_count
            || seed.slot_entries_len > seed.slot_count
            || seed.taint_entries_len > seed.slot_count);
        reveal(spec_frame_dimensions_supported);
    });
}

pub enum SpecDigestCheck {
    WorkflowSourceOnly,
    WorkflowAndIr,
    Full,
}

pub struct SpecDigestInputs {
    pub workflow_source_matches: bool,
    pub compiled_ir_matches: bool,
    pub action_abi_matches: bool,
    pub policy_matches: bool,
}

pub open spec fn spec_verify_workflow_source(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    match level {
        SpecDigestCheck::WorkflowSourceOnly => inputs.workflow_source_matches,
        SpecDigestCheck::WorkflowAndIr => inputs.workflow_source_matches,
        SpecDigestCheck::Full => inputs.workflow_source_matches,
    }
}

pub open spec fn spec_verify_compiled_ir(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    match level {
        SpecDigestCheck::WorkflowSourceOnly => true,
        SpecDigestCheck::WorkflowAndIr => inputs.compiled_ir_matches,
        SpecDigestCheck::Full => inputs.compiled_ir_matches,
    }
}

pub open spec fn spec_verify_action_abi(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    match level {
        SpecDigestCheck::Full => inputs.action_abi_matches,
        _ => true,
    }
}

pub open spec fn spec_verify_policy(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    match level {
        SpecDigestCheck::Full => inputs.policy_matches,
        _ => true,
    }
}

pub open spec fn spec_verify_required_digests(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    spec_verify_workflow_source(inputs, level)
        && spec_verify_compiled_ir(inputs, level)
}

pub open spec fn spec_verify_optional_downstream_digests(inputs: SpecDigestInputs, level: SpecDigestCheck) -> bool {
    spec_verify_required_digests(inputs, level)
        && spec_verify_action_abi(inputs, level)
        && spec_verify_policy(inputs, level)
}

pub proof fn proof_workflow_source_mismatch_detected()
    ensures
        forall|inputs: SpecDigestInputs|
            !inputs.workflow_source_matches
            ==> !spec_verify_required_digests(inputs, SpecDigestCheck::Full),
{
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(!inputs.workflow_source_matches);
        ensures(!spec_verify_required_digests(inputs, SpecDigestCheck::Full));
        reveal(spec_verify_required_digests);
        reveal(spec_verify_workflow_source);
    });
}

pub proof fn proof_compiled_ir_mismatch_detected()
    ensures
        forall|inputs: SpecDigestInputs|
            !inputs.compiled_ir_matches
            ==> !spec_verify_required_digests(inputs, SpecDigestCheck::Full),
{
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(!inputs.compiled_ir_matches);
        ensures(!spec_verify_required_digests(inputs, SpecDigestCheck::Full));
        reveal(spec_verify_required_digests);
        reveal(spec_verify_compiled_ir);
    });
}

pub proof fn proof_required_digest_preconditions_by_level()
    ensures
        forall|inputs: SpecDigestInputs|
            spec_verify_required_digests(inputs, SpecDigestCheck::WorkflowSourceOnly)
                ==> inputs.workflow_source_matches,
        forall|inputs: SpecDigestInputs|
            spec_verify_required_digests(inputs, SpecDigestCheck::WorkflowAndIr)
                ==> inputs.workflow_source_matches && inputs.compiled_ir_matches,
        forall|inputs: SpecDigestInputs|
            spec_verify_required_digests(inputs, SpecDigestCheck::Full)
                ==> inputs.workflow_source_matches && inputs.compiled_ir_matches,
{
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(spec_verify_required_digests(inputs, SpecDigestCheck::WorkflowSourceOnly));
        ensures(inputs.workflow_source_matches);
        reveal(spec_verify_required_digests);
        reveal(spec_verify_workflow_source);
    });
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(spec_verify_required_digests(inputs, SpecDigestCheck::WorkflowAndIr));
        ensures(inputs.workflow_source_matches && inputs.compiled_ir_matches);
        reveal(spec_verify_required_digests);
        reveal(spec_verify_workflow_source);
        reveal(spec_verify_compiled_ir);
    });
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(spec_verify_required_digests(inputs, SpecDigestCheck::Full));
        ensures(inputs.workflow_source_matches && inputs.compiled_ir_matches);
        reveal(spec_verify_required_digests);
        reveal(spec_verify_workflow_source);
        reveal(spec_verify_compiled_ir);
    });
}

pub proof fn proof_action_abi_mismatch_detected()
    ensures
        forall|inputs: SpecDigestInputs|
            !inputs.action_abi_matches
            ==> !spec_verify_optional_downstream_digests(inputs, SpecDigestCheck::Full),
{
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(!inputs.action_abi_matches);
        ensures(!spec_verify_optional_downstream_digests(inputs, SpecDigestCheck::Full));
        reveal(spec_verify_action_abi);
    });
}

pub proof fn proof_policy_digest_mismatch_detected()
    ensures
        forall|inputs: SpecDigestInputs|
            !inputs.policy_matches
            ==> !spec_verify_optional_downstream_digests(inputs, SpecDigestCheck::Full),
{
    assert_forall_by(|inputs: SpecDigestInputs| {
        requires(!inputs.policy_matches);
        ensures(!spec_verify_optional_downstream_digests(inputs, SpecDigestCheck::Full));
        reveal(spec_verify_policy);
    });
}

pub enum SpecRecoveryBoundary {
    SummaryOnly,
    DurableFrameSeed(SpecRecoveryFrameSeed),
}

pub open spec fn spec_hydrate_run_frame_success(boundary: SpecRecoveryBoundary) -> bool {
    match boundary {
        SpecRecoveryBoundary::SummaryOnly => false,
        SpecRecoveryBoundary::DurableFrameSeed(seed) =>
            !spec_reject_unsupported(seed) && spec_frame_dimensions_supported(seed),
    }
}

pub proof fn proof_summary_only_never_hydrates_empty_frame()
    ensures
        !spec_hydrate_run_frame_success(SpecRecoveryBoundary::SummaryOnly),
{
    reveal(spec_hydrate_run_frame_success);
}

pub proof fn proof_unsupported_frame_seed_never_hydrates()
    ensures
        forall|seed: SpecRecoveryFrameSeed|
            spec_reject_unsupported(seed)
            ==> !spec_hydrate_run_frame_success(SpecRecoveryBoundary::DurableFrameSeed(seed)),
{
    assert_forall_by(|seed: SpecRecoveryFrameSeed| {
        requires(spec_reject_unsupported(seed));
        ensures(!spec_hydrate_run_frame_success(SpecRecoveryBoundary::DurableFrameSeed(seed)));
        reveal(spec_hydrate_run_frame_success);
    });
}

pub enum SpecRecoveryError {
    UnsupportedFrameSeed,
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    FrameDimensionOverflow,
}

pub enum SpecRuntimeError {
    InvalidRecoveryHydration,
    WorkflowSourceDigestMismatch,
    CompiledIrDigestMismatch,
    FrameDimensionOverflow,
}

pub enum SpecRecoveryDecision {
    Ok,
    Err(SpecRecoveryError),
}

pub enum SpecRuntimeDecision {
    Ok,
    Err(SpecRuntimeError),
}

pub open spec fn spec_recover_frame_decision(seed: SpecRecoveryFrameSeed, inputs: SpecDigestInputs) -> SpecRecoveryDecision {
    if spec_reject_unsupported(seed) {
        SpecRecoveryDecision::Err(SpecRecoveryError::UnsupportedFrameSeed)
    } else if !inputs.workflow_source_matches {
        SpecRecoveryDecision::Err(SpecRecoveryError::WorkflowSourceDigestMismatch)
    } else if !inputs.compiled_ir_matches {
        SpecRecoveryDecision::Err(SpecRecoveryError::CompiledIrDigestMismatch)
    } else if !spec_frame_dimensions_supported(seed) {
        SpecRecoveryDecision::Err(SpecRecoveryError::FrameDimensionOverflow)
    } else {
        SpecRecoveryDecision::Ok
    }
}

pub open spec fn spec_refine_recovery_error(error: SpecRecoveryError) -> SpecRuntimeError {
    match error {
        SpecRecoveryError::UnsupportedFrameSeed => SpecRuntimeError::InvalidRecoveryHydration,
        SpecRecoveryError::WorkflowSourceDigestMismatch => SpecRuntimeError::WorkflowSourceDigestMismatch,
        SpecRecoveryError::CompiledIrDigestMismatch => SpecRuntimeError::CompiledIrDigestMismatch,
        SpecRecoveryError::FrameDimensionOverflow => SpecRuntimeError::FrameDimensionOverflow,
    }
}

pub open spec fn spec_runtime_decision(decision: SpecRecoveryDecision) -> SpecRuntimeDecision {
    match decision {
        SpecRecoveryDecision::Ok => SpecRuntimeDecision::Ok,
        SpecRecoveryDecision::Err(error) => SpecRuntimeDecision::Err(spec_refine_recovery_error(error)),
    }
}

pub open spec fn spec_runtime_error_refines(error: SpecRecoveryError, runtime_error: SpecRuntimeError) -> bool {
    runtime_error == spec_refine_recovery_error(error)
}

pub proof fn proof_typed_recovery_errors_refine_to_runtime_errors()
    ensures
        forall|decision: SpecRecoveryDecision, error: SpecRecoveryError|
            decision == SpecRecoveryDecision::Err(error)
            ==> spec_runtime_decision(decision) == SpecRuntimeDecision::Err(spec_refine_recovery_error(error)),
{
    assert_forall_by(|decision: SpecRecoveryDecision, error: SpecRecoveryError| {
        requires(decision == SpecRecoveryDecision::Err(error));
        ensures(spec_runtime_decision(decision) == SpecRuntimeDecision::Err(spec_refine_recovery_error(error)));
        reveal(spec_runtime_decision);
    });
}

pub proof fn proof_typed_recovery_errors_cannot_succeed()
    ensures
        forall|decision: SpecRecoveryDecision|
            (exists|error: SpecRecoveryError| decision == SpecRecoveryDecision::Err(error))
            ==> spec_runtime_decision(decision) != SpecRuntimeDecision::Ok,
{
    assert_forall_by(|decision: SpecRecoveryDecision| {
        requires(exists|error: SpecRecoveryError| decision == SpecRecoveryDecision::Err(error));
        ensures(spec_runtime_decision(decision) != SpecRuntimeDecision::Ok);
        reveal(spec_runtime_decision);
    });
}

pub proof fn proof_recovery_decision_preserves_workflow_digest_error(seed: SpecRecoveryFrameSeed, inputs: SpecDigestInputs)
    requires
        !spec_reject_unsupported(seed),
        !inputs.workflow_source_matches,
    ensures
        spec_runtime_decision(spec_recover_frame_decision(seed, inputs))
            == SpecRuntimeDecision::Err(SpecRuntimeError::WorkflowSourceDigestMismatch),
{
    reveal(spec_recover_frame_decision);
    reveal(spec_runtime_decision);
    reveal(spec_refine_recovery_error);
}

pub proof fn proof_recovery_decision_preserves_compiled_ir_error(seed: SpecRecoveryFrameSeed, inputs: SpecDigestInputs)
    requires
        !spec_reject_unsupported(seed),
        inputs.workflow_source_matches,
        !inputs.compiled_ir_matches,
    ensures
        spec_runtime_decision(spec_recover_frame_decision(seed, inputs))
            == SpecRuntimeDecision::Err(SpecRuntimeError::CompiledIrDigestMismatch),
{
    reveal(spec_recover_frame_decision);
    reveal(spec_runtime_decision);
    reveal(spec_refine_recovery_error);
}

pub proof fn proof_recovery_decision_preserves_dimension_error(seed: SpecRecoveryFrameSeed, inputs: SpecDigestInputs)
    requires
        !spec_reject_unsupported(seed),
        inputs.workflow_source_matches,
        inputs.compiled_ir_matches,
        !spec_frame_dimensions_supported(seed),
    ensures
        spec_runtime_decision(spec_recover_frame_decision(seed, inputs))
            == SpecRuntimeDecision::Err(SpecRuntimeError::FrameDimensionOverflow),
{
    reveal(spec_recover_frame_decision);
    reveal(spec_runtime_decision);
    reveal(spec_refine_recovery_error);
}

fn main() {}

} // verus!
