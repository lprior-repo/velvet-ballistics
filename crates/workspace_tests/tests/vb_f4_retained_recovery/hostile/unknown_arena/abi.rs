use super::*;

#[test]
fn scheduled_and_completed_action_abi_must_match_accepted_artifact() -> TestResult {
    let (_directory, mut events) = super::completed_trace(SlotValue::I64(19))?;
    let directory = super::replay_trace(&events)?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(ACTION_RUN)?;
    let before_bytes = postcard::to_allocvec(&before)?;
    let valid_runtime = runtime(journal.clone());
    let product = valid_runtime.recover_product(ACTION_RUN);
    assert!(
        matches!(product, Ok(RuntimeRecoveryProduct::Resumable(_))),
        "valid accepted action ABI must recover: {product:?}"
    );
    super::assert_unmodified(&valid_runtime, &journal, &before, &before_bytes, ACTION_RUN)?;
    drop(valid_runtime);
    drop(journal);

    let forged = WorkflowDigest::from_bytes([0x99; 32]);
    let mut changed = 0;
    for event in &mut events {
        match event {
            JournalEvent::ActionScheduledTicket {
                action_abi_digest, ..
            }
            | JournalEvent::ActionCompletedEnvelope {
                action_abi_digest, ..
            } => {
                *action_abi_digest = forged;
                changed += 1;
            }
            _ => {}
        }
    }
    assert_eq!(changed, 2);
    let directory = super::replay_trace(&events)?;
    let journal = Arc::new(FjallJournal::open(directory.path(), None)?);
    let before = journal.events_for_run_full(ACTION_RUN)?;
    let before_bytes = postcard::to_allocvec(&before)?;
    let forged_runtime = runtime(journal.clone());
    let product = forged_runtime.recover_product(ACTION_RUN);
    assert!(
        matches!(product, Err(RuntimeError::RecoveryCannotResume { ref reason }) if reason == "action_abi_digest_mismatch"),
        "both forged envelopes must fail against accepted ABI: {product:?}"
    );
    super::assert_unmodified(
        &forged_runtime,
        &journal,
        &before,
        &before_bytes,
        ACTION_RUN,
    )?;
    assert!(matches!(
        forged_runtime.recover_and_resume(ACTION_RUN),
        Err(RuntimeError::RecoveryCannotResume { ref reason })
            if reason == "action_abi_digest_mismatch"
    ));
    super::assert_unmodified(
        &forged_runtime,
        &journal,
        &before,
        &before_bytes,
        ACTION_RUN,
    )
}
