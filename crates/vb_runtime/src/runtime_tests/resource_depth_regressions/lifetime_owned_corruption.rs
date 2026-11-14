fn owned_submission_for_run(
    run: vb_core::RunId,
    now: u64,
) -> Result<
    (
        vb_core::WorkflowDigest,
        vb_core::WorkflowDigest,
        vb_storage::RunSubmission,
    ),
    String,
> {
    let deadline = now.checked_add(60_000).ok_or("deadline overflow")?;
    let workflow = owned_waiting_workflow(deadline)?;
    let source_bytes = b"owned workflow source".to_vec();
    let source_digest =
        vb_core::WorkflowDigest::from_bytes(*blake3::hash(&source_bytes).as_bytes());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Strict, &[])
            .map_err(|error| error.to_string())?;
    let artifact_digest = artifact.digest;
    let checkpoint =
        vb_core::value_store::ValueStore::with_max_slots(workflow.resource_contract().max_slots)
            .checkpoint_v1();
    let bundle = vb_core::root_bundle::OwnedRootBundle::new(Box::default(), checkpoint, &workflow)
        .map_err(|error| error.to_string())?;
    let inputs = vb_storage::OwnedRunInputsRecordV1::new(
        run,
        artifact.digest,
        source_digest,
        &bundle,
        &workflow,
    )
    .map_err(|error| error.to_string())?;
    let submission = vb_storage::RunSubmission::owned(
        vb_storage::WorkflowSourceRecord {
            digest: source_digest,
            source: source_bytes,
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
    )
    .map_err(|error| error.to_string())?;
    Ok((source_digest, artifact_digest, submission))
}

pub(crate) fn owned_journal_for_run(
    run: vb_core::RunId,
    now: u64,
) -> Result<
    (
        tempfile::TempDir,
        Arc<vb_storage::FjallJournal>,
        vb_core::WorkflowDigest,
    ),
    String,
> {
    let (source, _, submission) = owned_submission_for_run(run, now)?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    journal
        .commit_run_submission(&submission, RuntimePolicy::Strict)
        .map_err(|error| error.to_string())?;
    Ok((directory, journal, source))
}

fn strict_runtime_for_owned_journal(
    journal: Arc<vb_storage::FjallJournal>,
) -> Result<Runtime, String> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("one shard")?,
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal),
    ))
}

#[test]
fn owned_activation_rejects_duplicate_durable_budget_origin_without_mutation() -> Result<(), String>
{
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(906);
    let (_directory, journal, source) = owned_journal_for_run(run, now)?;
    journal
        .append_strict(&vb_storage::JournalEvent::RunBudgetOriginEvent {
            run,
            seq: vb_storage::EventSeq::new(4),
            started_millis: now,
        })
        .map_err(|error| error.to_string())?;
    let before = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let runtime = strict_runtime_for_owned_journal(Arc::clone(&journal))?;
    let pending = runtime.pending_boundary_snapshot(8);
    assert!(matches!(
        runtime.activate_owned_submission(run, source),
        Err(RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::OwnedInputs(
                vb_storage::OwnedInputError::IdentityMismatch
            ))
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(
        journal
            .events_for_run_full(run)
            .map_err(|error| error.to_string())?,
        before
    );
    Ok(())
}

#[test]
fn owned_activation_propagates_persisted_origin_decode_error_without_mutation() -> Result<(), String>
{
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(907);
    let (_directory, journal, source) = owned_journal_for_run(run, now)?;
    journal
        .inject_raw_event(
            run,
            vb_storage::EventSeq::new(3),
            vb_storage::RecordKind::RunBudgetOrigin,
            &[0xde, 0xad, 0xbe, 0xef],
        )
        .map_err(|error| error.to_string())?;
    let raw_before = journal
        .get_event_bytes(run, vb_storage::EventSeq::new(3))
        .map_err(|error| error.to_string())?;
    let runtime = strict_runtime_for_owned_journal(Arc::clone(&journal))?;
    let pending = runtime.pending_boundary_snapshot(8);
    assert!(matches!(
        runtime.activate_owned_submission(run, source),
        Err(RuntimeError::StorageJournalAppend { source })
            if matches!(source.as_ref(), vb_storage::JournalError::PostcardDecodeFailed(_))
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(
        journal
            .get_event_bytes(run, vb_storage::EventSeq::new(3))
            .map_err(|error| error.to_string())?,
        raw_before
    );
    Ok(())
}

struct RestoreOwnedClock;

impl Drop for RestoreOwnedClock {
    fn drop(&mut self) {
        crate::shard::helpers::timer::restore_wall_clock_for_test();
    }
}

#[test]
fn owned_activation_fails_closed_when_wall_clock_is_unavailable() -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(915);
    let (_directory, journal, source) = owned_journal_for_run(run, now)?;
    let before = journal
        .events_for_run_full(run)
        .map_err(|error| error.to_string())?;
    let runtime = strict_runtime_for_owned_journal(Arc::clone(&journal))?;
    let pending = runtime.pending_boundary_snapshot(8);
    let _restore = RestoreOwnedClock;
    crate::shard::helpers::timer::install_wall_clock_unavailable_for_test();
    assert!(matches!(
        runtime.activate_owned_submission(run, source),
        Err(RuntimeError::RecoveryCannotResume { reason })
            if reason == "wall_clock_unavailable"
    ));
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(
        journal
            .events_for_run_full(run)
            .map_err(|error| error.to_string())?,
        before
    );
    Ok(())
}

#[test]
fn future_owned_origin_is_rejected_before_durable_admission_for_every_policy() -> Result<(), String>
{
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let future = now
        .checked_add(3_600_000)
        .ok_or("future timestamp overflow")?;
    for (run, policy) in [
        (vb_core::RunId::new(908), RuntimePolicy::Strict),
        (vb_core::RunId::new(909), RuntimePolicy::Journaled),
        (vb_core::RunId::new(910), RuntimePolicy::Relaxed),
    ] {
        let (source_digest, artifact_digest, submission) = owned_submission_for_run(run, future)?;
        let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
        let journal = vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?;
        assert!(matches!(
            journal.commit_run_submission(&submission, policy),
            Err(vb_storage::JournalError::InvalidConfig {
                field: "run_budget_origin",
                reason: "future"
            })
        ));
        assert!(
            journal
                .run_header(run)
                .map_err(|error| error.to_string())?
                .is_none()
        );
        assert!(
            journal
                .run_inputs(run)
                .map_err(|error| error.to_string())?
                .is_none()
        );
        assert!(
            journal
                .workflow_source(source_digest)
                .map_err(|error| error.to_string())?
                .is_none()
        );
        assert!(
            journal
                .compiled_ir(artifact_digest)
                .map_err(|error| error.to_string())?
                .is_none()
        );
        assert!(
            journal
                .events_for_run_full(run)
                .map_err(|error| error.to_string())?
                .is_empty()
        );
    }
    Ok(())
}
