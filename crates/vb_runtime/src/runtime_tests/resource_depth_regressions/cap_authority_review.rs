fn review_strict_owned_shard(journal: Arc<vb_storage::FjallJournal>) -> crate::shard::Shard {
    crate::shard::Shard::new_with_journal(
        ShardConfig {
            policy: RuntimePolicy::Strict,
            ..runtime_config()
        },
        crate::journal::StorageRuntimeJournal::shared_strict(journal),
    )
}

#[test]
fn review_raw_owned_activation_drains_then_preserves_active_cap()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::shard::ShardCommand;
    let (_directory, journal, run, source) = review_owned_cap_journal()?;
    let mut shard = review_strict_owned_shard(journal.clone());
    shard.enqueue(ShardCommand::ActivateOwnedSubmission {
        run,
        expected_source_digest: source,
    })?;
    assert_eq!(shard.command_queue_pending_for(run), 1);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.command_queue_pending_for(run), 0);
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.active_run_count(), 1);
    let state = shard.run_state_get(run).ok_or("active state")?;
    assert_eq!(
        state
            .admission
            .as_ref()
            .and_then(|a| a.budget())
            .map(|b| b.max_queue_depth),
        Some(1)
    );
    let frame = state.frame.checkpoint_v1();
    let store = state.store.checkpoint_v1();
    let events = journal.events_for_run_full(run)?;
    let quota = shard.global_cost_usage();
    shard.enqueue(ShardCommand::Cancel { run, reason: None })?;
    assert_eq!(
        shard.enqueue(ShardCommand::Cancel { run, reason: None }),
        Err(RuntimeError::PerRunQueueDepthExceeded {
            run,
            pending: 1,
            limit: 1
        })
    );
    let state = shard
        .run_state_get(run)
        .ok_or("active state after refusal")?;
    assert_eq!(state.frame.checkpoint_v1(), frame);
    assert_eq!(state.store.checkpoint_v1(), store);
    assert_eq!(journal.events_for_run_full(run)?, events);
    assert_eq!(shard.global_cost_usage(), quota);
    assert_eq!(shard.tick(), Ok(true));
    assert_eq!(shard.command_queue_pending_for(run), 0);
    assert_eq!(shard.command_queue_len(), 0);
    assert_eq!(shard.active_run_count(), 0);
    assert_eq!(shard.global_cost_usage(), 0);
    let cancelled = journal.events_for_run_full(run)?;
    assert_eq!(
        cancelled
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunCancelled { .. }))
            .count(),
        1
    );
    assert_eq!(
        cancelled
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunAccepted { .. }))
            .count(),
        1
    );
    assert_eq!(
        cancelled
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunAdmissionBudget { .. }))
            .count(),
        1
    );
    assert_eq!(
        cancelled
            .iter()
            .filter(|event| matches!(event, vb_storage::JournalEvent::RunBudgetOriginEvent { .. }))
            .count(),
        1
    );
    shard.enqueue(ShardCommand::Shutdown)?;
    let _progressed = shard.tick()?;
    assert_eq!(
        shard.enqueue(ShardCommand::ActivateOwnedSubmission {
            run: RunId::new(999),
            expected_source_digest: WorkflowDigest::from_bytes([7; 32]),
        }),
        Err(RuntimeError::ShutdownInProgress)
    );
    assert_eq!(journal.events_for_run_full(run)?, cancelled);
    assert_eq!(shard.command_queue_len(), 0);
    Ok(())
}

#[test]
fn review_verified_cap_token_is_bound_to_run_and_source() -> Result<(), Box<dyn std::error::Error>>
{
    let (_directory, journal, run, source) = review_owned_cap_journal()?;
    let shard = review_strict_owned_shard(journal.clone());
    let before = journal.events_for_run_full(run)?;
    for (candidate_run, candidate_source) in [
        (RunId::new(999), source),
        (run, WorkflowDigest::from_bytes([7; 32])),
    ] {
        let verified = shard.preflight_owned_activation(run, source)?;
        assert_eq!(
            shard.enqueue_owned_activation(candidate_run, candidate_source, verified),
            Err(RuntimeError::UnsupportedOperation {
                operation: "owned_activation_queue_authority_mismatch"
            })
        );
        assert_eq!(shard.command_queue_len(), 0);
        assert_eq!(shard.command_queue_pending_for(run), 0);
        assert_eq!(shard.command_queue_pending_for(candidate_run), 0);
        assert_eq!(shard.active_run_count(), 0);
        assert_eq!(shard.global_cost_usage(), 0);
        assert_eq!(journal.events_for_run_full(run)?, before);
    }
    Ok(())
}
