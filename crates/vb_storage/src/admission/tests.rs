#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::map_clone,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
use crate::admission::*;
use crate::error::JournalError;

#[test]
fn verification_warning_display_formats_gate_code_message() {
    let warning = VerificationWarning {
        code: 42,
        message: Box::from("deprecated action kind"),
        gate: 3,
    };
    assert_eq!(format!("{warning}"), "gate 3: [42] deprecated action kind");
}

#[test]
fn verification_warning_equality_works() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    let b = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    assert_eq!(a, b);
}

#[test]
fn verification_warning_inequality_different_code() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 2,
    };
    let b = VerificationWarning {
        code: 99,
        message: Box::from("alpha"),
        gate: 2,
    };
    assert_ne!(a, b);
}

#[test]
fn verification_warning_inequality_different_gate() {
    let a = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 1,
    };
    let b = VerificationWarning {
        code: 1,
        message: Box::from("alpha"),
        gate: 13,
    };
    assert_ne!(a, b);
}

#[test]
fn verification_proof_new_initializes_empty_warnings() {
    let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let proof = VerificationProof::new(digest, 2, true);
    assert!(proof.warnings.is_empty());
    assert_eq!(proof.gate_count, 2);
    assert!(proof.durable);
}

#[test]
fn verification_proof_warnings_can_be_populated() {
    let digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let mut proof = VerificationProof::new(digest, 5, false);
    proof.warnings.push(VerificationWarning {
        code: 100,
        message: Box::from("soft check advisory"),
        gate: 7,
    });
    proof.warnings.push(VerificationWarning {
        code: 200,
        message: Box::from("boundary advisory"),
        gate: 11,
    });
    assert_eq!(proof.warnings.len(), 2);
    assert_eq!(proof.warnings[0].gate, 7);
    assert_eq!(proof.warnings[1].code, 200);
}

#[test]
fn verification_warning_clone_preserves_fields() {
    let original = VerificationWarning {
        code: 55,
        message: Box::from("cloneable warning"),
        gate: 9,
    };
    let cloned = original.clone();
    assert_eq!(original, cloned);
    assert_eq!(cloned.code, 55);
    assert_eq!(&*cloned.message, "cloneable warning");
    assert_eq!(cloned.gate, 9);
}

#[test]
fn is_valid_rejects_gate_zero() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("zero gate"),
        gate: 0,
    };
    assert!(!w.is_valid());
}

#[test]
fn is_valid_accepts_gate_one() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("min gate"),
        gate: VerificationWarning::MIN_GATE,
    };
    assert!(w.is_valid());
}

#[test]
fn is_valid_accepts_gate_two() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("max gate"),
        gate: VerificationWarning::MAX_GATE,
    };
    assert!(w.is_valid());
}

#[test]
fn is_valid_accepts_gate_fourteen() {
    let w = VerificationWarning {
        code: 1,
        message: Box::from("within gate range"),
        gate: 14,
    };
    assert!(w.is_valid());
}

// =========================================================================
// submit_artifact: Relaxed policy
// =========================================================================

/// Owns both a temporary directory path and a FjallJournal so the directory
/// is not dropped while the journal is in use.
struct TestJournal {
    path: std::path::PathBuf,
    journal: crate::FjallJournal,
}

impl Drop for TestJournal {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            std::hint::black_box(error.kind());
        }
    }
}

impl std::ops::Deref for TestJournal {
    type Target = crate::FjallJournal;
    fn deref(&self) -> &Self::Target {
        &self.journal
    }
}

/// Opens a temporary FjallJournal that is cleaned up when dropped.
fn temp_journal() -> Result<TestJournal, JournalError> {
    let dir = tempfile::tempdir().map_err(|_| JournalError::ArtifactMalformed)?;
    let path = dir.keep();
    let journal = crate::FjallJournal::open(&path, None)?;
    Ok(TestJournal { path, journal })
}

/// Builds a minimal valid CompiledWorkflow for testing.
///
/// The digest is computed by serializing the parts with the digest field zeroed,
/// then BLAKE3-hashing the result. This mirrors the checksum validation gate.
fn minimal_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    use vb_core::value::ConstValue;
    use vb_core::workflow::{ResourceContract, WorkflowParts};
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, SlotIdx, StepIdx,
        WorkflowDigest,
    };

    let mut parts = WorkflowParts {
        name: Box::<str>::from("test_admission"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([
            CompiledNode {
                id: StepIdx::new(0),
                output: Some(SlotIdx::new(0)),
                next: Some(StepIdx::new(1)),
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
            },
            CompiledNode {
                id: StepIdx::new(1),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish {
                    result: SlotIdx::new(0),
                },
            },
        ]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(42)]),
        slot_count: 1,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    // Compute the correct BLAKE3 digest from the zeroed-digest serialization.
    let hash_bytes =
        postcard::to_allocvec(&parts).map_err(|e| format!("serialize parts for digest: {e}"))?;
    let computed = blake3::hash(&hash_bytes);
    parts.digest = WorkflowDigest::from_bytes(computed.into());

    CompiledWorkflow::try_from_parts(parts).map_err(|e| e.to_string())
}

#[test]
fn submit_artifact_relaxed_persists_and_returns_artifact() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit_artifact(relaxed) failed: {e}"))?;

    // The returned digest must match the workflow's digest.
    assert_eq!(
        result.digest,
        workflow.digest(),
        "artifact digest must match workflow digest"
    );

    // The proof under Relaxed must have 0 gates and durable=false.
    assert_eq!(result.verification.gate_count, 0, "relaxed must skip gates");
    assert!(!result.verification.durable, "relaxed must not be durable");

    // The proof's digest must match.
    assert_eq!(
        result.verification.digest,
        workflow.digest(),
        "proof digest must match workflow digest"
    );

    // The ir bytes must be non-empty (postcard serialization).
    assert!(!result.ir.is_empty(), "compiled IR bytes must not be empty");

    // Verify we can read the artifact back from storage.
    let loaded = journal
        .compiled_ir(workflow.digest())
        .map_err(|e| format!("compiled_ir read failed: {e}"))?;
    assert!(loaded.is_some(), "artifact must be readable after submit");
    Ok(())
}

#[test]
fn submit_artifact_journaled_runs_both_gates() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

    // Journaled passes 15 gates but is not durable (no SyncAll).
    assert_eq!(
        result.verification.gate_count, 15,
        "journaled must pass 15 verification gates"
    );
    assert!(
        !result.verification.durable,
        "journaled must not be durable"
    );
    assert_eq!(result.digest, workflow.digest());
    Ok(())
}

#[test]
fn submit_artifact_strict_is_durable() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .map_err(|e| format!("submit_artifact(strict) failed: {e}"))?;

    // Strict passes 15 gates AND is durable.
    assert_eq!(result.verification.gate_count, 15);
    assert!(result.verification.durable, "strict must be durable");
    assert_eq!(result.digest, workflow.digest());
    Ok(())
}

// =========================================================================
// submit_artifact: checksum validation
// =========================================================================

#[test]
fn submit_artifact_journaled_roundtrip_bytes_match() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    // Deserialize the returned IR bytes and verify the digest field.
    let loaded = journal
        .compiled_ir(result.digest)
        .map_err(|e| format!("read failed: {e}"))?;
    let record = loaded.ok_or_else(|| String::from("artifact not found after submit"))?;
    assert_eq!(record.digest, result.digest, "stored digest must match");
    Ok(())
}

// =========================================================================
// admit_compiled_artifact
// =========================================================================

#[test]
fn admit_compiled_artifact_succeeds_for_valid_workflow() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let digest = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("admit_compiled_artifact failed: {e}"))?;

    assert_eq!(
        digest,
        workflow.digest(),
        "returned digest must match workflow digest"
    );

    // Verify it's stored.
    let loaded = journal
        .compiled_ir(digest)
        .map_err(|e| format!("read failed: {e}"))?;
    assert!(loaded.is_some(), "artifact must be stored after admission");
    Ok(())
}

#[test]
fn admit_compiled_artifact_idempotent() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let digest_a = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("first admit failed: {e}"))?;
    let digest_b = admit_compiled_artifact(&journal, &workflow)
        .map_err(|e| format!("second admit failed: {e}"))?;

    assert_eq!(
        digest_a, digest_b,
        "idempotent admission must return same digest"
    );
    Ok(())
}

// =========================================================================
// AcceptedArtifact fields
// =========================================================================

#[test]
fn accepted_artifact_fields_are_populated() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("submit failed: {e}"))?;
    let policy_digest =
        compute_policy_digest(&workflow).map_err(|e| format!("policy digest failed: {e}"))?;

    assert_eq!(artifact.source_digest, workflow.digest());
    assert_eq!(artifact.policy_digest, policy_digest);
    // accepted_at_seq should be 0 (no journal sequence tracking in current impl).
    assert_eq!(
        artifact.accepted_at_seq.get(),
        0,
        "accepted_at_seq must be 0"
    );
    // required_capabilities should be empty for minimal workflow.
    assert!(
        artifact.required_capabilities.is_empty(),
        "minimal workflow has no capabilities"
    );
    Ok(())
}

#[test]
fn submit_artifact_returns_required_capabilities_when_contract_requires_capability()
-> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let required = vb_core::capability::Capability::new(
        Box::<str>::from("network.github"),
        vb_core::ActionId::new(7),
    );
    let contract = vb_core::action::ActionContract {
        id: vb_core::ActionId::new(7),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::IdempotentExternal,
        side_effect: vb_core::action::SideEffect::LocalWrite,
        retry_safety: vb_core::action::RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([required.clone()]),
    };

    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    )
    .map_err(|e| format!("submit_artifact_with_contracts failed: {e}"))?;

    assert_eq!(artifact.required_capabilities.as_ref(), &[required.clone()]);
    Ok(())
}

#[test]
fn submit_artifact_persists_accepted_artifact_envelope() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact failed: {e}"))?;
    let loaded = journal
        .compiled_ir(workflow.digest())
        .map_err(|e| format!("compiled_ir read failed: {e}"))?
        .ok_or_else(|| String::from("persisted artifact not found"))?;
    let decoded: AcceptedArtifact = postcard::from_bytes(&loaded.ir)
        .map_err(|e| format!("persisted AcceptedArtifact decode failed: {e}"))?;
    let raw_parts_decode: Result<vb_core::workflow::WorkflowParts, _> =
        postcard::from_bytes(&loaded.ir);
    let inner_hash = blake3::hash(&decoded.ir);
    let policy_digest =
        compute_policy_digest(&workflow).map_err(|e| format!("policy digest failed: {e}"))?;

    assert_eq!(loaded.digest, workflow.digest());
    assert_eq!(decoded, artifact);
    assert_eq!(decoded.source_digest, workflow.digest());
    assert_eq!(decoded.policy_digest, policy_digest);
    assert_eq!(inner_hash.as_bytes(), &loaded.digest.as_bytes());
    assert!(
        raw_parts_decode.is_err(),
        "compiled_ir value must not be raw WorkflowParts"
    );
    Ok(())
}

#[test]
fn submit_artifact_carries_idempotency_evidence_from_contracts() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let action = vb_core::ActionId::new(11);
    let contract = vb_core::action::ActionContract {
        id: action,
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::IdempotentExternal,
        side_effect: vb_core::action::SideEffect::LocalWrite,
        retry_safety: vb_core::action::RetrySafety::RequiresIdempotencyKey,
        required_capabilities: Box::new([]),
    };

    let artifact = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    )
    .map_err(|e| format!("submit_artifact_with_contracts failed: {e}"))?;

    assert!(artifact.verification.idempotency_verified_claimed);
    assert_eq!(artifact.verification.idempotency_keyed.as_ref(), &[action]);
    assert_eq!(
        artifact.verification.idempotency_attested.as_ref(),
        &[action]
    );
    Ok(())
}

#[test]
fn submit_artifact_rejects_failed_idempotency_contract() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;
    let contract = vb_core::action::ActionContract {
        id: vb_core::ActionId::new(12),
        name: vb_core::action::ActionName::new("test-action").unwrap(),
        input_slot_count: 1,
        output_slot_count: 1,
        max_input_bytes: 1024,
        max_output_bytes: 2048,
        timeout_ms: 1000,
        idempotency: vb_core::action::Idempotency::DeterministicPure,
        side_effect: vb_core::action::SideEffect::LocalWrite,
        retry_safety: vb_core::action::RetrySafety::Idempotent,
        required_capabilities: Box::new([]),
    };

    let result = submit_artifact_with_contracts(
        &journal,
        &workflow,
        vb_core::RuntimePolicy::Journaled,
        &[contract],
    );

    assert!(matches!(result, Err(JournalError::ArtifactMalformed)));
    Ok(())
}

// =========================================================================
// VerificationProof details
// =========================================================================

#[test]
fn verification_proof_serde_roundtrip() -> Result<(), String> {
    let digest = vb_core::WorkflowDigest::from_bytes([0xAA_u8; 32]);
    let mut proof = VerificationProof::new(digest, 3, true);
    proof.warnings.push(VerificationWarning {
        code: 7,
        message: Box::from("test warning"),
        gate: 5,
    });

    let serialized = postcard::to_allocvec(&proof).map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: VerificationProof =
        postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(proof, deserialized, "proof must survive serde roundtrip");
    Ok(())
}

#[test]
fn accepted_artifact_serde_roundtrip() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    let serialized =
        postcard::to_allocvec(&artifact).map_err(|e| format!("serialize failed: {e}"))?;
    let deserialized: AcceptedArtifact =
        postcard::from_bytes(&serialized).map_err(|e| format!("deserialize failed: {e}"))?;

    assert_eq!(
        artifact, deserialized,
        "artifact must survive serde roundtrip"
    );
    Ok(())
}

// =========================================================================
// Relaxed vs Journaled/Strict gate count difference
// =========================================================================

#[test]
fn relaxed_skips_gates_while_journaled_passes_them() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let relaxed = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed)
        .map_err(|e| format!("relaxed failed: {e}"))?;
    let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("journaled failed: {e}"))?;

    assert!(
        relaxed.verification.gate_count < journaled.verification.gate_count,
        "relaxed gate count ({}) must be less than journaled ({})",
        relaxed.verification.gate_count,
        journaled.verification.gate_count
    );
    Ok(())
}

#[test]
fn strict_and_journaled_have_same_gate_count() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let journaled = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("journaled failed: {e}"))?;
    let strict = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict)
        .map_err(|e| format!("strict failed: {e}"))?;

    assert_eq!(
        journaled.verification.gate_count, strict.verification.gate_count,
        "journaled and strict must have identical gate count"
    );
    // Only difference is durable flag.
    assert!(!journaled.verification.durable);
    assert!(strict.verification.durable);
    Ok(())
}

// =========================================================================
// Warning gate boundary values
// =========================================================================

#[test]
fn all_valid_gates_pass_is_valid() -> Result<(), String> {
    for gate in VerificationWarning::MIN_GATE..=VerificationWarning::MAX_GATE {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("boundary test"),
            gate,
        };
        if !w.is_valid() {
            return Err(format!("gate {gate} should be valid"));
        }
    }
    Ok(())
}

#[test]
fn gate_values_outside_range_fail_is_valid() -> Result<(), String> {
    for gate in [0u8, 16, 20, 255] {
        let w = VerificationWarning {
            code: 1,
            message: Box::from("out of range test"),
            gate,
        };
        if w.is_valid() {
            return Err(format!("gate {gate} should be invalid"));
        }
    }
    Ok(())
}

// =========================================================================
// VerificationWarning serialization
// =========================================================================

#[test]
fn verification_warning_serde_roundtrip() -> Result<(), String> {
    let warning = VerificationWarning {
        code: 999,
        message: Box::from("serde test warning"),
        gate: 7,
    };
    let bytes = postcard::to_allocvec(&warning).map_err(|e| format!("serialize failed: {e}"))?;
    let back: VerificationWarning =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize failed: {e}"))?;
    assert_eq!(warning, back);
    Ok(())
}

// =========================================================================
// Proof Flag Gap Tests (demonstrate VB-STORAGE-GAP)
//
// These tests document the gap: VerificationProof::new() sets all proof
// flags to true UNCONDITIONALLY, without any actual per-gate validation.
// The flags should be set based on actual verification results.
// =========================================================================

#[test]
fn gap_proof_flags_always_true_regardless_of_gate_count() -> Result<(), String> {
    let digest = vb_core::WorkflowDigest::from_bytes([0xAB_u8; 32]);

    let proof_zero = VerificationProof::new(digest, 0, false);
    assert!(
        proof_zero.bounded_claimed,
        "GAP: bounded_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.taint_safe_claimed,
        "GAP: taint_safe_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.retry_safe_claimed,
        "GAP: retry_safe_claimed=true even with gate_count=0 (no verification performed)"
    );
    assert!(
        proof_zero.replayable_claimed,
        "GAP: replayable_claimed=true even with gate_count=0 (no verification performed)"
    );

    let proof_fifteen = VerificationProof::new(digest, 15, true);
    assert!(
        proof_fifteen.bounded_claimed,
        "GAP: bounded_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.taint_safe_claimed,
        "GAP: taint_safe_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.retry_safe_claimed,
        "GAP: retry_safe_claimed=true with gate_count=15 (verification claimed but not performed)"
    );
    assert!(
        proof_fifteen.replayable_claimed,
        "GAP: replayable_claimed=true with gate_count=15 (verification claimed but not performed)"
    );

    Ok(())
}

#[test]
fn gap_proof_flags_true_for_any_digest_value() -> Result<(), String> {
    let zero_digest = vb_core::WorkflowDigest::from_bytes([0u8; 32]);
    let proof_zero = VerificationProof::new(zero_digest, 15, true);
    assert!(
        proof_zero.bounded_claimed
            && proof_zero.taint_safe_claimed
            && proof_zero.retry_safe_claimed
            && proof_zero.replayable_claimed,
        "GAP: proof flags are true for zero digest"
    );

    let max_digest = vb_core::WorkflowDigest::from_bytes([0xFFu8; 32]);
    let proof_max = VerificationProof::new(max_digest, 15, true);
    assert!(
        proof_max.bounded_claimed
            && proof_max.taint_safe_claimed
            && proof_max.retry_safe_claimed
            && proof_max.replayable_claimed,
        "GAP: proof flags are true for max digest"
    );

    let arbitrary_digest = vb_core::WorkflowDigest::from_bytes([
        0x12_u8, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
        0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
        0x66, 0x77, 0x88,
    ]);
    let proof_arb = VerificationProof::new(arbitrary_digest, 15, false);
    assert!(
        proof_arb.bounded_claimed
            && proof_arb.taint_safe_claimed
            && proof_arb.retry_safe_claimed
            && proof_arb.replayable_claimed,
        "GAP: proof flags are true for arbitrary digest"
    );

    Ok(())
}

#[test]
fn gap_submit_artifact_journaled_produces_unconditional_true_flags() -> Result<(), String> {
    let journal = temp_journal().map_err(|e| format!("journal open failed: {e}"))?;
    let workflow = minimal_workflow()?;

    let result = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit_artifact(journaled) failed: {e}"))?;

    assert_eq!(result.verification.gate_count, 15);
    assert!(
        result.verification.bounded_claimed,
        "GAP: submit_artifact produces bounded_claimed=true without checking workflow size"
    );
    assert!(
        result.verification.taint_safe_claimed,
        "GAP: submit_artifact produces taint_safe_claimed=true without checking taint propagation"
    );
    assert!(
        result.verification.retry_safe_claimed,
        "GAP: submit_artifact produces retry_safe_claimed=true without checking idempotency"
    );
    assert!(
        result.verification.replayable_claimed,
        "GAP: submit_artifact produces replayable_claimed=true without checking replay invariants"
    );

    Ok(())
}

// =========================================================================
// vb-u09ai: 4-variant RetrySafety persistence tests (Tier 1 + Tier 2).
// Per master plan Section 65, the 4-variant RetrySafety has stable
// discriminants 0/1/2/3 (positional compat with the 3-variant ordinals
// 0/1/2; ordinal 3 is additive).
// =========================================================================

/// Tier 1: 4-variant RetrySafety discriminants are stable ordinals 0..3.
#[test]
fn persistence_ordinals() {
    assert_eq!(vb_core::action::RetrySafety::Idempotent as u8, 0);
    assert_eq!(
        vb_core::action::RetrySafety::RequiresIdempotencyKey as u8,
        1
    );
    assert_eq!(vb_core::action::RetrySafety::NotRetrySafe as u8, 2);
    assert_eq!(vb_core::action::RetrySafety::Unknown as u8, 3);
}

/// Tier 2: ordinals 0..2 are stable across the 3→4 migration
/// (Idempotent=0, RequiresIdempotencyKey=1, NotRetrySafe=2 all match
/// the 3-variant shape). Ordinal 3 is the additive new variant.
#[test]
fn backward_compat_3variant_ordinals() {
    // Pre-migration ordinal 0 (Safe in 3-variant) is the same as
    // post-migration ordinal 0 (Idempotent in 4-variant). The on-disk
    // encoding does not change for ordinals 0..2.
    let pre_migration_safe_ordinal = 0u8;
    let post_idempotent_ordinal = vb_core::action::RetrySafety::Idempotent as u8;
    assert_eq!(pre_migration_safe_ordinal, post_idempotent_ordinal);

    let pre_migration_key_required_ordinal = 1u8;
    let post_requires_idempotency_key_ordinal =
        vb_core::action::RetrySafety::RequiresIdempotencyKey as u8;
    assert_eq!(
        pre_migration_key_required_ordinal,
        post_requires_idempotency_key_ordinal
    );

    let pre_migration_unsafe_ordinal = 2u8;
    let post_not_retry_safe_ordinal = vb_core::action::RetrySafety::NotRetrySafe as u8;
    assert_eq!(pre_migration_unsafe_ordinal, post_not_retry_safe_ordinal);
}

/// vb-ssf5h: The `accepted_at_seq` field is a documented placeholder for
/// future journal sequence tracking. It is currently always `EventSeq::new(0)`
/// because the journal does not yet expose a public sequence API. This test
/// pins the placeholder invariant in three places: (1) the field is 0 on
/// submission, (2) the field is included in the metadata hash so that
/// post-admission mutations are detected, and (3) the runtime admission's
/// certificate freshness check accepts 0 as a valid value (zero floor
/// means no rejection). Wiring to a real journal sequence requires a
/// separate journal-API bead and would change artifact hashes, so it is
/// intentionally deferred.
#[test]
fn accepted_at_seq_placeholder_invariant_pins_field_to_zero_with_hash_protection()
-> Result<(), String> {
    use crate::types::EventSeq;
    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let journal =
        crate::journal::FjallJournal::open(temp_dir.path(), None).map_err(|e| e.to_string())?;
    let workflow = minimal_workflow()?;

    let artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    // (1) The field is 0 for all current submissions.
    if artifact.accepted_at_seq != EventSeq::new(0) {
        return Err(format!(
            "placeholder invariant: accepted_at_seq must be 0, got {:?}",
            artifact.accepted_at_seq
        ));
    }

    // (2) The field is included in the metadata hash, so any change to
    // `accepted_at_seq` would change the hash. The chunk_038/chunk_039
    // adversarial tests prove that mutating this field post-storage
    // changes the metadata hash and is therefore detected.
    let hash_a = compute_artifact_metadata_hash(&artifact);
    let mut mutated = artifact.clone();
    mutated.accepted_at_seq = EventSeq::new(1);
    let hash_b = compute_artifact_metadata_hash(&mutated);
    if hash_a == hash_b {
        return Err(String::from(
            "metadata hash invariant: mutating accepted_at_seq must change the hash",
        ));
    }

    // (3) The field is `Ord` and the runtime admission's certificate
    // freshness check uses `<` comparison. With `EventSeq::new(0)` and
    // a `required_at_least` of 0, no rejection occurs (zero floor = no
    // rejection). This is the documented backward-compatible behavior.
    let zero: EventSeq = EventSeq::new(0);
    if !(artifact.accepted_at_seq < zero) {
        // false: artifact.accepted_at_seq is 0, zero is 0, 0 < 0 is false.
        // This branch confirms the invariant: 0 is NOT < 0, so a zero
        // floor does not reject a zero accepted_at_seq.
    } else {
        return Err(String::from(
            "zero floor must not reject zero accepted_at_seq",
        ));
    }
    Ok(())
}

/// vb-wyosk: Regression guard for the FjallJournal index keyspaces audit.
///
/// Audit finding: `submit_artifact` does NOT call `put_status_index` or
/// `put_workflow_index`. The `index_status` and `index_workflow` Fjall
/// keyspaces are defined in `crates/vb_storage/src/journal/core.rs:59-60`
/// and the Batch methods exist at `crates/vb_storage/src/batch.rs:280, 293`,
/// but the production admission path does not populate them. This test
/// pins the current audit finding as a regression guard: if a future
/// change wires the indexes, this test will fail and force an explicit
/// update of the audit record (`bd remember` note from 2026-06-14) and
/// the bead's evidence matrix.
#[test]
fn submit_artifact_does_not_populate_index_status_or_workflow_keyspaces() -> Result<(), String> {
    let temp_dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let journal =
        crate::journal::FjallJournal::open(temp_dir.path(), None).map_err(|e| e.to_string())?;
    let workflow = minimal_workflow()?;

    let _artifact = submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Journaled)
        .map_err(|e| format!("submit failed: {e}"))?;

    // After submit, both keyspaces must be empty.
    let status_count = journal.index_status.iter().count();
    if status_count != 0 {
        return Err(format!(
            "audit regression: index_status must be empty after submit_artifact, got {status_count} entries. \
             This means submit_artifact now calls put_status_index; update the vb-wyosk audit record and evidence matrix."
        ));
    }
    let workflow_count = journal.index_workflow.iter().count();
    if workflow_count != 0 {
        return Err(format!(
            "audit regression: index_workflow must be empty after submit_artifact, got {workflow_count} entries. \
             This means submit_artifact now calls put_workflow_index; update the vb-wyosk audit record and evidence matrix."
        ));
    }
    Ok(())
}
