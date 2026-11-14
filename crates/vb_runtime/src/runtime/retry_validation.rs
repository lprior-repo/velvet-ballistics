impl Runtime {
    /// Validates a caller-supplied retry checkpoint against the run's durable journal.
    ///
    /// The durable failure boundary ends in `RunFailedEvent { seq: S, attempt: A }` followed by
    /// `RetryCheckpoint { seq: S+1, attempt: A+1 }`. The supplied checkpoint must decode to the
    /// exact latest durable `RetryCheckpoint` payload (binding run/step/attempt/prior_seq/
    /// artifact/workflow/policy/grants/frame/store/collect/contracts), its `prior_seq` must name
    /// the immediately preceding `RunFailedEvent` sequence, and `next_seq` must be the sequence
    /// immediately after the checkpoint. Checked arithmetic throughout; every rejection is
    /// fail-closed with no state or journal mutation.
    fn validate_retry_checkpoint_against_durable_chain(
        &self,
        run: RunId,
        checkpoint: &crate::run_checkpoint::RunCheckpointV1,
        next_seq: vb_storage::EventSeq,
    ) -> RuntimeResult<()> {
        let journal = self.recovery_storage_journal()?;
        let events = journal
            .events_for_run_full(run)
            .map_err(|error| RuntimeError::Recovery {
                error: format!("retry journal read failed: {error}"),
            })?;
        validate_retry_budget_origin(&events, run, checkpoint)?;
        let durable = latest_retry_checkpoint(&events, run)?;
        validate_durable_retry_checkpoint(&events, run, checkpoint, next_seq, durable)
    }

    fn rejected_recovery_product_error(
        product: crate::recovery::RuntimeRecoveryProduct,
    ) -> RuntimeError {
        match product {
            crate::recovery::RuntimeRecoveryProduct::CannotResume(product) => {
                RuntimeError::RecoveryCannotResume {
                    reason: String::from(product.reason()),
                }
            }
            crate::recovery::RuntimeRecoveryProduct::SummaryOnly(_)
            | crate::recovery::RuntimeRecoveryProduct::Resumable(_) => {
                RuntimeError::UnsupportedFullRecoveryHydration
            }
        }
    }
}

fn latest_retry_checkpoint(
    events: &[vb_storage::JournalEvent],
    run: RunId,
) -> RuntimeResult<&vb_storage::JournalEvent> {
    events
        .iter()
        .rev()
        .find(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RetryCheckpoint { run: event_run, .. }
                    if *event_run == run
            )
        })
        .ok_or_else(|| RuntimeError::Recovery {
            error: "run has no durable retry checkpoint".to_string(),
        })
}

fn validate_durable_retry_checkpoint(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
    next_seq: vb_storage::EventSeq,
    durable: &vb_storage::JournalEvent,
) -> RuntimeResult<()> {
    let vb_storage::JournalEvent::RetryCheckpoint {
        run: durable_run,
        seq: durable_seq,
        attempt: durable_attempt,
        payload_digest: durable_payload_digest,
        payload: durable_payload,
        ..
    } = durable
    else {
        return Err(RuntimeError::Recovery {
            error: "run has no durable retry checkpoint".to_string(),
        });
    };

    validate_retry_checkpoint_payload(durable_payload, durable_payload_digest, checkpoint)?;
    validate_retry_checkpoint_attempt(run, durable_run, *durable_attempt, checkpoint)?;
    validate_retry_checkpoint_position(events, run, durable_seq, checkpoint, next_seq)
}

fn validate_retry_checkpoint_payload(
    payload: &[u8],
    expected_digest: &[u8; 32],
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
) -> RuntimeResult<()> {
    if blake3::hash(payload).as_bytes() != expected_digest {
        return Err(RuntimeError::Recovery {
            error: "durable retry checkpoint payload digest mismatch".to_string(),
        });
    }
    let durable_checkpoint =
        crate::run_checkpoint::decode_checkpoint_payload(payload).map_err(|error| {
            RuntimeError::Recovery {
                error: format!("durable retry checkpoint decode failed: {error}"),
            }
        })?;
    if durable_checkpoint != *checkpoint {
        return Err(RuntimeError::Recovery {
            error: "retry checkpoint does not match durable RetryCheckpoint evidence".to_string(),
        });
    }
    Ok(())
}

fn validate_retry_checkpoint_attempt(
    run: RunId,
    durable_run: &RunId,
    durable_attempt: u16,
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
) -> RuntimeResult<()> {
    if *durable_run != run {
        return Err(RuntimeError::Recovery {
            error: "durable retry checkpoint belongs to a different run".to_string(),
        });
    }
    if checkpoint.run_attempt.get() != durable_attempt {
        return Err(RuntimeError::Recovery {
            error: "retry checkpoint run attempt does not match durable RetryCheckpoint"
                .to_string(),
        });
    }
    Ok(())
}

fn validate_retry_checkpoint_position(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    durable_seq: &vb_storage::EventSeq,
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
    next_seq: vb_storage::EventSeq,
) -> RuntimeResult<()> {
    validate_checkpoint_prior_position(checkpoint, durable_seq)?;
    validate_checkpoint_next_position(durable_seq, next_seq)?;
    validate_checkpoint_head(events, durable_seq)?;
    validate_preceding_failure(events, run, checkpoint)
}

fn validate_checkpoint_prior_position(
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
    durable_seq: &vb_storage::EventSeq,
) -> RuntimeResult<()> {
    let expected = checkpoint
        .prior_seq
        .checked_add(1)
        .ok_or_else(|| RuntimeError::Recovery {
            error: "retry checkpoint prior_seq overflow".to_string(),
        })?;
    if durable_seq.get() == expected {
        Ok(())
    } else {
        Err(RuntimeError::Recovery {
            error: "retry checkpoint prior_seq does not match durable RetryCheckpoint position"
                .to_string(),
        })
    }
}

fn validate_checkpoint_next_position(
    durable_seq: &vb_storage::EventSeq,
    next_seq: vb_storage::EventSeq,
) -> RuntimeResult<()> {
    let expected = durable_seq
        .get()
        .checked_add(1)
        .ok_or_else(|| RuntimeError::Recovery {
            error: "retry checkpoint next_seq overflow".to_string(),
        })?;
    if next_seq.get() == expected {
        Ok(())
    } else {
        Err(RuntimeError::Recovery {
            error: "retry next_seq does not match durable RetryCheckpoint position".to_string(),
        })
    }
}

fn validate_checkpoint_head(
    events: &[vb_storage::JournalEvent],
    durable_seq: &vb_storage::EventSeq,
) -> RuntimeResult<()> {
    if events
        .last()
        .is_some_and(|event| event.seq() == *durable_seq)
    {
        Ok(())
    } else {
        Err(RuntimeError::Recovery {
            error: "retry checkpoint has already been consumed (journal advanced past it)"
                .to_string(),
        })
    }
}

fn validate_preceding_failure(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
) -> RuntimeResult<()> {
    let expected_attempt = expected_failed_run_attempt(checkpoint)?;
    let preceding_failed = events.iter().find(|event| {
        event.seq().get() == checkpoint.prior_seq
            && matches!(
                event,
                vb_storage::JournalEvent::RunFailedEvent { run: event_run, .. }
                    if *event_run == run
            )
    });
    match preceding_failed {
        Some(vb_storage::JournalEvent::RunFailedEvent { attempt, .. })
            if *attempt == expected_attempt =>
        {
            Ok(())
        }
        _ => Err(RuntimeError::Recovery {
            error: "retry checkpoint does not follow the run's own durable RunFailed event"
                .to_string(),
        }),
    }
}

fn expected_failed_run_attempt(
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
) -> RuntimeResult<u16> {
    checkpoint
        .run_attempt
        .get()
        .checked_sub(1)
        .ok_or_else(|| RuntimeError::Recovery {
            error: "retry checkpoint run_attempt underflow".to_string(),
        })
}

fn validate_retry_budget_origin(
    events: &[vb_storage::JournalEvent],
    run: RunId,
    checkpoint: &crate::run_checkpoint::RunCheckpointV1,
) -> RuntimeResult<()> {
    let admission = crate::recovery::hydrate_run_admission_from_events(events).ok_or(
        RuntimeError::RecoveryCannotResume {
            reason: String::from("admission_missing"),
        },
    )?;
    let origin = crate::recovery::recover_run_budget_origin(events, run, &admission)?;
    let validated =
        crate::recovery::validate_recovered_lifetime_at_wall_clock(Some(&admission), origin)?;
    if checkpoint.run_started_millis == validated {
        Ok(())
    } else {
        Err(RuntimeError::RecoveryCannotResume {
            reason: String::from("budget_origin_invalid"),
        })
    }
}
