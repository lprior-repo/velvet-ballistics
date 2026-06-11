use vb_proof_kernels::envelope_header::{
    EnvelopeHeader, HEADER_LEN, ValidationError, ValidationResult, compute_header_crc,
    validate_header_before_alloc, validate_header_crc,
};
use vb_proof_kernels::vb_kyyf_normalization::{
    DeterminismError, DigestStatus, PublicObservation, TaintStatus, TerminalResult,
    compare_cross_run, compare_generated_ir, compare_replay, normalize_observation,
};

fn clean_digest() -> DigestStatus {
    DigestStatus {
        workflow_source_matches: true,
        compiled_ir_matches: true,
        action_abi_matches: true,
        policy_matches: true,
    }
}

fn observation() -> PublicObservation {
    PublicObservation {
        result: TerminalResult::Ok,
        taint: TaintStatus::Clean,
        event_signature: 10,
        event_payload_signature: 20,
        digest_status: clean_digest(),
        replay_policy_blocked: false,
        unsupported_generated_subset: false,
        semantic_slot_signature: 30,
        semantic_action_signature: 40,
        semantic_suspension: false,
        semantic_taint_signature: 50,
        temp_path_signature: 60,
        process_id_signature: 70,
        wall_clock_signature: 80,
        generated_run_signature: 90,
    }
}

#[test]
fn header_len_constant_matches_wire_contract() {
    assert_eq!(HEADER_LEN, 60);
}

#[test]
fn new_header_sets_magic_value() {
    assert_eq!(EnvelopeHeader::new().magic, EnvelopeHeader::MAGIC_VALUE);
}

#[test]
fn default_header_matches_new_header() {
    assert_eq!(EnvelopeHeader::default(), EnvelopeHeader::new());
}

#[test]
fn header_validate_magic_accepts_default() {
    assert!(EnvelopeHeader::new().validate_magic());
}

#[test]
fn header_validate_magic_rejects_bad_magic() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0;
    assert!(!header.validate_magic());
}

#[test]
fn header_validate_header_len_returns_true_for_typed_header() {
    assert!(EnvelopeHeader::new().validate_header_len());
}

#[test]
fn header_payload_len_combines_high_and_low_words() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_hi = 1;
    header.payload_len_u32 = 2;
    assert_eq!(header.payload_len(), 0x1_0000_0002);
}

#[test]
fn header_validate_payload_len_accepts_exact_maximum() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 32;
    assert!(header.validate_payload_len(32));
}

#[test]
fn header_validate_payload_len_rejects_oversize() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 33;
    assert!(!header.validate_payload_len(32));
}

#[test]
fn header_validate_before_alloc_accepts_valid_header() {
    assert_eq!(
        EnvelopeHeader::new().validate_before_alloc(0),
        ValidationResult::Ok
    );
}

#[test]
fn header_validate_before_alloc_rejects_invalid_magic() {
    let mut header = EnvelopeHeader::new();
    header.magic = 0xDEAD_BEEF;
    assert_eq!(
        header.validate_before_alloc(1024),
        ValidationResult::Err(ValidationError::InvalidMagic)
    );
}

#[test]
fn header_validate_before_alloc_rejects_payload_too_large() {
    let mut header = EnvelopeHeader::new();
    header.payload_len_u32 = 1025;
    assert_eq!(
        header.validate_before_alloc(1024),
        ValidationResult::Err(ValidationError::PayloadTooLarge)
    );
}

#[test]
fn validate_header_before_alloc_wrapper_matches_method() {
    let header = EnvelopeHeader::new();
    assert_eq!(
        validate_header_before_alloc(&header, 1024),
        header.validate_before_alloc(1024)
    );
}

#[test]
fn compute_header_crc_keeps_stub_zero_contract() {
    assert_eq!(compute_header_crc(&EnvelopeHeader::new()), 0);
}

#[test]
fn validate_header_crc_keeps_stub_true_contract() {
    assert!(validate_header_crc(&EnvelopeHeader::new()));
}

#[test]
fn digest_status_all_match_accepts_clean_digests() {
    assert!(clean_digest().all_match());
}

#[test]
fn digest_status_all_match_rejects_workflow_mismatch() {
    assert!(
        !DigestStatus {
            workflow_source_matches: false,
            ..clean_digest()
        }
        .all_match()
    );
}

#[test]
fn digest_status_all_match_rejects_compiled_ir_mismatch() {
    assert!(
        !DigestStatus {
            compiled_ir_matches: false,
            ..clean_digest()
        }
        .all_match()
    );
}

#[test]
fn normalize_observation_preserves_result() {
    let normalized = normalize_observation(observation());
    assert!(matches!(normalized.result, TerminalResult::Ok));
}

#[test]
fn normalize_observation_preserves_taint() {
    let normalized = normalize_observation(PublicObservation {
        taint: TaintStatus::Tainted,
        ..observation()
    });
    assert!(matches!(normalized.taint, TaintStatus::Tainted));
}

#[test]
fn normalize_observation_preserves_event_signature() {
    assert_eq!(normalize_observation(observation()).event_signature, 10);
}

#[test]
fn normalize_observation_preserves_payload_signature() {
    assert_eq!(
        normalize_observation(observation()).event_payload_signature,
        20
    );
}

#[test]
fn normalize_observation_preserves_semantic_slot_signature() {
    assert_eq!(
        normalize_observation(observation()).semantic_slot_signature,
        30
    );
}

#[test]
fn normalize_observation_preserves_semantic_action_signature() {
    assert_eq!(
        normalize_observation(observation()).semantic_action_signature,
        40
    );
}

#[test]
fn normalize_observation_preserves_semantic_suspension() {
    let normalized = normalize_observation(PublicObservation {
        semantic_suspension: true,
        ..observation()
    });
    assert!(normalized.semantic_suspension);
}

#[test]
fn normalize_observation_preserves_semantic_taint_signature() {
    assert_eq!(
        normalize_observation(observation()).semantic_taint_signature,
        50
    );
}

#[test]
fn compare_cross_run_ignores_cold_metadata() {
    let right = PublicObservation {
        temp_path_signature: 999,
        process_id_signature: 998,
        wall_clock_signature: 997,
        generated_run_signature: 996,
        ..observation()
    };
    assert!(matches!(compare_cross_run(observation(), right), Ok(())));
}

#[test]
fn compare_cross_run_rejects_terminal_result_delta() {
    let right = PublicObservation {
        result: TerminalResult::Failed,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(observation(), right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_rejects_taint_delta() {
    let right = PublicObservation {
        taint: TaintStatus::Unknown,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(observation(), right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_rejects_event_signature_delta() {
    let right = PublicObservation {
        event_signature: 999,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(observation(), right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_replay_rejects_digest_mismatch() {
    let first = PublicObservation {
        digest_status: DigestStatus {
            policy_matches: false,
            ..clean_digest()
        },
        ..observation()
    };
    assert!(matches!(
        compare_replay(first, observation()),
        Err(DeterminismError::ReplayDigestMismatch)
    ));
}

#[test]
fn compare_replay_rejects_policy_block() {
    let first = PublicObservation {
        replay_policy_blocked: true,
        ..observation()
    };
    assert!(matches!(
        compare_replay(first, observation()),
        Err(DeterminismError::ReplayPolicyBlocked)
    ));
}

#[test]
fn compare_replay_rejects_sequence_delta_before_normalized_compare() {
    let second = PublicObservation {
        event_signature: 999,
        ..observation()
    };
    assert!(matches!(
        compare_replay(observation(), second),
        Err(DeterminismError::ReplaySequenceViolation)
    ));
}

#[test]
fn compare_replay_accepts_matching_observations() {
    assert!(matches!(
        compare_replay(observation(), observation()),
        Ok(())
    ));
}

#[test]
fn compare_generated_ir_rejects_unsupported_generated_subset() {
    let generated = PublicObservation {
        unsupported_generated_subset: true,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::UnsupportedGeneratedSubset)
    ));
}

#[test]
fn compare_generated_ir_rejects_semantic_divergence() {
    let generated = PublicObservation {
        semantic_action_signature: 999,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_accepts_matching_observations() {
    assert!(matches!(
        compare_generated_ir(observation(), observation()),
        Ok(())
    ));
}
