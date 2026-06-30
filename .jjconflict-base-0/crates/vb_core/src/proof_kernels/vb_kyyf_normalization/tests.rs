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
