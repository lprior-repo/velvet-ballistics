#[test]
fn atomic_issued_owned_submission_activates_from_persisted_authority_after_reopen()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let workflow = honest_digest_workflow(two_slot_workflow().ok_or("workflow")?)?;
    let policy = vb_core::RuntimePolicy::Strict;
    let artifact = vb_storage::admission::prepare_compiled_artifact(&workflow, policy, &[])?;
    assert_eq!(artifact.accepted_at_seq.get(), 0);
    let prepared_digest = artifact.digest;
    let source = b"issued owned activation".to_vec();
    let source_digest = WorkflowDigest::from_bytes(blake3::hash(&source).into());
    let run = RunId::new(91_005);
    let checkpoint =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots)
            .checkpoint_v1();
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(Box::default(), checkpoint, &workflow)?;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )?;
    let now = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?;
    let submission = vb_storage::RunSubmission::owned(
        vb_storage::WorkflowSourceRecord {
            source,
            digest: source_digest,
        },
        artifact,
        inputs,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: now,
        },
    )?;
    {
        let journal = vb_storage::FjallJournal::open(directory.path(), None)?;
        journal.commit_run_submission(&submission, policy)?;
    }
    let journal = std::sync::Arc::new(vb_storage::FjallJournal::open(directory.path(), None)?);
    let events = journal.events_for_run(run)?;
    let issued_digest = match events.get(1).ok_or("admission")? {
        vb_storage::JournalEvent::RunAdmissionBudget {
            artifact_digest,
            input_digest: vb_storage::RunInputBinding::Digest(_),
            policy_pin: vb_storage::RunPolicyPin::Pinned(_),
            ..
        } => *artifact_digest,
        _ => return Err("missing current authority".into()),
    };
    assert_ne!(issued_digest, prepared_digest);
    let artifact: vb_storage::AcceptedArtifact =
        postcard::from_bytes(&journal.compiled_ir(issued_digest)?.ok_or("artifact")?.ir)?;
    assert_eq!(artifact.accepted_at_seq.get(), 1);
    let mut config = small_config();
    config.policy = policy;
    config.max_active_runs = 1;
    let mut shard = Shard::new_with_journal_and_artifact_store(
        config,
        crate::journal::StorageRuntimeJournal::shared_strict(journal.clone()),
        crate::admission::StorageArtifactStore::shared(journal.clone()),
    ).expect("valid test configuration");
    shard.enqueue(ShardCommand::ActivateOwnedSubmission {
        run,
        expected_source_digest: source_digest,
    })?;
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.counters().snapshot().runs_failed, 0);
    let after = journal.events_for_run(run)?;
    assert_eq!(after.get(..4), events.get(..4));
    assert_eq!(shard.counters().snapshot().runs_submitted, 1);
    assert_eq!(shard.counters().snapshot().runs_completed, 1);
    assert_eq!(shard.active_run_count(), 0);
    assert!(matches!(after.last(), Some(vb_storage::JournalEvent::RunFinished { run: owner, result, attempt: 1, .. }) if *owner == run && *result == SlotIdx::new(1)));
    let writes: Result<Vec<_>, postcard::Error> = after.iter().filter_map(|event| {
        match event {
            vb_storage::JournalEvent::SlotWrittenEvent { slot, value: Some(bytes), .. } => Some(postcard::from_bytes::<SlotValue>(bytes).map(|value| (*slot, value))),
            _ => None,
        }
    }).collect();
    assert_eq!(writes?, vec![(SlotIdx::ZERO, SlotValue::I64(10)), (SlotIdx::new(1), SlotValue::I64(10))]);
    Ok(())
}
