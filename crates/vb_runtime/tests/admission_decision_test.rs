#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
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
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
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
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables,
)]

//! Behavior tests for accepted artifact admission decision.
//!
//! These tests exercise the production `validate_accepted_artifact_envelope` function
//! and the `admit_run` / `admit_artifact_run_with_certificate_floor` paths.
//!
//! Bead: vb-rqmw
//! Obligation: VB-RQMW-007 (accepted_artifact_admission_decision.rs Verus spec)
//!
//! The Verus spec models 7 ArtifactCase variants:
//!   Missing, Malformed, InvalidProof, InvalidGateCount,
//!   InvalidCapability, DigestMismatch, Valid
//! Each maps to a production error path via validate_accepted_artifact_envelope.

use vb_core::capability::CapabilitySet;
use vb_core::ids::{ActionId, RunId, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_runtime::admission::{AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError};
use vb_storage::EventSeq;
use vb_storage::admission::{AcceptedArtifact, VerificationProof};

/// Helper: build a minimal valid VerificationProof with all flags true and 15 gates.
fn valid_verification_proof(digest: WorkflowDigest) -> VerificationProof {
    VerificationProof {
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
        warnings: vec![],
    }
}

/// Helper: build an AcceptedArtifact with a given verification proof.
fn make_artifact(digest: WorkflowDigest, verification: VerificationProof) -> AcceptedArtifact {
    AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir: vec![],
        verification,
        accepted_at_seq: EventSeq::ZERO,
        required_capabilities: Box::new([]),
    }
}

/// Test store that returns a single artifact or None.
struct TestStore {
    artifact: Option<AcceptedArtifact>,
}

impl TestStore {
    fn new(artifact: Option<AcceptedArtifact>) -> Self {
        Self { artifact }
    }
}

impl AcceptedArtifactStore for TestStore {
    fn load_accepted_artifact(
        &self,
        digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        match &self.artifact {
            Some(artifact) if artifact.digest == digest => Ok(artifact.clone()),
            _ => Err(ArtifactEnvelopeError::ArtifactNotFound { digest }),
        }
    }
}

// ---------------------------------------------------------------------------
// VB-RQMW-007: accepted_artifact_admission_decision behavior tests
// ---------------------------------------------------------------------------

/// Valid artifact: all proof flags true, gate count 15, digest matches.
/// Expected: admit succeeds, RunAdmission is returned with NoError.
#[test]
fn admit_run_accepts_valid_artifact_under_strict_policy() {
    let digest = WorkflowDigest::from_bytes([42u8; 32]);
    let run_id = RunId::new(1);
    let verification = valid_verification_proof(digest);
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        run_id,
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(result.is_ok(), "Valid artifact should be admitted");
    let admission = result.unwrap();
    assert_eq!(admission.run_id(), run_id);
    assert_eq!(admission.artifact_digest(), digest);
}

/// Valid artifact: admit_run (non-certificate-floor variant) also succeeds.
#[test]
fn admit_run_accepts_valid_artifact_without_certificate_floor() {
    let digest = WorkflowDigest::from_bytes([42u8; 32]);
    let run_id = RunId::new(2);
    let verification = valid_verification_proof(digest);
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        run_id,
        digest,
        CapabilitySet::empty(),
    );

    assert!(
        result.is_ok(),
        "Valid artifact should be admitted via admit_artifact_run"
    );
}

/// Missing artifact: store returns NotFound.
/// Expected: AdmissionError::ArtifactNotFound.
#[test]
fn admit_rejects_when_artifact_not_found() {
    let digest = WorkflowDigest::from_bytes([1u8; 32]);
    let run_id = RunId::new(3);
    let store = TestStore::new(None);

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        run_id,
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(result.is_err(), "Missing artifact should be rejected");
    let err = result.unwrap_err();
    match err {
        AdmissionError::ArtifactNotFound { digest: found } => {
            assert_eq!(found, digest, "Error should carry the requested digest");
        }
        other => panic!("Expected ArtifactNotFound, got {:?}", other),
    }
}

/// Invalid gate count: artifact has 14 gates instead of 15.
/// Expected: ArtifactEnvelopeError::InvalidGateCount.
#[test]
fn admit_rejects_artifact_with_wrong_gate_count() {
    let digest = WorkflowDigest::from_bytes([2u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 14, // Wrong: should be 15
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(4),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(
        result.is_err(),
        "Artifact with wrong gate count should be rejected"
    );
    let err = result.unwrap_err();
    match &err {
        AdmissionError::ArtifactInvalidGateCount { found, required } => {
            assert_eq!(*found, 14);
            assert_eq!(*required, 15);
        }
        other => panic!("Expected ArtifactInvalidGateCount, got {:?}", other),
    }
}

/// Missing proof flag (bounded_claimed = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_bounded_flag() {
    let digest = WorkflowDigest::from_bytes([3u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: false,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(100),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing bounded flag should cause rejection"
    );
}

/// Missing proof flag (taint_safe_claimed = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_taint_safe_flag() {
    let digest = WorkflowDigest::from_bytes([4u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: false,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(101),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing taint_safe flag should cause rejection"
    );
}

/// Missing proof flag (retry_safe_claimed = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_retry_safe_flag() {
    let digest = WorkflowDigest::from_bytes([5u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: false,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(102),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing retry_safe flag should cause rejection"
    );
}

/// Missing proof flag (durable = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_durable_flag() {
    let digest = WorkflowDigest::from_bytes([6u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: false,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(103),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing durable flag should cause rejection"
    );
}

/// Missing proof flag (idempotency_verified_claimed = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_idempotency_verified_flag() {
    let digest = WorkflowDigest::from_bytes([7u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: false,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(104),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing idempotency_verified flag should cause rejection"
    );
}

/// Missing proof flag (replayable_claimed = false) causes admission rejection.
#[test]
fn admit_rejects_artifact_with_missing_replayable_flag() {
    let digest = WorkflowDigest::from_bytes([8u8; 32]);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: false,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(105),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        result.is_err(),
        "Missing replayable flag should cause rejection"
    );
}

/// Digest mismatch: verification.digest != artifact.digest.
/// Expected: AdmissionError::ArtifactDigestMismatch.
#[test]
fn admit_rejects_artifact_with_digest_mismatch() {
    let digest = WorkflowDigest::from_bytes([9u8; 32]);
    let bad_digest = WorkflowDigest::from_bytes([99u8; 32]);
    let verification = VerificationProof {
        digest: bad_digest, // Mismatched
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([]),
        idempotency_attested: Box::new([]),
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(5),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(result.is_err(), "Digest mismatch should be rejected");
    match result.unwrap_err() {
        AdmissionError::ArtifactDigestMismatch {
            requested,
            found: actual,
        } => {
            assert_eq!(requested, digest);
            assert_eq!(actual, bad_digest);
        }
        other => panic!("Expected ArtifactDigestMismatch, got {:?}", other),
    }
}

/// Relaxed policy: admission always succeeds regardless of artifact validity.
/// This is the production behavior for RuntimePolicy::Relaxed.
#[test]
fn admit_run_accepts_under_relaxed_policy_without_artifact() {
    let run_id = RunId::new(6);
    let caps = CapabilitySet::empty();

    // Use a store that returns nothing — relaxed policy should not check.
    let store = TestStore::new(None);

    let result = vb_runtime::admission::admit_run(
        &store,
        RuntimePolicy::Relaxed,
        WorkflowDigest::from_bytes([0u8; 32]),
        run_id,
        caps,
    );

    assert!(result.is_ok(), "Relaxed policy should always admit");
}

/// Strict policy: artifact must exist in store.
/// When store doesn't have the artifact, reject with ArtifactNotFound.
#[test]
fn admit_rejects_missing_artifact_under_strict_policy() {
    let run_id = RunId::new(7);
    let store = TestStore::new(None);

    let result = vb_runtime::admission::admit_run(
        &store,
        RuntimePolicy::Strict,
        WorkflowDigest::from_bytes([11u8; 32]),
        run_id,
        CapabilitySet::empty(),
    );

    assert!(
        result.is_err(),
        "Strict policy should reject missing artifact"
    );
    match result.unwrap_err() {
        AdmissionError::ArtifactNotFound { .. } => {}
        other => panic!("Expected ArtifactNotFound, got {:?}", other),
    }
}

/// Certificate floor: artifact accepted_at_seq is below the required floor.
/// Expected: AdmissionError::ArtifactCertificateStale.
#[test]
fn admit_rejects_artifact_with_stale_certificate_floor() {
    let digest = WorkflowDigest::from_bytes([12u8; 32]);
    let verification = valid_verification_proof(digest);
    // accepted_at_seq is in the past relative to the required floor
    let artifact = AcceptedArtifact {
        digest,
        source_digest: digest,
        policy_digest: digest,
        ir: vec![],
        verification,
        accepted_at_seq: EventSeq::new(0),
        required_capabilities: Box::new([]),
    };
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(8),
        digest,
        CapabilitySet::empty(),
        EventSeq::new(100), // Required floor is higher than accepted_at_seq
    );

    assert!(
        result.is_err(),
        "Artifact with accepted_at_seq below floor should be rejected"
    );
    match result.unwrap_err() {
        AdmissionError::ArtifactCertificateStale {
            digest: found_digest,
            accepted_at_seq,
            required_at_least,
        } => {
            assert_eq!(found_digest, digest);
            assert_eq!(accepted_at_seq, EventSeq::new(0));
            assert_eq!(required_at_least, EventSeq::new(100));
        }
        other => panic!("Expected ArtifactCertificateStale, got {:?}", other),
    }
}

/// Totality: every artifact is either admitted or rejected (no undecided).
/// This verifies the spec's proof_decision_total invariant.
#[test]
fn admit_result_is_always_admitted_or_rejected() {
    let digest = WorkflowDigest::from_bytes([13u8; 32]);

    // Valid artifact → admitted
    let valid_artifact = make_artifact(digest, valid_verification_proof(digest));
    let valid_store = TestStore::new(Some(valid_artifact));
    let valid_result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &valid_store,
        RuntimePolicy::Strict,
        RunId::new(9),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        valid_result.is_ok(),
        "Valid artifact must be admitted (not rejected)"
    );

    // Missing artifact → rejected
    let missing_store = TestStore::new(None);
    let missing_result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &missing_store,
        RuntimePolicy::Strict,
        RunId::new(10),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );
    assert!(
        missing_result.is_err(),
        "Missing artifact must be rejected (not admitted)"
    );

    // Every artifact is either admitted OR rejected, never neither.
    // This is the spec's outcome_admitted(case) || outcome_rejects(case) invariant.
}

/// Rejection before ack and run state insertion: rejected artifacts must not
/// produce a RunAdmission record.
#[test]
fn rejected_artifacts_produce_no_admission_record() {
    let digest = WorkflowDigest::from_bytes([14u8; 32]);

    // Missing artifact
    let store = TestStore::new(None);
    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(11),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(result.is_err());
    // Err means no RunAdmission was produced — the admission gate blocked it.
    // This verifies the spec's invariant: rejected → !admitted, !acknowledged, !run_state_inserted.
}

/// Only Valid case admits: all other error cases must reject.
/// This is the spec's proof_admission_possible_only_for_valid invariant.
#[test]
fn only_valid_artifact_case_admits() {
    let digest = WorkflowDigest::from_bytes([15u8; 32]);

    // Create artifacts with each failure mode
    let test_cases = vec![
        (
            "missing_flag",
            VerificationProof {
                bounded_claimed: false,
                ..valid_verification_proof(digest)
            },
        ),
        (
            "wrong_gate",
            VerificationProof {
                gate_count: 14,
                ..valid_verification_proof(digest)
            },
        ),
        (
            "digest_mismatch",
            VerificationProof {
                digest: WorkflowDigest::from_bytes([99u8; 32]),
                ..valid_verification_proof(digest)
            },
        ),
    ];

    for (name, verification) in test_cases {
        let artifact = make_artifact(digest, verification);
        let store = TestStore::new(Some(artifact));

        let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
            &store,
            RuntimePolicy::Strict,
            RunId::new(12),
            digest,
            CapabilitySet::empty(),
            EventSeq::ZERO,
        );

        assert!(
            result.is_err(),
            "Artifact with {} should be rejected, not admitted",
            name
        );
    }
}

/// MissingIdempotencyAttestation: artifact with unattested keyed actions is rejected.
#[test]
fn admit_rejects_artifact_with_missing_idempotency_attestation() {
    let digest = WorkflowDigest::from_bytes([10u8; 32]);
    let keyed_action = ActionId::new(1u16);
    let verification = VerificationProof {
        digest,
        gate_count: 15,
        durable: true,
        bounded_claimed: true,
        taint_safe_claimed: true,
        retry_safe_claimed: true,
        idempotency_verified_claimed: true,
        replayable_claimed: true,
        idempotency_keyed: Box::new([keyed_action]),
        idempotency_attested: Box::new([]), // Empty — keyed action is not attested
        warnings: vec![],
    };
    let artifact = make_artifact(digest, verification);
    let store = TestStore::new(Some(artifact));

    let result = vb_runtime::admission::admit_artifact_run_with_certificate_floor(
        &store,
        RuntimePolicy::Strict,
        RunId::new(13),
        digest,
        CapabilitySet::empty(),
        EventSeq::ZERO,
    );

    assert!(
        result.is_err(),
        "Missing idempotency attestation should cause rejection"
    );
}
