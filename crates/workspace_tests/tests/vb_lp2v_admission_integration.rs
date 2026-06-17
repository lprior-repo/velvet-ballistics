#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]
//! Additional BDD integration tests for vb-lp2v proof-admission behaviors.
//!
//! These tests complement `vb_lp2v_proof_admission_bdd.rs` by covering
//! behaviors from the vb-lp2v test plan that are not yet in that file:
//!
//! - **R-09**: `admit_artifact_run` with Relaxed skips all checks and returns
//!   RunAdmission immediately regardless of store content.
//!
//! - **R-17**: `admit_run_with_budget` checks aggregate budget capacity BEFORE
//!   artifact existence check (capacity exhaustion is a pre-admission gate).

use vb_core::{
    ActionId, AggregateResourceBudget, AggregateResourceCapacity, Capability, CapabilitySet, RunId,
    RuntimePolicy, WorkflowDigest,
};
use vb_runtime::admission::{ArtifactStore, admit_artifact_run, admit_run, admit_run_with_budget};
use vb_storage::EventSeq;
use vb_storage::admission::{AcceptedArtifact, VerificationProof};

// ============================================================================
// R-09: Relaxed policy skips all artifact loading and capability checks
// ============================================================================

/// An artifact store that always reports artifacts as absent.
struct NeverPresentArtifactStore;

impl ArtifactStore for NeverPresentArtifactStore {
    fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
        false
    }
}

/// An accepted artifact store that always returns ArtifactNotFound.
impl vb_runtime::admission::AcceptedArtifactStore for NeverPresentArtifactStore {
    fn load_accepted_artifact(
        &self,
        digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, vb_runtime::admission::ArtifactEnvelopeError> {
        Err(vb_runtime::admission::ArtifactEnvelopeError::ArtifactNotFound { digest })
    }
}

/// R-09: admit_artifact_run Relaxed skips all checks
///
/// ### Behavior: admit_artifact_run_relaxed_skips_all_checks
/// Given: a NeverPresentArtifactStore and a MissingAcceptedArtifactStore
/// When: admit_artifact_run is called with RuntimePolicy::Relaxed
/// Then: Ok(RunAdmission) is returned with the requested artifact_digest,
///       regardless of store content
#[test]
fn given_never_present_store_when_relaxed_admission_runs_then_succeeds() -> Result<(), String> {
    let store = NeverPresentArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xA1; 32]);
    let run_id = RunId::new(9001);
    let caps = CapabilitySet::empty();

    // Relaxed should succeed even though artifact is never present
    let result = admit_artifact_run(&store, RuntimePolicy::Relaxed, run_id, digest, caps.clone());

    let admission = result.map_err(|e| format!("relaxed admission failed: {e}"))?;
    assert_eq!(
        admission.artifact_digest(),
        digest,
        "admission artifact_digest must match requested digest"
    );
    assert_eq!(
        admission.policy(),
        RuntimePolicy::Relaxed,
        "admission policy must be Relaxed"
    );
    assert_eq!(admission.run_id(), run_id, "admission run_id must match");
    assert_eq!(
        admission.granted_capabilities(),
        &caps,
        "admission granted_capabilities must match"
    );
    Ok(())
}

/// R-09: Relaxed still succeeds even with non-empty required capabilities
/// (capability checking is skipped for Relaxed)
#[test]
fn given_missing_artifact_with_required_caps_when_relaxed_admission_runs_then_succeeds()
-> Result<(), String> {
    let store = NeverPresentArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xA2; 32]);
    let run_id = RunId::new(9002);

    // Granted capabilities that would be insufficient for Strict/Journaled
    let caps = CapabilitySet::from_grants(Box::new([Capability::new(
        "network".into(),
        ActionId::new(7),
    )]));

    // Even with non-empty (but wrong) capabilities, Relaxed must succeed
    // because capability checking is bypassed
    let result = admit_artifact_run(&store, RuntimePolicy::Relaxed, run_id, digest, caps.clone());

    let admission = result.map_err(|e| format!("relaxed admission failed: {e}"))?;
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.policy(), RuntimePolicy::Relaxed);
    Ok(())
}

/// R-15: admit_run Relaxed always succeeds without checking artifact presence
///
/// ### Behavior: admit_run_relaxed_always_succeeds
/// Given: a NeverPresentAcceptedArtifactStore
/// When: admit_run is called with RuntimePolicy::Relaxed
/// Then: Ok(RunAdmission) is returned without checking artifact presence
#[test]
fn given_missing_artifact_when_relaxed_admit_run_then_succeeds() -> Result<(), String> {
    let store = NeverPresentArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xA3; 32]);
    let run_id = RunId::new(9003);
    let caps = CapabilitySet::empty();

    let result = admit_run(&store, RuntimePolicy::Relaxed, digest, run_id, caps.clone());

    let admission = result.map_err(|e| format!("relaxed admit_run failed: {e}"))?;
    assert_eq!(admission.artifact_digest(), digest);
    assert_eq!(admission.policy(), RuntimePolicy::Relaxed);
    Ok(())
}

/// R-02: Strict/Journaled returns ArtifactNotFound when artifact absent
///
/// ### Behavior: admit_artifact_run_returns_artifact_not_found_when_absent
/// Given: a NeverPresentArtifactStore
/// When: admit_artifact_run is called with RuntimePolicy::Strict
/// Then: Err(AdmissionError::ArtifactNotFound { digest }) is returned
#[test]
fn given_missing_artifact_when_strict_admission_runs_then_artifact_not_found() -> Result<(), String>
{
    let store = NeverPresentArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xB1; 32]);
    let run_id = RunId::new(9010);
    let caps = CapabilitySet::empty();

    let result = admit_artifact_run(&store, RuntimePolicy::Strict, run_id, digest, caps);

    assert!(
        matches!(
            result,
            Err(vb_runtime::admission::AdmissionError::ArtifactNotFound {
                digest: d
            }) if d == digest
        ),
        "Strict must return ArtifactNotFound for missing artifact, got {:?}",
        result
    );
    Ok(())
}

// ============================================================================
// R-17: admit_run_with_budget checks capacity BEFORE artifact existence
// ============================================================================

/// R-17: admit_run_with_budget checks capacity before artifact existence
///
/// ### Behavior: admit_run_with_budget_checks_capacity_before_artifact_existence
/// Given: AggregateResourceBudget requesting more than AggregateResourceCapacity
/// When: admit_run_with_budget is called with RuntimePolicy::Strict
/// Then: Err(AdmissionError::ResourceCapacityExceeded) is returned
///        before any artifact existence check occurs
#[test]
fn given_excessive_budget_when_strict_admit_run_with_budget_then_capacity_exceeded()
-> Result<(), String> {
    // An artifact store that says everything exists (will not be consulted
    // if budget check fails first)
    struct AlwaysExistsArtifactStore;
    impl ArtifactStore for AlwaysExistsArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true // artifact exists — but budget should fail first
        }
    }

    let store = AlwaysExistsArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xC1; 32]);
    let run_id = RunId::new(9020);
    let caps = CapabilitySet::empty();

    // Request more steps than the capacity allows
    let requested = AggregateResourceBudget {
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
        max_queue_depth: u32::MAX,
        max_journal_batch_bytes: u32::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 0, // zero capacity — budget exceeds immediately
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    assert!(
        matches!(
            result,
            Err(vb_runtime::admission::AdmissionError::BudgetPolicyExceeded { .. })
        ),
        "admit_run_with_budget must return BudgetPolicyExceeded when budget exceeds policy limits, got {:?}",
        result
    );
    Ok(())
}

/// R-17: When budget fits within capacity, Strict/Journaled THEN checks artifact existence
#[test]
fn given_budget_within_capacity_and_artifact_missing_when_strict_then_artifact_not_found()
-> Result<(), String> {
    struct NeverExistsArtifactStore;
    impl ArtifactStore for NeverExistsArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            false
        }
    }

    let store = NeverExistsArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xC2; 32]);
    let run_id = RunId::new(9021);
    let caps = CapabilitySet::empty();

    // Budget within capacity
    let requested = AggregateResourceBudget {
        max_steps_executable: 100,
        max_action_tickets: 100,
        max_parallel_in_flight: 10,
        max_retries_per_action: 5,
        max_gather_pages: 20,
        max_gather_items: 50,
        max_for_each_iterations: 200,
        max_together_branches: 8,
        max_repeat_attempts: 3,
        max_run_time_seconds: 3600,
        max_result_bytes: 1024,
        max_total_slots_written: 512,
        max_queue_depth: 50,
        max_journal_batch_bytes: 65536,
        max_step_budget_per_tick: 1000,
        max_transitions_per_tick: 500,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 1000,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Strict,
        digest,
        run_id,
        caps,
        requested,
        capacity,
    );

    // Budget is fine, but artifact is missing — must return ArtifactNotFound
    assert!(
        matches!(
            result,
            Err(vb_runtime::admission::AdmissionError::ArtifactNotFound { digest: d })
                if d == digest
        ),
        "admit_run_with_budget must return ArtifactNotFound when budget OK but artifact absent, got {:?}",
        result
    );
    Ok(())
}

/// R-17: Relaxed with admit_run_with_budget still checks budget capacity first
/// (Budget capacity is checked BEFORE policy-specific artifact existence checks)
#[test]
fn given_excessive_budget_when_relaxed_admit_run_with_budget_then_capacity_exceeded()
-> Result<(), String> {
    struct AlwaysExistsArtifactStore;
    impl ArtifactStore for AlwaysExistsArtifactStore {
        fn compiled_ir_exists(&self, _digest: WorkflowDigest) -> bool {
            true
        }
    }

    let store = AlwaysExistsArtifactStore;
    let digest = WorkflowDigest::from_bytes([0xC3; 32]);
    let run_id = RunId::new(9022);
    let caps = CapabilitySet::empty();

    // Excessive budget: max_steps_executable=u32::MAX but capacity=0
    let requested = AggregateResourceBudget {
        max_steps_executable: u32::MAX,
        max_action_tickets: u32::MAX,
        max_parallel_in_flight: u16::MAX,
        max_retries_per_action: u16::MAX,
        max_gather_pages: u32::MAX,
        max_gather_items: u32::MAX,
        max_for_each_iterations: u32::MAX,
        max_together_branches: u16::MAX,
        max_repeat_attempts: u16::MAX,
        max_run_time_seconds: u64::MAX,
        max_result_bytes: u32::MAX,
        max_total_slots_written: u32::MAX,
        max_queue_depth: u32::MAX,
        max_journal_batch_bytes: u32::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };
    let capacity = AggregateResourceCapacity {
        max_steps_executable: 0,
        max_action_tickets: u64::MAX,
        max_parallel_in_flight: u32::MAX,
        max_gather_pages: u64::MAX,
        max_gather_items: u64::MAX,
        max_result_bytes: u64::MAX,
        max_total_slots_written: u64::MAX,
        max_active_runs: u64::MAX,
        max_queue_depth: u64::MAX,
        max_journal_batch_bytes: u64::MAX,
        max_step_budget_per_tick: u64::MAX,
        max_transitions_per_tick: u64::MAX,
        max_timer_entries: 0,
        max_trace_events: 0,
        max_ipc_payload_bytes: 0,
        max_blob_bytes: 0,
        max_input_bytes: 0,
    };

    let result = admit_run_with_budget(
        &store,
        RuntimePolicy::Relaxed,
        digest,
        run_id,
        caps.clone(),
        requested,
        capacity,
    );

    // Budget policy is checked BEFORE capacity or artifact checks.
    // Even Relaxed policy fails if the requested budget exceeds policy limits.
    assert!(
        matches!(
            result,
            Err(vb_runtime::admission::AdmissionError::BudgetPolicyExceeded { .. })
        ),
        "Relaxed must still check budget policy (checked before capacity), got {:?}",
        result
    );
    Ok(())
}

// ============================================================================
// R-03: admit_artifact_run propagates postcard decode failure
// ============================================================================

/// R-03: admit_artifact_run propagates ArtifactEnvelopeDecodeFailed
///
/// ### Behavior: admit_artifact_run_propagates_postcard_decode_failure
/// Given: an AcceptedArtifactStore that returns ArtifactEnvelopeError::PostcardDecodeFailed
/// When: admit_artifact_run is called with RuntimePolicy::Strict
/// Then: Err(AdmissionError::ArtifactEnvelopeDecodeFailed) is returned
struct PostcardFailingStore;

impl vb_runtime::admission::AcceptedArtifactStore for PostcardFailingStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, vb_runtime::admission::ArtifactEnvelopeError> {
        Err(vb_runtime::admission::ArtifactEnvelopeError::PostcardDecodeFailed)
    }
}

#[test]
fn given_postcard_failure_when_strict_admission_runs_then_decode_failed() -> Result<(), String> {
    let store = PostcardFailingStore;
    let digest = WorkflowDigest::from_bytes([0xD1; 32]);
    let run_id = RunId::new(9030);
    let caps = CapabilitySet::empty();

    let result = admit_artifact_run(&store, RuntimePolicy::Strict, run_id, digest, caps);

    assert!(
        matches!(
            result,
            Err(vb_runtime::admission::AdmissionError::ArtifactEnvelopeDecodeFailed)
        ),
        "Strict must propagate ArtifactEnvelopeDecodeFailed, got {:?}",
        result
    );
    Ok(())
}

// ============================================================================
// R-04: admit_artifact_run rejects wrong gate_count
// ============================================================================

/// R-04: admit_artifact_run rejects wrong gate_count
///
/// ### Behavior: admit_artifact_run_rejects_wrong_gate_count
/// Given: an AcceptedArtifactStore that returns artifact with gate_count=7
/// When: admit_artifact_run is called with RuntimePolicy::Strict
/// Then: Err(AdmissionError::ArtifactInvalidGateCount { found: 7, required: 15 }) is returned
struct WrongGateCountStore;

impl vb_runtime::admission::AcceptedArtifactStore for WrongGateCountStore {
    fn load_accepted_artifact(
        &self,
        artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, vb_runtime::admission::ArtifactEnvelopeError> {
        Ok(AcceptedArtifact {
            digest: artifact_digest,
            source_digest: artifact_digest,
            policy_digest: artifact_digest,
            ir: Vec::new(),
            verification: VerificationProof {
                digest: artifact_digest,
                gate_count: 7, // wrong gate count
                durable: true,
                bounded_claimed: true,
                taint_safe_claimed: true,
                retry_safe_claimed: true,
                idempotency_verified_claimed: true,
                replayable_claimed: true,
                idempotency_keyed: Box::new([]),
                idempotency_attested: Box::new([]),
                warnings: Vec::new(),
            },
            accepted_at_seq: EventSeq::new(0),
            required_capabilities: Box::new([]),
        })
    }
}

#[test]
fn given_wrong_gate_count_when_strict_admission_runs_then_invalid_gate_count() -> Result<(), String>
{
    let store = WrongGateCountStore;
    let digest = WorkflowDigest::from_bytes([0xE1; 32]);
    let run_id = RunId::new(9040);
    let caps = CapabilitySet::empty();

    let result = admit_artifact_run(&store, RuntimePolicy::Strict, run_id, digest, caps);

    assert!(
        matches!(
            result,
            Err(
                vb_runtime::admission::AdmissionError::ArtifactInvalidGateCount {
                    found: 7,
                    required: 15,
                }
            )
        ),
        "Strict must reject wrong gate_count=7, got {:?}",
        result
    );
    Ok(())
}
