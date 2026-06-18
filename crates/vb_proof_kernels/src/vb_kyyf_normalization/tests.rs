//! Unit tests for vb-kyyf normalization types and comparison functions.

#![allow(dead_code)]

use super::*;

const CLEAN_DIGESTS: DigestStatus = DigestStatus {
    workflow_source_matches: true,
    compiled_ir_matches: true,
    action_abi_matches: true,
    policy_matches: true,
};

const fn observation() -> PublicObservation {
    PublicObservation {
        result: TerminalResult::Ok,
        taint: TaintStatus::Clean,
        event_signature: 1,
        event_payload_signature: 2,
        digest_status: CLEAN_DIGESTS,
        replay_policy_blocked: false,
        unsupported_generated_subset: false,
        semantic_slot_signature: 3,
        semantic_action_signature: 4,
        semantic_suspension: false,
        semantic_taint_signature: 5,
        temp_path_signature: 10,
        process_id_signature: 11,
        wall_clock_signature: 12,
        generated_run_signature: 13,
    }
}

// ── cold_metadata_is_normalized_away ──────────────────────────────────────

#[test]
fn cold_metadata_is_normalized_away() {
    let left = observation();
    let right = PublicObservation {
        temp_path_signature: 20,
        process_id_signature: 21,
        wall_clock_signature: 22,
        generated_run_signature: 23,
        ..observation()
    };

    assert!(matches!(compare_cross_run(left, right), Ok(())));
}

#[test]
fn semantic_delta_is_rejected() {
    let left = observation();
    let right = PublicObservation {
        semantic_slot_signature: 99,
        ..observation()
    };

    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn replay_digest_mismatch_keeps_exact_taxonomy() {
    let left = PublicObservation {
        digest_status: DigestStatus {
            workflow_source_matches: false,
            ..CLEAN_DIGESTS
        },
        ..observation()
    };

    assert!(matches!(
        compare_replay(left, observation()),
        Err(DeterminismError::ReplayDigestMismatch)
    ));
}

// ── DigestStatus::all_match — field coverage ───────────────────────────

#[test]
fn digest_status_all_match_when_all_true() {
    assert!(CLEAN_DIGESTS.all_match());
}

#[test]
fn digest_status_all_match_false_on_workflow_source() {
    let s = DigestStatus {
        workflow_source_matches: false,
        ..CLEAN_DIGESTS
    };
    assert!(!s.all_match());
}

#[test]
fn digest_status_all_match_false_on_compiled_ir() {
    let s = DigestStatus {
        compiled_ir_matches: false,
        ..CLEAN_DIGESTS
    };
    assert!(!s.all_match());
}

#[test]
fn digest_status_all_match_false_on_action_abi() {
    let s = DigestStatus {
        action_abi_matches: false,
        ..CLEAN_DIGESTS
    };
    assert!(!s.all_match());
}

#[test]
fn digest_status_all_match_false_on_policy() {
    let s = DigestStatus {
        policy_matches: false,
        ..CLEAN_DIGESTS
    };
    assert!(!s.all_match());
}

#[test]
fn digest_status_all_match_false_when_all_false() {
    let s = DigestStatus {
        workflow_source_matches: false,
        compiled_ir_matches: false,
        action_abi_matches: false,
        policy_matches: false,
    };
    assert!(!s.all_match());
}

// ── normalize_observation — field projection ───────────────────────────

#[test]
fn normalize_observation_drops_temp_path_signature() {
    let raw = PublicObservation {
        temp_path_signature: 1234,
        ..observation()
    };
    let norm = normalize_observation(raw);
    let other = normalize_observation(observation());
    assert!(norm.event_signature == other.event_signature);
    assert!(norm.event_payload_signature == other.event_payload_signature);
}

#[test]
fn normalize_observation_drops_process_id_signature() {
    let raw = PublicObservation {
        process_id_signature: 5678,
        ..observation()
    };
    let norm = normalize_observation(raw);
    let other = normalize_observation(observation());
    assert!(norm.event_signature == other.event_signature);
}

#[test]
fn normalize_observation_drops_wall_clock_signature() {
    let raw = PublicObservation {
        wall_clock_signature: 9012,
        ..observation()
    };
    let norm = normalize_observation(raw);
    let other = normalize_observation(observation());
    assert!(norm.event_signature == other.event_signature);
}

#[test]
fn normalize_observation_drops_generated_run_signature() {
    let raw = PublicObservation {
        generated_run_signature: 3456,
        ..observation()
    };
    let norm = normalize_observation(raw);
    let other = normalize_observation(observation());
    assert!(norm.event_signature == other.event_signature);
}

#[test]
fn normalize_observation_preserves_all_nondropped_fields() {
    let raw = PublicObservation {
        result: TerminalResult::Failed,
        taint: TaintStatus::Tainted,
        event_signature: 99,
        event_payload_signature: 100,
        digest_status: CLEAN_DIGESTS,
        replay_policy_blocked: true,
        unsupported_generated_subset: true,
        semantic_slot_signature: 11,
        semantic_action_signature: 22,
        semantic_suspension: true,
        semantic_taint_signature: 33,
        temp_path_signature: 1,
        process_id_signature: 2,
        wall_clock_signature: 3,
        generated_run_signature: 4,
    };
    let norm = normalize_observation(raw);
    assert!(matches!(norm.result, TerminalResult::Failed));
    assert!(matches!(norm.taint, TaintStatus::Tainted));
    assert!(norm.event_signature == 99u64);
    assert!(norm.event_payload_signature == 100u64);
    assert!(norm.replay_policy_blocked);
    assert!(norm.unsupported_generated_subset);
    assert!(norm.semantic_slot_signature == 11u64);
    assert!(norm.semantic_action_signature == 22u64);
    assert!(norm.semantic_suspension);
    assert!(norm.semantic_taint_signature == 33u64);
}

// ── compare_cross_run — exhaustiveness ─────────────────────────────────

#[test]
fn compare_cross_run_equal_observations_ok() {
    let a = observation();
    let b = observation();
    assert!(matches!(compare_cross_run(a, b), Ok(())));
}

#[test]
fn compare_cross_run_different_result_rejected() {
    let left = observation();
    let right = PublicObservation {
        result: TerminalResult::Failed,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_taint_rejected() {
    let left = observation();
    let right = PublicObservation {
        taint: TaintStatus::Tainted,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_event_signature_rejected() {
    let left = observation();
    let right = PublicObservation {
        event_signature: 7777,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_payload_signature_rejected() {
    let left = observation();
    let right = PublicObservation {
        event_payload_signature: 7777,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_semantic_action_rejected() {
    let left = observation();
    let right = PublicObservation {
        semantic_action_signature: 7777,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_semantic_suspension_rejected() {
    let left = observation();
    let right = PublicObservation {
        semantic_suspension: true,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_cross_run_different_semantic_taint_rejected() {
    let left = observation();
    let right = PublicObservation {
        semantic_taint_signature: 7777,
        ..observation()
    };
    assert!(matches!(
        compare_cross_run(left, right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

// ── compare_replay — replay-specific paths ──────────────────────────────

#[test]
fn compare_replay_digest_mismatch_on_first_only() {
    let left = PublicObservation {
        digest_status: DigestStatus {
            workflow_source_matches: false,
            ..CLEAN_DIGESTS
        },
        ..observation()
    };
    assert!(matches!(
        compare_replay(left, observation()),
        Err(DeterminismError::ReplayDigestMismatch)
    ));
}

#[test]
fn compare_replay_digest_mismatch_on_second_only() {
    let right = PublicObservation {
        digest_status: DigestStatus {
            action_abi_matches: false,
            ..CLEAN_DIGESTS
        },
        ..observation()
    };
    assert!(matches!(
        compare_replay(observation(), right),
        Err(DeterminismError::ReplayDigestMismatch)
    ));
}

#[test]
fn compare_replay_policy_blocked_on_first() {
    let left = PublicObservation {
        replay_policy_blocked: true,
        ..observation()
    };
    assert!(matches!(
        compare_replay(left, observation()),
        Err(DeterminismError::ReplayPolicyBlocked)
    ));
}

#[test]
fn compare_replay_policy_blocked_on_second() {
    let right = PublicObservation {
        replay_policy_blocked: true,
        ..observation()
    };
    assert!(matches!(
        compare_replay(observation(), right),
        Err(DeterminismError::ReplayPolicyBlocked)
    ));
}

#[test]
fn compare_replay_sequence_violation_on_event_signature() {
    let right = PublicObservation {
        event_signature: 9999,
        ..observation()
    };
    assert!(matches!(
        compare_replay(observation(), right),
        Err(DeterminismError::ReplaySequenceViolation)
    ));
}

#[test]
fn compare_replay_observation_specific_rejection_after_sequence_check() {
    let right = PublicObservation {
        semantic_slot_signature: 5555,
        ..observation()
    };
    assert!(matches!(
        compare_replay(observation(), right),
        Err(DeterminismError::NondeterministicObservation)
    ));
}

#[test]
fn compare_replay_happy_path() {
    assert!(compare_replay(observation(), observation()).is_ok());
}

// ── compare_generated_ir — exhaustiveness ──────────────────────────────

#[test]
fn compare_generated_ir_equal_ok() {
    assert!(compare_generated_ir(observation(), observation()).is_ok());
}

#[test]
fn compare_generated_ir_unsupported_on_ir() {
    let ir = PublicObservation {
        unsupported_generated_subset: true,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(ir, observation()),
        Err(DeterminismError::UnsupportedGeneratedSubset)
    ));
}

#[test]
fn compare_generated_ir_unsupported_on_generated() {
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
fn compare_generated_ir_divergence_on_result() {
    let generated = PublicObservation {
        result: TerminalResult::Failed,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_taint() {
    let generated = PublicObservation {
        taint: TaintStatus::Tainted,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_event_signature() {
    let generated = PublicObservation {
        event_signature: 99,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_payload_signature() {
    let generated = PublicObservation {
        event_payload_signature: 99,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_digest_status() {
    let generated = PublicObservation {
        digest_status: DigestStatus {
            workflow_source_matches: false,
            ..CLEAN_DIGESTS
        },
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_replay_policy() {
    let generated = PublicObservation {
        replay_policy_blocked: true,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_semantic_slot() {
    let generated = PublicObservation {
        semantic_slot_signature: 9999,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_semantic_action() {
    let generated = PublicObservation {
        semantic_action_signature: 9999,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

#[test]
fn compare_generated_ir_divergence_on_semantic_suspension() {
    let generated = PublicObservation {
        semantic_suspension: true,
        ..observation()
    };
    assert!(matches!(
        compare_generated_ir(observation(), generated),
        Err(DeterminismError::GeneratedIrDivergence)
    ));
}

// ── Copy semantics for Copy types ──────────────────────────────────────

#[test]
fn terminal_result_is_copy() {
    let a = TerminalResult::Ok;
    let b = a;
    assert!(matches!(a, TerminalResult::Ok));
    assert!(matches!(b, TerminalResult::Ok));
}

#[test]
fn taint_status_is_copy() {
    let a = TaintStatus::Clean;
    let b = a;
    assert!(matches!(a, TaintStatus::Clean));
    assert!(matches!(b, TaintStatus::Clean));
}

#[test]
fn determinism_error_is_copy() {
    let a = DeterminismError::NondeterministicObservation;
    let b = a;
    assert!(matches!(a, DeterminismError::NondeterministicObservation));
    assert!(matches!(b, DeterminismError::NondeterministicObservation));
}

#[test]
fn digest_status_is_copy() {
    let a = CLEAN_DIGESTS;
    let b = a;
    assert!(a.workflow_source_matches);
    assert!(b.workflow_source_matches);
}

#[test]
fn public_observation_is_copy() {
    let a = observation();
    let b = a;
    assert!(matches!(a.result, TerminalResult::Ok));
    assert!(matches!(b.result, TerminalResult::Ok));
}
