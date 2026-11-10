use super::*;

#[test]
fn retry_timer_wire_version_and_bounds_are_explicit() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    let workflow = compile(3, "1h", false)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    submit(&mut runtime, workflow)?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    let event = journal
        .events_for_run_full(RUN)?
        .into_iter()
        .find(|event| matches!(event, JournalEvent::RetryTimerScheduled { .. }))
        .ok_or("timer missing")?;
    let bytes = postcard::to_allocvec(&event)?;
    assert_eq!(bytes.first(), Some(&51));
    assert_eq!(postcard::from_bytes::<JournalEvent>(&bytes)?, event);
    let JournalEvent::RetryTimerScheduled {
        run,
        seq,
        step,
        attempt,
        timer,
    } = event
    else {
        return Err("timer shape".into());
    };
    for case in 0..5 {
        let mut invalid = timer.clone();
        match case {
            0 => invalid.version = 2,
            1 => invalid.next_attempt = 1,
            2 => invalid.deadline_unix_ms = invalid.scheduled_unix_ms,
            3 => invalid.generation = 0,
            _ => invalid.checkpoint_digest = [0; 32],
        }
        let mut raw = vec![51u8];
        raw.extend(postcard::to_allocvec(&(run, seq, step, attempt, &invalid))?);
        assert!(
            matches!(
                postcard::from_bytes::<JournalEvent>(&raw),
                Err(postcard::Error::SerdeDeCustom)
            ),
            "case {case}"
        );
        assert!(
            matches!(
                postcard::to_allocvec(&JournalEvent::RetryTimerScheduled {
                    run,
                    seq,
                    step,
                    attempt,
                    timer: invalid
                }),
                Err(postcard::Error::SerdeSerCustom)
            ),
            "case {case}"
        );
    }
    Ok(())
}

#[test]
fn retry_checkpoint_identity_tampering_cannot_admit_or_write() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    let workflow = compile(3, "1h", false)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    submit(&mut runtime, workflow.clone())?;
    fail(&mut runtime, ActionFailureCode::Timeout)?;
    let original = journal.events_for_run_full(RUN)?;
    for case in 0..5 {
        let target_directory = tempfile::tempdir()?;
        let (target, mut recovered) = open(target_directory.path())?;
        vb_storage::submit_artifact_with_contracts(
            &target,
            &workflow,
            RuntimePolicy::Journaled,
            &[contract()?],
        )?;
        for mut event in original.clone() {
            if let JournalEvent::RetryTimerScheduled { timer, .. } = &mut event {
                let mut checkpoint: vb_runtime::run_checkpoint::RunCheckpointV1 =
                    postcard::from_bytes(&timer.checkpoint)?;
                match case {
                    0 => {
                        checkpoint.prior_seq = checkpoint
                            .prior_seq
                            .checked_add(1)
                            .ok_or("sequence overflow")?
                    }
                    1 => {
                        *checkpoint
                            .action_attempts
                            .get_mut(1)
                            .ok_or("counter missing")? = 3
                    }
                    2 => {
                        timer.generation = timer
                            .generation
                            .checked_add(1)
                            .ok_or("generation overflow")?
                    }
                    3 => {
                        timer.deadline_unix_ms = timer
                            .deadline_unix_ms
                            .checked_add(1)
                            .ok_or("deadline overflow")?
                    }
                    _ => checkpoint.run_id = vb_core::RunId::new(77),
                }
                timer.checkpoint = postcard::to_allocvec(&checkpoint)?;
                timer.checkpoint_digest = *blake3::hash(&timer.checkpoint).as_bytes();
                timer.validate()?;
            }
            target.append_strict(&event)?;
        }
        let before = target.events_for_run_full(RUN)?;
        let expected = if case == 4 {
            "retry timer: checkpoint frame belongs to a different run"
        } else {
            "retry timer: checkpoint/failure/policy identity mismatch"
        };
        assert!(
            matches!(recovered.recover_product(RUN), Err(RuntimeError::Recovery {error}) if error == expected),
            "case {case}"
        );
        assert!(
            matches!(recovered.recover_and_resume(RUN), Err(RuntimeError::Recovery {error}) if error == expected),
            "case {case}"
        );
        recovered.tick_all()?;
        assert!(matches!(
            recovered.lookup_pending_action_ticket(RUN, 1),
            None
        ));
        assert!(matches!(
            recovered.capture_timer_entry(RUN),
            Err(RuntimeError::RunNotFound)
        ));
        assert_eq!(target.events_for_run_full(RUN)?, before);
        assert_eq!(recovered.counters_snapshot().runs_submitted, 0);
    }
    Ok(())
}

pub(super) fn reject_duplicate_action_schedule(events: &[JournalEvent]) -> TestResult {
    let directory = tempfile::tempdir()?;
    let (journal, mut runtime) = open(directory.path())?;
    let workflow = compile(3, "2s", false)?;
    vb_storage::submit_artifact_with_contracts(
        &journal,
        &workflow,
        RuntimePolicy::Journaled,
        &[contract()?],
    )?;
    for event in events {
        journal.append_strict(event)?;
    }
    let mut duplicate = events.last().cloned().ok_or("schedule missing")?;
    let JournalEvent::ActionScheduledTicket { seq, .. } = &mut duplicate else {
        return Err("schedule shape".into());
    };
    *seq = vb_storage::EventSeq::new(seq.get().checked_add(1).ok_or("sequence overflow")?);
    journal.append_strict(&duplicate)?;
    let before = journal.events_for_run_full(RUN)?;
    assert!(
        matches!(runtime.recover_and_resume(RUN), Err(RuntimeError::Recovery {error}) if error == "retry timer: rescheduled action tail length")
    );
    runtime.tick_all()?;
    assert!(matches!(runtime.lookup_pending_action_ticket(RUN, 1), None));
    assert_eq!(journal.events_for_run_full(RUN)?, before);
    Ok(())
}
