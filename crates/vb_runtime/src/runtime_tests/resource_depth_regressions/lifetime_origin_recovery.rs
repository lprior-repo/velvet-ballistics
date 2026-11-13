#[derive(Clone, Copy)]
enum OriginMutation {
    Duplicate,
    Late,
}

fn mutate_origin_prefix(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    now: u64,
    mutation: OriginMutation,
) -> Result<(), String> {
    match mutation {
        OriginMutation::Late => journal
            .append_strict(&vb_storage::JournalEvent::RunFailedEvent {
                run,
                seq: vb_storage::EventSeq::new(4),
                attempt: 1,
            })
            .and_then(|()| {
                journal.append_strict(&vb_storage::JournalEvent::RunBudgetOriginEvent {
                    run,
                    seq: vb_storage::EventSeq::new(5),
                    started_millis: now,
                })
            })
            .map_err(|error| error.to_string()),
        OriginMutation::Duplicate => journal
            .append_strict(&vb_storage::JournalEvent::RunBudgetOriginEvent {
                run,
                seq: vb_storage::EventSeq::new(4),
                started_millis: now,
            })
            .map_err(|error| error.to_string()),
    }
}

fn raw_run_rows(
    journal: &vb_storage::FjallJournal,
    run: vb_core::RunId,
    last_seq: u64,
) -> Result<Vec<Vec<u8>>, String> {
    (0..=last_seq)
        .map(|seq| {
            journal
                .get_event_bytes(run, vb_storage::EventSeq::new(seq))
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("event row {seq} is missing"))
        })
        .collect()
}

fn expected_origin_error(mutation: OriginMutation) -> &'static str {
    match mutation {
        OriginMutation::Duplicate | OriginMutation::Late => "budget_origin_invalid",
    }
}

fn missing_origin_journal_for_run(
    run: vb_core::RunId,
    now: u64,
) -> Result<(tempfile::TempDir, Arc<vb_storage::FjallJournal>), String> {
    let workflow = owned_waiting_workflow(now.checked_add(60_000).ok_or("deadline overflow")?)?;
    let source_bytes = b"missing-origin workflow source".to_vec();
    let source_digest =
        vb_core::WorkflowDigest::from_bytes(*blake3::hash(&source_bytes).as_bytes());
    let artifact =
        vb_storage::admission::prepare_compiled_artifact(&workflow, RuntimePolicy::Relaxed, &[])
            .map_err(|error| error.to_string())?;
    let artifact_digest = artifact.digest;
    let input = vb_storage::RunInputsRecord::new(
        run,
        artifact_digest,
        source_digest,
        &workflow,
        Box::default(),
    )
    .map_err(|error| error.to_string())?;
    let input_digest = input.input_digest();
    let submission = vb_storage::RunSubmission::new(
        vb_storage::WorkflowSourceRecord {
            digest: source_digest,
            source: source_bytes,
        },
        artifact,
        input,
        vb_storage::RunHeaderRecord {
            run,
            workflow_id: vb_core::WorkflowId::new(0),
            compiled_digest: workflow.digest(),
            status: 0,
            accepted_at_ms: now,
        },
    )
    .map_err(|error| error.to_string())?;
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    journal
        .commit_run_submission(&submission, RuntimePolicy::Relaxed)
        .map_err(|error| error.to_string())?;
    journal
        .append_strict(&vb_storage::JournalEvent::RunAccepted {
            run,
            seq: vb_storage::EventSeq::new(0),
            workflow: workflow.digest(),
        })
        .map_err(|error| error.to_string())?;
    journal
        .append_strict(&vb_storage::JournalEvent::RunAdmissionBudget {
            run,
            seq: vb_storage::EventSeq::new(1),
            artifact_digest,
            input_digest: vb_storage::RunInputBinding::Digest(input_digest),
            policy_pin: vb_storage::RunPolicyPin::Unverified,
            granted_capabilities: CapabilitySet::empty(),
            policy: RuntimePolicy::Relaxed,
            budget: vb_core::AggregateResourceBudget::from_workflow(&workflow)
                .map_err(|error| error.to_string())?,
        })
        .map_err(|error| error.to_string())?;
    Ok((directory, journal))
}

fn relaxed_runtime_for_journal(journal: Arc<vb_storage::FjallJournal>) -> Result<Runtime, String> {
    Ok(Runtime::new(
        NonZeroUsize::new(1).ok_or("one shard")?,
        ShardConfig {
            policy: RuntimePolicy::Relaxed,
            ..ShardConfig::default()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal),
    ))
}

fn assert_public_origin_refusal(
    mutation: OriginMutation,
    expected_reason: &'static str,
) -> Result<(), String> {
    let now = crate::shard::helpers::wall_clock_millis_since_epoch().ok_or("clock unavailable")?;
    let run = vb_core::RunId::new(920);
    let (directory, journal, _) = owned_journal_for_run(run, now)?;
    mutate_origin_prefix(journal.as_ref(), run, now, mutation)?;
    let last_seq = match mutation {
        OriginMutation::Duplicate => 4,
        OriginMutation::Late => 5,
    };
    let raw_before = raw_run_rows(journal.as_ref(), run, last_seq)?;
    drop(journal);
    let reopened = Arc::new(
        vb_storage::FjallJournal::open(directory.path(), None)
            .map_err(|error| error.to_string())?,
    );
    let _restore_clock = RestoreOriginClock;
    let recovery_clock = match expected_reason {
        "budget_origin_future" => now.checked_sub(1).ok_or("clock underflow")?,
        "run_lifetime_expired" => now
            .checked_add(31 * 24 * 60 * 60 * 1_000)
            .ok_or("expired clock overflow")?,
        _ => now,
    };
    if expected_reason != "recovery error" {
        crate::shard::helpers::timer::install_wall_clock_millis_for_test(recovery_clock);
    }
    let runtime = strict_runtime_for_owned_journal(Arc::clone(&reopened))?;
    let pending = runtime.pending_boundary_snapshot(8);
    let result = runtime.recover_product(run);
    match expected_reason {
        "recovery error" => assert!(matches!(result, Err(RuntimeError::Recovery { .. }))),
        reason => assert!(
            matches!(
                &result,
                Err(RuntimeError::RecoveryCannotResume { reason: found }) if found == reason
            ),
            "unexpected origin recovery outcome: {result:?}"
        ),
    }
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert!(matches!(
        runtime.snapshot_run(run, 1),
        Ok(crate::shard::InspectResponse::NotFound { run: found, .. }) if found == run
    ));
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    assert!(runtime.recover_and_resume(run).is_err());
    assert_eq!(runtime.pending_boundary_snapshot(8), pending);
    assert_eq!(raw_run_rows(reopened.as_ref(), run, last_seq)?, raw_before);
    Ok(())
}
