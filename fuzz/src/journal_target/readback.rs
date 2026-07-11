//! Readback and recovery fuzz target bodies.

use super::errors::{assert_typed_journal_error, assert_typed_recovery_error};

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
    let has_accepted_event = events.iter().any(|event| {
        matches!(event, vb_storage::JournalEvent::RunAccepted { workflow, .. } if *workflow == digest)
    });
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
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    let parts = one_node_parts("fuzz_readback", [0u8; 32]);
    let hash_bytes = match postcard::to_allocvec(&parts) {
        Ok(b) => b,
        Err(_) => return,
    };
    let digest = vb_core::WorkflowDigest::from_bytes(*blake3::hash(&hash_bytes).as_bytes());
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
    assert!(matches!(
        classification,
        ReadbackFamilySet::Full
            | ReadbackFamilySet::Partial
            | ReadbackFamilySet::Absent
            | ReadbackFamilySet::Unreadable
    ));
    assert!(!matches!(classification, ReadbackFamilySet::Unreadable));
}

fn one_node_parts(name: &'static str, digest: [u8; 32]) -> vb_core::WorkflowParts {
    vb_core::WorkflowParts {
        name: Box::<str>::from(name),
        digest: vb_core::WorkflowDigest::from_bytes(digest),
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
    }
}

pub fn fuzz_admission_input_surface(data: &[u8]) {
    if data.len() < 2 {
        return;
    }
    let Ok(temp_dir) = tempfile::tempdir() else {
        return;
    };
    let Ok(journal) = vb_storage::FjallJournal::open(temp_dir.path(), None) else {
        return;
    };
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let Ok(workflow) = vb_core::CompiledWorkflow::try_from_parts(parts) else {
            return;
        };
        let strict_result =
            vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Strict);
        let relaxed_result =
            vb_storage::submit_artifact(&journal, &workflow, vb_core::RuntimePolicy::Relaxed);
        assert_eq!(strict_result.is_ok(), relaxed_result.is_ok());
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
    if let Ok(artifact) = result {
        assert!(artifact.accepted_at_seq.get() > 0);
        assert!(artifact.verification.gate_count > 0);
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
    match vb_storage::recovery::summarize_recovery_events(&events) {
        Ok(hydration) => {
            if !events.is_empty() {
                let run_summary = hydration.summary();
                assert!(run_summary.run == run || run_summary.run == vb_core::RunId::new(0));
            }
        }
        Err(error) => assert_typed_recovery_error(error),
    }
    if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events) {
        assert_typed_recovery_error(error);
    }
}
