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

        let durable = events.iter().rev().find(|event| {
            matches!(
                event,
                vb_storage::JournalEvent::RetryCheckpoint { run: r, .. } if *r == run
            )
        });
        let vb_storage::JournalEvent::RetryCheckpoint {
            run: durable_run,
            seq: durable_seq,
            attempt: durable_attempt,
            payload_digest: durable_payload_digest,
            payload: durable_payload,
            ..
        } = durable.ok_or_else(|| RuntimeError::Recovery {
            error: "run has no durable retry checkpoint".to_string(),
        })?
        else {
            return Err(RuntimeError::Recovery {
                error: "run has no durable retry checkpoint".to_string(),
            });
        };

        // Integrity of the durable record itself.
        let observed_digest = *blake3::hash(durable_payload).as_bytes();
        if observed_digest != *durable_payload_digest {
            return Err(RuntimeError::Recovery {
                error: "durable retry checkpoint payload digest mismatch".to_string(),
            });
        }

        // The supplied checkpoint must decode to the exact durable checkpoint value.
        // Use the version-aware decoder to reject historical version 1 payloads
        // that lack the policy_pin field in RunAdmission.
        let durable_checkpoint = crate::run_checkpoint::decode_checkpoint_payload(durable_payload)
            .map_err(|error| RuntimeError::Recovery {
                error: format!("durable retry checkpoint decode failed: {error}"),
            })?;
        if durable_checkpoint != *checkpoint {
            return Err(RuntimeError::Recovery {
                error: "retry checkpoint does not match durable RetryCheckpoint evidence"
                    .to_string(),
            });
        }

        // run_attempt / step authority (also covered by the value equality above, but these
        // yield clearer diagnostics and double as a defense-in-depth binding).
        if *durable_run != run {
            return Err(RuntimeError::Recovery {
                error: "durable retry checkpoint belongs to a different run".to_string(),
            });
        }
        if checkpoint.run_attempt.get() != *durable_attempt {
            return Err(RuntimeError::Recovery {
                error: "retry checkpoint run attempt does not match durable RetryCheckpoint"
                    .to_string(),
            });
        }

        // prior_seq must be the sequence immediately before the durable checkpoint.
        let expected_checkpoint_seq =
            checkpoint
                .prior_seq
                .checked_add(1)
                .ok_or_else(|| RuntimeError::Recovery {
                    error: "retry checkpoint prior_seq overflow".to_string(),
                })?;
        if durable_seq.get() != expected_checkpoint_seq {
            return Err(RuntimeError::Recovery {
                error: "retry checkpoint prior_seq does not match durable RetryCheckpoint position"
                    .to_string(),
            });
        }

        // next_seq must be the sequence immediately after the durable checkpoint.
        let expected_next_seq =
            durable_seq
                .get()
                .checked_add(1)
                .ok_or_else(|| RuntimeError::Recovery {
                    error: "retry checkpoint next_seq overflow".to_string(),
                })?;
        if next_seq.get() != expected_next_seq {
            return Err(RuntimeError::Recovery {
                error: "retry next_seq does not match durable RetryCheckpoint position".to_string(),
            });
        }

        // The checkpoint must be the journal head. An explicit retry that already consumed
        // this checkpoint appends StepStarted / ActionScheduled / RunFinished AFTER it, so a
        // checkpoint that is no longer the last event is already-consumed and must be rejected
        // (a stale `next_seq` alone cannot be trusted to detect this).
        let checkpoint_is_head = events
            .last()
            .is_some_and(|event| event.seq().get() == durable_seq.get());
        if !checkpoint_is_head {
            return Err(RuntimeError::Recovery {
                error: "retry checkpoint has already been consumed (journal advanced past it)"
                    .to_string(),
            });
        }

        // The event immediately preceding the checkpoint must be the run's own RunFailed
        // event for the attempt immediately before the checkpoint's run_attempt.
        let expected_failed_attempt =
            checkpoint
                .run_attempt
                .get()
                .checked_sub(1)
                .ok_or_else(|| RuntimeError::Recovery {
                    error: "retry checkpoint run_attempt underflow".to_string(),
                })?;
        let preceding_failed = events.iter().find(|event| {
            event.seq().get() == checkpoint.prior_seq
                && matches!(
                    event,
                    vb_storage::JournalEvent::RunFailedEvent { run: r, .. } if *r == run
                )
        });
        match preceding_failed {
            Some(vb_storage::JournalEvent::RunFailedEvent { attempt, .. })
                if *attempt == expected_failed_attempt =>
            {
                Ok(())
            }
            _ => Err(RuntimeError::Recovery {
                error: "retry checkpoint does not follow the run's own durable RunFailed event"
                    .to_string(),
            }),
        }
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
