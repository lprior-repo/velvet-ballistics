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
//! Artifact version barrier integration tests for runtime admission.
//!
//! These tests verify the runtime enforcement of artifact version barriers
//! including schema version validation, gate count requirements, and proof
//! flag verification before run admission.
//!
//! ## Coverage
//!
//! - Schema version barrier: only version == 1 is accepted
//! - Gate count barrier: exactly 15 verification gates required
//! - Proof flag barrier: all 6 proof flags must be true
//! - Digest barrier: artifact digest must match verification proof digest
//!
//! ## Test Philosophy
//!
//! These are **failing-first TDD tests**. The implementation does not exist yet.
//! Each test documents the expected contract and will pass once the runtime
//! artifact version barrier is properly implemented.

use proptest::{prop_assert, prop_assume, proptest};
use vb_compile::{CompileError, CompileErrors, YamlCompiler};
use vb_core::capability::CapabilitySet;
use vb_core::ids::{RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{
    AdmissionError, ArtifactEnvelopeError, MissingAcceptedArtifactStore, admit_artifact_run,
};
use vb_storage::admission::{AcceptedArtifact, VerificationProof};

// ---------------------------------------------------------------------------
// Test fixtures and helpers
// ---------------------------------------------------------------------------

fn empty_digest() -> WorkflowDigest {
    WorkflowDigest::from_bytes([0u8; 32])
}

fn make_artifact_with_gate_count(gate_count: u8, all_flags_true: bool) -> AcceptedArtifact {
    AcceptedArtifact {
        digest: empty_digest(),
        source_digest: empty_digest(),
        policy_digest: empty_digest(),
        ir: Vec::new(),
        verification: VerificationProof {
            digest: empty_digest(),
            gate_count,
            durable: true,
            bounded_claimed: all_flags_true,
            taint_safe_claimed: all_flags_true,
            retry_safe_claimed: all_flags_true,
            idempotency_verified_claimed: all_flags_true,
            replayable_claimed: all_flags_true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: vb_storage::types::EventSeq::ZERO,
        required_capabilities: Box::new([]),
    }
}

fn make_artifact_with_flags(
    gate_count: u8,
    bounded: bool,
    taint_safe: bool,
    retry_safe: bool,
    durable: bool,
    replayable: bool,
    idempotency_verified: bool,
) -> AcceptedArtifact {
    AcceptedArtifact {
        digest: empty_digest(),
        source_digest: empty_digest(),
        policy_digest: empty_digest(),
        ir: Vec::new(),
        verification: VerificationProof {
            digest: empty_digest(),
            gate_count,
            durable,
            bounded_claimed: bounded,
            taint_safe_claimed: taint_safe,
            retry_safe_claimed: retry_safe,
            idempotency_verified_claimed: idempotency_verified,
            replayable_claimed: replayable,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: vb_storage::types::EventSeq::ZERO,
        required_capabilities: Box::new([]),
    }
}

fn make_artifact_with_digest(digest: WorkflowDigest) -> AcceptedArtifact {
    AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir: Vec::new(),
        verification: VerificationProof {
            digest,
            gate_count: 15,
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
        accepted_at_seq: vb_storage::types::EventSeq::ZERO,
        required_capabilities: Box::new([]),
    }
}

/// A store that returns a pre-configured artifact regardless of digest.
#[derive(Debug, Default)]
struct FixedAcceptedArtifactStore {
    artifact: Option<AcceptedArtifact>,
    should_fail_load: bool,
}

impl FixedAcceptedArtifactStore {
    fn new(artifact: AcceptedArtifact) -> Self {
        Self {
            artifact: Some(artifact),
            should_fail_load: false,
        }
    }

    #[allow(dead_code)]
    fn with_failure() -> Self {
        Self {
            artifact: None,
            should_fail_load: true,
        }
    }
}

impl vb_runtime::admission::AcceptedArtifactStore for FixedAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        if self.should_fail_load {
            return Err(ArtifactEnvelopeError::ArtifactNotFound {
                digest: empty_digest(),
            });
        }
        self.artifact
            .clone()
            .ok_or(ArtifactEnvelopeError::ArtifactNotFound {
                digest: empty_digest(),
            })
    }
}

// ---------------------------------------------------------------------------
// Gate Count Barrier Tests
// ---------------------------------------------------------------------------

// B-05: gate_count != 15 must be rejected
#[test]
fn gate_count_zero_rejected() {
    let artifact = make_artifact_with_gate_count(0, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(1),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            AdmissionError::ArtifactInvalidGateCount {
                found: 0,
                required: 15
            }
        ),
        "expected ArtifactInvalidGateCount {{ found: 0, required: 15 }}, got {:?}",
        err
    );
}

#[test]
fn gate_count_fourteen_rejected() {
    let artifact = make_artifact_with_gate_count(14, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(2),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            AdmissionError::ArtifactInvalidGateCount {
                found: 14,
                required: 15
            }
        ),
        "expected ArtifactInvalidGateCount {{ found: 14, required: 15 }}, got {:?}",
        err
    );
}

#[test]
fn gate_count_sixteen_rejected() {
    let artifact = make_artifact_with_gate_count(16, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(3),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            AdmissionError::ArtifactInvalidGateCount {
                found: 16,
                required: 15
            }
        ),
        "expected ArtifactInvalidGateCount {{ found: 16, required: 15 }}, got {:?}",
        err
    );
}

#[test]
fn gate_count_fifteen_accepted() {
    let artifact = make_artifact_with_gate_count(15, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(4),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "gate_count == 15 must be accepted, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Proof Flag Barrier Tests
// ---------------------------------------------------------------------------

// B-06: bounded flag must be true
#[test]
fn bounded_false_rejected() {
    let artifact = make_artifact_with_flags(15, false, true, true, true, true, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(5),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "bounded"),
        "expected ArtifactInvalidProofFlag {{ flag: \"bounded\" }}, got {:?}",
        err
    );
}

// B-07: taint_safe flag must be true
#[test]
fn taint_safe_false_rejected() {
    let artifact = make_artifact_with_flags(15, true, false, true, true, true, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(6),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "taint_safe"),
        "expected ArtifactInvalidProofFlag {{ flag: \"taint_safe\" }}, got {:?}",
        err
    );
}

// B-08: retry_safe flag must be true
#[test]
fn retry_safe_false_rejected() {
    let artifact = make_artifact_with_flags(15, true, true, false, true, true, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(7),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "retry_safe"),
        "expected ArtifactInvalidProofFlag {{ flag: \"retry_safe\" }}, got {:?}",
        err
    );
}

// B-09: durable flag must be true
#[test]
fn durable_false_rejected() {
    let artifact = make_artifact_with_flags(15, true, true, true, false, true, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(8),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "durable"),
        "expected ArtifactInvalidProofFlag {{ flag: \"durable\" }}, got {:?}",
        err
    );
}

// B-10: replayable flag must be true
#[test]
fn replayable_false_rejected() {
    let artifact = make_artifact_with_flags(15, true, true, true, true, false, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(9),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "replayable"),
        "expected ArtifactInvalidProofFlag {{ flag: \"replayable\" }}, got {:?}",
        err
    );
}

// B-11: idempotency_verified flag must be true
#[test]
fn idempotency_verified_false_rejected() {
    let artifact = make_artifact_with_flags(15, true, true, true, true, true, false);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(10),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactInvalidProofFlag { flag } if flag == "idempotency_verified"),
        "expected ArtifactInvalidProofFlag {{ flag: \"idempotency_verified\" }}, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Digest Barrier Tests
// ---------------------------------------------------------------------------

// B-12: digest mismatch must be rejected
#[test]
fn artifact_digest_mismatch_rejected() {
    let requested_digest = empty_digest();
    let different_digest = WorkflowDigest::from_bytes([1u8; 32]);
    let artifact = make_artifact_with_digest(different_digest);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(11),
        requested_digest,
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            AdmissionError::ArtifactDigestMismatch {
                requested,
                found
            } if requested == requested_digest && found == different_digest
        ),
        "expected ArtifactDigestMismatch {{ requested: {:?}, found: {:?} }}, got {:?}",
        requested_digest,
        different_digest,
        err
    );
}

// B-13: digest match must be accepted
#[test]
fn artifact_digest_match_accepted() {
    let digest = WorkflowDigest::from_bytes([42u8; 32]);
    let artifact = make_artifact_with_digest(digest);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(12),
        digest,
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "matching digest must be accepted, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Compile Error Barrier Tests
// ---------------------------------------------------------------------------

// B-15: UnsupportedStepPrimitive error
// Nested collect inside collect body is not supported in Phase 0 compiler.
#[test]
fn unsupported_step_primitive_rejected_at_compile() {
    let yaml = b"version: velvet-ballistics/v1
name: test
when:
  manual: {}
steps:
  - id: collect_pages
    collect:
      variable: page
      source: \"0\"
      steps:
        - id: inner
          collect:
            variable: inner_page
            source: \"1\"
  - id: done
    finish:
      result: 0
";
    let compiler = YamlCompiler::default();
    let result = compiler.compile(yaml);
    assert!(
        matches!(
            result,
            Err(CompileErrors(ref errors)) if errors.first().is_some_and(|e| matches!(e, CompileError::UnsupportedStepPrimitive { .. }))
        ),
        "expected UnsupportedStepPrimitive, got {result:?}"
    );
}

// NOTE: B-16 (ExpressionLoweringUnsupported) is not testable through YamlCompiler::compile
// because the YAML schema validates and coerces values before expression lowering occurs.
// Text constants in expressions trigger ExpressionLoweringUnsupported in the expression
// bytecode layer, but this is only accessible through the expression parser directly,
// not through the YAML compilation path. This behavior is covered by unit tests in
// vb_compile/src/expression_bytecode.rs.

// ---------------------------------------------------------------------------
// Relaxed Policy Tests
// ---------------------------------------------------------------------------

#[test]
fn relaxed_policy_skips_barrier_validation() {
    let artifact = make_artifact_with_gate_count(0, false);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Relaxed,
        RunId::new(13),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "Relaxed policy must skip barrier validation, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Missing Artifact Tests
// ---------------------------------------------------------------------------

#[test]
fn missing_artifact_rejected_under_strict_policy() {
    let store = MissingAcceptedArtifactStore::shared();
    let result = admit_artifact_run(
        store.as_ref(),
        RuntimePolicy::Strict,
        RunId::new(14),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        matches!(result, Err(_)),
        "expected Err result, got {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AdmissionError::ArtifactNotFound { .. }),
        "expected ArtifactNotFound, got {:?}",
        err
    );
}

// ---------------------------------------------------------------------------
// Boundary: All Proof Flags True Together
// ---------------------------------------------------------------------------

#[test]
fn all_proof_flags_true_accepted() {
    let artifact = make_artifact_with_flags(15, true, true, true, true, true, true);
    let store = FixedAcceptedArtifactStore::new(artifact);
    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(15),
        empty_digest(),
        CapabilitySet::empty(),
    );
    assert!(
        result.is_ok(),
        "all proof flags true must be accepted, got {:?}",
        result
    );
}

// ---------------------------------------------------------------------------
// Proptest: Gate Count Invariants
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn validate_gate_count_rejects_invalid_counts(count in proptest::num::u8::ANY) {
        prop_assume!(count != 15, "skipping valid case");
        let artifact = make_artifact_with_gate_count(count, true);
        let store = FixedAcceptedArtifactStore::new(artifact);
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(0),
            empty_digest(),
            CapabilitySet::empty(),
        );
        prop_assert!(result.is_err(), "gate count {} must be rejected", count);
    }

    #[test]
    fn validate_gate_count_accepts_fifteen(count in proptest::strategy::Just(15u8)) {
        let artifact = make_artifact_with_gate_count(count, true);
        let store = FixedAcceptedArtifactStore::new(artifact);
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(0),
            empty_digest(),
            CapabilitySet::empty(),
        );
        prop_assert!(result.is_ok(), "gate count 15 must be accepted");
    }

    #[test]
    fn digest_mismatch_detected_by_admission(d1 in proptest::array::uniform32(0u8..), d2 in proptest::array::uniform32(0u8..)) {
        prop_assume!(d1 != d2);
        let artifact = make_artifact_with_digest(WorkflowDigest::from_bytes(d1));
        let store = FixedAcceptedArtifactStore::new(artifact);
        let result = admit_artifact_run(
            &store,
            RuntimePolicy::Strict,
            RunId::new(0),
            WorkflowDigest::from_bytes(d2),
            CapabilitySet::empty(),
        );
        prop_assert!(result.is_err(), "digest mismatch must be detected");
    }
}
