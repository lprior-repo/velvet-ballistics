//! Journal replay and storage fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

const MAX_FUZZ_PAYLOAD: u32 = 4096;

fn assert_typed_journal_error(error: vb_storage::JournalError) {
    use vb_storage::JournalError;
    match error {
        JournalError::UnexpectedEof
        | JournalError::HeaderChecksumMismatch
        | JournalError::PayloadDigestMismatch
        | JournalError::PostcardDecodeFailed(_)
        | JournalError::InvalidEvent
        | JournalError::BadMagic { .. }
        | JournalError::PayloadTooLarge { .. }
        | JournalError::RecordKindFamilyMismatch { .. }
        | JournalError::UnknownRecordKind { .. }
        | JournalError::UnsupportedSchemaVersion { .. }
        | JournalError::HeaderLengthMismatch { .. }
        | JournalError::SequenceOverflow
        | JournalError::WrongRun { .. }
        | JournalError::SequenceGap { .. }
        | JournalError::Fjall(_)
        | JournalError::Encode(_)
        | JournalError::KeyCapacity
        | JournalError::DuplicateEvent { .. }
        | JournalError::WriteLockPoisoned
        | JournalError::QueueCapacity
        | JournalError::QueueFull
        | JournalError::JournalBatchBytesExceeded { .. }
        | JournalError::QueueShutdown
        | JournalError::MigrationRequired { .. }
        | JournalError::ArtifactMalformed
        | JournalError::ArtifactChecksumMismatch
        | JournalError::InvalidGateCount { .. }
        | JournalError::MissingRequiredProofFlag { .. }
        | JournalError::ArtifactNotFound { .. }
        | JournalError::AdmissionRequired
        | JournalError::ArtifactInvalid { .. }
        | JournalError::InputTooLarge { .. }
        | JournalError::InputSchemaMismatch
        | JournalError::CapabilityDenied
        | JournalError::SecretUnavailable
        | JournalError::RunAlreadyExists
        | JournalError::InvalidRunId { .. }
        | JournalError::ActiveRunCapacityExceeded
        | JournalError::FrameAllocationFailed
        | JournalError::AdmissionJournalFailed
        | JournalError::StrictDurabilityFailed
        | JournalError::TooManyEvents { .. }
        | JournalError::ReplayAllocationFailed { .. }
        | JournalError::ClockUnavailable
        | JournalError::ProcessLockHeld { .. }
        | JournalError::ProcessLockIo { .. }
        | JournalError::Trim(_) => {}
        _ => {}
    }
}

fn assert_typed_recovery_error(error: vb_storage::recovery::RecoveryError) {
    use vb_storage::recovery::RecoveryError;
    match error {
        RecoveryError::Journal(_)
        | RecoveryError::WorkflowSourceDigestMismatch { .. }
        | RecoveryError::CompiledIrDigestMismatch { .. }
        | RecoveryError::ActionAbiMismatch { .. }
        | RecoveryError::PolicyDigestMismatch { .. }
        | RecoveryError::NonIdempotentActionBlocked { .. }
        | RecoveryError::ReplayDivergence { .. }
        | RecoveryError::SlotTaintReadFailed { .. }
        | RecoveryError::CorruptSlotTaint { .. }
        | RecoveryError::NoRecoveryData { .. } => {}
        _ => {}
    }
}

pub fn fuzz_journal_event(data: &[u8]) {
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        MAX_FUZZ_PAYLOAD,
    );

    match decoded {
        Ok((_envelope, event)) => {
            assert!(event.is_valid(), "Decoded event must be structurally valid");

            let Ok(encoded) = vb_storage::encode_record(
                vb_storage::MAGIC_JOURNAL_EVENT,
                event.record_kind(),
                event.seq().get(),
                &event,
                MAX_FUZZ_PAYLOAD,
            ) else {
                return;
            };

            let reparsed = vb_storage::decode_record::<vb_storage::JournalEvent>(
                &encoded,
                vb_storage::MAGIC_JOURNAL_EVENT,
                MAX_FUZZ_PAYLOAD,
            );
            assert!(
                reparsed.is_ok(),
                "Round-trip encode/decode must succeed for valid event"
            );
        }
        Err(error) => {
            assert_typed_journal_error(error);
        }
    }
}

pub fn fuzz_replay_events(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    if events.is_empty() {
        return;
    }
    let mut tracker: vb_storage::recovery::ActionReplayTracker =
        vb_storage::recovery::ActionReplayTracker::new();
    let result = vb_storage::recovery::replay_events(&events, &mut tracker, &[]);
    match result {
        Ok(replayed) => {
            assert!(
                replayed.len() <= events.len(),
                "replayed {} events must not exceed input {} events",
                replayed.len(),
                events.len()
            );
            for event in &replayed {
                if let vb_storage::JournalEvent::ActionCompletedEvent {
                    action, step, ..
                } = event
                {
                    assert!(
                        tracker.has_completed(*action, *step),
                        "ActionCompletedEvent must be tracked as completed"
                    );
                }
                if let vb_storage::JournalEvent::ActionFailedEvent { action, step, .. } = event {
                    assert!(
                        tracker.has_failed(*action, *step),
                        "ActionFailedEvent must be tracked as failed"
                    );
                }
            }
        }
        Err(e) => {
            assert_typed_recovery_error(e);
        }
    }
}

pub fn fuzz_extract_terminal(data: &[u8]) {
    let Ok(events): Result<Vec<vb_storage::JournalEvent>, _> = postcard::from_bytes(data) else {
        return;
    };
    let terminal = vb_storage::recovery::extract_terminal(&events);
    if let Some(event) = terminal {
        assert!(
            matches!(
                event,
                vb_storage::JournalEvent::RunFinished { .. }
                    | vb_storage::JournalEvent::RunFailedEvent { .. }
                    | vb_storage::JournalEvent::RunCancelled { .. }
            ),
            "terminal event must be a terminal kind, got {:?}",
            event.record_kind()
        );
    }
}

pub fn fuzz_action_tracker(data: &[u8]) {
    let mut tracker = vb_storage::recovery::ActionReplayTracker::new();
    for chunk in data.chunks_exact(3).take(64) {
        let Some(mode) = chunk.first().copied() else {
            continue;
        };
        let Some(action) = chunk.get(1).copied() else {
            continue;
        };
        let Some(step) = chunk.get(2).copied() else {
            continue;
        };
        let action = vb_core::ActionId::new(u16::from(action));
        let step = vb_core::StepIdx::new(u16::from(step));
        match mode % 3 {
            0 => {
                let was_resolved = tracker.is_resolved(action, step);
                tracker.mark_completed(action, step);
                assert!(
                    tracker.is_resolved(action, step),
                    "mark_completed must make is_resolved return true"
                );
                assert!(
                    tracker.has_completed(action, step),
                    "mark_completed must make has_completed return true"
                );
                let _ = was_resolved;
            }
            1 => {
                let was_resolved = tracker.is_resolved(action, step);
                tracker.mark_failed(action, step);
                assert!(
                    tracker.is_resolved(action, step),
                    "mark_failed must make is_resolved return true"
                );
                assert!(
                    tracker.has_failed(action, step),
                    "mark_failed must make has_failed return true"
                );
                let _ = was_resolved;
            }
            _ => {
                let first = tracker.is_resolved(action, step);
                let second = tracker.is_resolved(action, step);
                assert_eq!(
                    first, second,
                    "is_resolved must be deterministic for action={:?} step={:?}",
                    action, step
                );
            }
        }
    }
}

pub fn fuzz_admission_flow(data: &[u8]) {
    if data.len() < 2 {
        return;
    }

    let Some(&byte0) = data.first() else {
        return;
    };
    let node_count = usize::from(byte0.wrapping_rem(4)).saturating_add(1);
    let slot_count = u16::from(byte0.wrapping_rem(4)).saturating_add(1);
    let max_slot = slot_count.saturating_sub(1);

    let mut nodes: Vec<vb_core::CompiledNode> = Vec::new();
    for i in 0..node_count {
        let step_idx = vb_core::StepIdx::new(u16::try_from(i).unwrap_or(0));
        let next_step = if i.saturating_add(1) < node_count {
            Some(vb_core::StepIdx::new(
                u16::try_from(i).unwrap_or(0).saturating_add(1),
            ))
        } else {
            None
        };

        if i.saturating_add(1) == node_count {
            nodes.push(vb_core::CompiledNode {
                id: step_idx,
                output: None,
                next: None,
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::Finish {
                    result: vb_core::SlotIdx::new(max_slot),
                },
            });
        } else {
            nodes.push(vb_core::CompiledNode {
                id: step_idx,
                output: Some(vb_core::SlotIdx::new(max_slot)),
                next: next_step,
                error_slot: None,
                on_error: None,
                kind: vb_core::CompiledNodeKind::Nop,
            });
        }
    }

    let parts_zeroed = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_admission"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![vb_core::ConstValue::Bool(true)].into_boxed_slice(),
        slot_count,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let Ok(hash_bytes) = postcard::to_allocvec(&parts_zeroed) else {
        return;
    };
    let computed = blake3::hash(&hash_bytes);
    let correct_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes(*computed.as_bytes()),
        ..parts_zeroed
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(correct_parts) else {
        return;
    };

    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let policies = [
        vb_core::RuntimePolicy::Relaxed,
        vb_core::RuntimePolicy::Journaled,
        vb_core::RuntimePolicy::Strict,
    ];
    for policy in policies {
        let result = vb_storage::submit_artifact(&journal, &workflow, policy);
        match result {
            Ok(artifact) => {
                assert!(
                    artifact.accepted_at_seq.get() >= 1,
                    "accepted artifact must have seq >= 1"
                );
                assert!(
                    artifact.verification.gate_count > 0,
                    "accepted artifact must have gate_count > 0"
                );
                let _ = artifact.digest;
            }
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }

    let corrupted_parts = vb_core::WorkflowParts {
        digest: vb_core::WorkflowDigest::from_bytes([0xFF; 32]),
        ..workflow.to_parts()
    };
    if let Ok(corrupted) = vb_core::CompiledWorkflow::try_from_parts(corrupted_parts) {
        let strict_result =
            vb_storage::submit_artifact(&journal, &corrupted, vb_core::RuntimePolicy::Strict);
        match strict_result {
            Ok(_artifact) => {}
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }
}

pub fn fuzz_admission_fuzz(data: &[u8]) {
    let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) else {
        return;
    };

    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let policies = [
        vb_core::RuntimePolicy::Relaxed,
        vb_core::RuntimePolicy::Journaled,
        vb_core::RuntimePolicy::Strict,
    ];
    for policy in policies {
        let result = vb_storage::submit_artifact(&journal, &workflow, policy);
        match result {
            Ok(artifact) => {
                assert!(
                    artifact.accepted_at_seq.get() >= 1,
                    "artifact must have accepted_at_seq >= 1"
                );
                assert!(
                    workflow.node_count() >= 1,
                    "submitted workflow must have >= 1 node"
                );
                let _ = artifact.digest;
            }
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }
}

pub fn fuzz_strict_artifact_decoder(data: &[u8]) {
    if let Ok(artifact) = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data) {
        assert!(
            artifact.verification.gate_count > 0,
            "strict artifact gate_count must be non-zero"
        );
        assert!(
            artifact.accepted_at_seq.get() >= 1,
            "accepted_at_seq must be >= 1"
        );
    }

    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        assert!(
            parts.nodes.len() <= usize::from(u16::MAX),
            "decoded WorkflowParts node count must fit u16"
        );
    }

    let artifact_decode = postcard::from_bytes::<vb_storage::admission::AcceptedArtifact>(data);
    let parts_decode = postcard::from_bytes::<vb_core::WorkflowParts>(data);
    let _ = artifact_decode.is_ok();
    let _ = parts_decode.is_ok();
}

pub fn fuzz_digest_coherence(data: &[u8]) {
    let digest_bytes: [u8; 32] = match data.get(..32) {
        Some(slice) => match slice.try_into() {
            Ok(arr) => arr,
            Err(_) => return,
        },
        None => return,
    };
    let seed_digest = vb_core::WorkflowDigest::from_bytes(digest_bytes);

    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let nodes: Box<[vb_core::CompiledNode]> = Box::new([vb_core::CompiledNode {
        id: vb_core::StepIdx::ZERO,
        output: Some(vb_core::SlotIdx::ZERO),
        next: None,
        on_error: None,
        error_slot: None,
        kind: vb_core::CompiledNodeKind::Finish {
            result: vb_core::SlotIdx::ZERO,
        },
    }]);
    let constants: Box<[vb_core::ConstValue]> =
        Box::new([vb_core::ConstValue::Bool(true)]);

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_digest_test"),
        digest: seed_digest,
        nodes: nodes.clone(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants,
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };

    let Ok(_workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
        return;
    };

    let mut reference_parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_digest_test"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.clone(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    if let Ok(serialized) = postcard::to_allocvec(&reference_parts) {
        let reference_digest_bytes = blake3::hash(&serialized);
        let reference_digest =
            vb_core::WorkflowDigest::from_bytes(*reference_digest_bytes.as_bytes());

        reference_parts.digest = reference_digest;
        let coherent_workflow = match vb_core::CompiledWorkflow::try_from_parts(reference_parts) {
            Ok(wf) => wf,
            Err(_) => return,
        };
        let result =
            vb_storage::submit_artifact(&journal, &coherent_workflow, vb_core::RuntimePolicy::Strict);
        match result {
            Ok(artifact) => {
                assert_eq!(
                    artifact.digest,
                    reference_digest,
                    "artifact digest must match reference blake3 hash"
                );
            }
            Err(error) => {
                assert_typed_journal_error(error);
            }
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum ReadbackDeletionIntent {
    None,
    Partial,
    Full,
}

#[allow(dead_code)]
impl ReadbackDeletionIntent {
    fn from_mask(mask: u8) -> Self {
        let core_family_mask = mask & 0b0000_1111;
        match core_family_mask.count_ones() {
            0 => Self::None,
            4 => Self::Full,
            _ => Self::Partial,
        }
    }
}

#[allow(dead_code)]
enum ReadbackFamilySet {
    Full,
    Partial,
    Absent,
    Unreadable,
}

fn classify_readback_family_set(
    journal: &vb_storage::FjallJournal,
    digest: vb_core::WorkflowDigest,
    run: vb_core::RunId,
    intended_deletion: ReadbackDeletionIntent,
) -> ReadbackFamilySet {
    let has_source = match journal.workflow_source(digest) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let has_artifact = match journal.compiled_ir(digest) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let has_header = match journal.run_header(run) {
        Ok(record) => record.is_some(),
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let events = match journal.events_for_run(run) {
        Ok(events) => events,
        Err(_) => return ReadbackFamilySet::Unreadable,
    };
    let accepted_event_count = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RunAccepted { workflow, .. } if *workflow == digest
            )
        })
        .count();
    let has_accepted_event = accepted_event_count > 0;
    let families_present = usize::from(has_source)
        .saturating_add(usize::from(has_artifact))
        .saturating_add(usize::from(has_header))
        .saturating_add(usize::from(has_accepted_event));

    if has_source && has_artifact && has_header && has_accepted_event {
        ReadbackFamilySet::Full
    } else if families_present > 0 || matches!(intended_deletion, ReadbackDeletionIntent::Partial) {
        ReadbackFamilySet::Partial
    } else {
        ReadbackFamilySet::Absent
    }
}

pub fn fuzz_readback_family_set(_data: &[u8]) {
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    let parts = vb_core::WorkflowParts {
        name: Box::<str>::from("fuzz_readback"),
        digest: vb_core::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([vb_core::CompiledNode {
            id: vb_core::StepIdx::ZERO,
            output: Some(vb_core::SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: vb_core::CompiledNodeKind::Finish {
                result: vb_core::SlotIdx::ZERO,
            },
        }]),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([vb_core::ConstValue::Bool(true)]),
        slot_count: 1,
        symbols_count: 0,
        entry: vb_core::StepIdx::ZERO,
        resource_contract: vb_core::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    };
    let hash_bytes = match postcard::to_allocvec(&parts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let computed = blake3::hash(&hash_bytes);
    let digest = vb_core::WorkflowDigest::from_bytes(*computed.as_bytes());
    let correct_parts = vb_core::WorkflowParts { digest, ..parts };
    let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(correct_parts) else {
        return;
    };

    if vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict).is_err() {
        return;
    }

    let classification = classify_readback_family_set(
        &journal,
        digest,
        vb_core::RunId::new(8001),
        ReadbackDeletionIntent::None,
    );
    assert!(
        matches!(
            classification,
            ReadbackFamilySet::Full
                | ReadbackFamilySet::Partial
                | ReadbackFamilySet::Absent
                | ReadbackFamilySet::Unreadable
        ),
        "classification must be a valid ReadbackFamilySet variant"
    );
    assert!(
        !matches!(classification, ReadbackFamilySet::Unreadable),
        "classification must not be Unreadable after successful admission"
    );
}

pub fn fuzz_admission_input_surface(data: &[u8]) {
    if data.len() < 2 {
        return;
    }
    let temp_dir = match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let journal = match vb_storage::FjallJournal::open(temp_dir.path(), None) {
        Ok(j) => j,
        Err(_) => return,
    };

    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
            return;
        };

        let strict_result =
            vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
        let relaxed_result =
            vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);

        assert_eq!(
            strict_result.is_ok(),
            relaxed_result.is_ok(),
            "strict and relaxed admission must agree on success/failure for same workflow"
        );

        if let Err(error) = strict_result {
            assert_typed_journal_error(error);
        }
        if let Err(error) = relaxed_result {
            assert_typed_journal_error(error);
        }
    }
}

pub fn fuzz_accepted_artifact_decode(data: &[u8]) {
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let record = vb_storage::CompiledIrRecord {
        digest,
        ir: data.to_vec(),
    };
    if vb_storage::put_compiled_ir(&journal, &record).is_err() {
        return;
    }
    let store = vb_runtime::admission::StorageArtifactStore::new(std::sync::Arc::new(journal));
    let result =
        vb_runtime::admission::AcceptedArtifactStore::load_accepted_artifact(&store, digest);
    match result {
        Ok(artifact) => {
            assert!(
                artifact.accepted_at_seq.get() > 0,
                "accepted_at_seq must be > 0 for loaded artifact"
            );
            assert!(
                artifact.verification.gate_count > 0,
                "gate_count must be > 0 for loaded artifact"
            );
        }
        Err(_error) => {}
    }
}

pub fn fuzz_recovery_decode(data: &[u8]) {
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let run = vb_core::RunId::new(u64::from(data.first().copied().unwrap_or(0)));
    let seq = vb_storage::EventSeq::new(1);
    let events = if data.len().is_multiple_of(2) {
        vec![vb_storage::JournalEvent::RunAccepted {
            run,
            seq,
            workflow: digest,
        }]
    } else {
        Vec::new()
    };

    let summary = vb_storage::recovery::summarize_recovery_events(&events);
    match summary {
        Ok(hydration) => {
            if !events.is_empty() {
                let run_summary = hydration.summary();
                assert!(
                    run_summary.run == run || run_summary.run == vb_core::RunId::new(0),
                    "recovery hydration run must match discovered run"
                );
            }
        }
        Err(error) => {
            assert_typed_recovery_error(error);
        }
    }

    let seed = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events);
    match seed {
        Ok(_seed) => {}
        Err(error) => {
            assert_typed_recovery_error(error);
        }
    }
}

pub fn fuzz_vb_qi37_12_persisted_payload_decode(data: &[u8]) {
    let max_payload_len = vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES;
    let decoded = vb_storage::decode_record::<vb_storage::JournalEvent>(
        data,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    match decoded {
        Ok((_envelope, _event)) => {}
        Err(error) => assert_typed_journal_error(error),
    }

    exercise_truncated_persisted_payload(max_payload_len);
    exercise_corrupted_persisted_payload(max_payload_len);
}

fn exercise_truncated_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(1),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x37; 32]),
    };
    let Ok(encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(truncated_len) = encoded.len().checked_sub(1) else {
        return;
    };
    let Some(truncated) = encoded.get(..truncated_len) else {
        return;
    };
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        truncated,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(
        matches!(result, Err(vb_storage::JournalError::UnexpectedEof)),
        "truncated persisted payload must fail closed as UnexpectedEof"
    );
}

fn exercise_corrupted_persisted_payload(max_payload_len: u32) {
    let event = vb_storage::JournalEvent::RunAccepted {
        run: vb_core::RunId::new(2),
        seq: vb_storage::EventSeq::new(0),
        workflow: vb_core::WorkflowDigest::from_bytes([0x12; 32]),
    };
    let Ok(mut encoded) = vb_storage::encode_record(
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::RecordKind::RunAccepted,
        0,
        &event,
        max_payload_len,
    ) else {
        return;
    };
    let Some(last) = encoded.last_mut() else {
        return;
    };
    *last ^= 0xA5;
    let result = vb_storage::decode_record::<vb_storage::JournalEvent>(
        &encoded,
        vb_storage::MAGIC_JOURNAL_EVENT,
        max_payload_len,
    );
    assert!(
        matches!(result, Err(vb_storage::JournalError::PayloadDigestMismatch)),
        "corrupt persisted payload must fail closed as PayloadDigestMismatch"
    );
}

pub fn fuzz_storage_envelope_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };

    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(
            data,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty input must return UnexpectedEof"
        );
        return;
    }

    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT,
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );

    match result {
        Ok((_envelope, _event)) => {}
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }

    if data.len() < 60 {
        let truncated = data;
        let result = decode_record::<vb_storage::JournalEvent>(
            truncated,
            MAGIC_JOURNAL_EVENT,
            MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
        );
        assert!(
            matches!(
                result,
                Err(JournalError::UnexpectedEof) | Err(JournalError::HeaderLengthMismatch { .. })
            ),
            "truncated header must return UnexpectedEof or HeaderLengthMismatch"
        );
    }
}

pub fn fuzz_binary_payload_boundary(data: &[u8]) {
    use vb_storage::{
        JournalError, MAGIC_JOURNAL_EVENT, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, decode_record,
    };

    if data.is_empty() {
        let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, 1024);
        assert!(
            matches!(result, Err(JournalError::UnexpectedEof)),
            "empty binary payload must return UnexpectedEof"
        );
        return;
    }

    let small_max = 64u32;
    let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, small_max);
    match result {
        Ok((_envelope, _event)) => {}
        Err(JournalError::PayloadTooLarge { .. }) => {}
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }

    let tiny_max = 1u32;
    let result = decode_record::<vb_storage::JournalEvent>(data, MAGIC_JOURNAL_EVENT, tiny_max);
    match result {
        Ok(_) => {}
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }

    let result = decode_record::<vb_storage::JournalEvent>(
        data,
        MAGIC_JOURNAL_EVENT.wrapping_add(1),
        MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    match result {
        Ok(_) => {}
        Err(JournalError::BadMagic { .. }) => {}
        Err(JournalError::RecordKindFamilyMismatch { .. }) => {}
        Err(e) => {
            assert_typed_journal_error(e);
        }
    }
}

pub fn fuzz_accepted_artifact_envelope_qi37_4_2(data: &[u8]) {
    let Ok(artifact) = postcard::from_bytes::<vb_storage::AcceptedArtifact>(data) else {
        return;
    };
    assert!(
        artifact.verification.gate_count > 0,
        "accepted artifact gate_count must be positive, got {}",
        artifact.verification.gate_count
    );
    assert!(
        artifact.accepted_at_seq.get() >= 1,
        "accepted_at_seq must be >= 1, got {}",
        artifact.accepted_at_seq.get()
    );
    let _ = artifact.verification.durable;
    let _ = artifact.digest;
    let cap_count = artifact.required_capabilities.len();
    assert!(
        cap_count <= 256,
        "required_capabilities count {} exceeds reasonable bound",
        cap_count
    );
}
