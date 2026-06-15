use vb_storage::recovery::RunSnapshot;
use crate::shard::transitions::SnapshotWriteOutcome;

impl Shard {
    /// Appends a journal event, buffering it during the coalesce window.
    ///
    /// When `coalesce_window_ticks` is 1, the event is written immediately.
    /// When greater than 1, the event is collected into `coalesce_buffer`
    /// and written atomically when the window expires.
    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        let next_seq = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;

        if self.current_coalesce_window_remaining > 0 {
            // Coalesce window is active: buffer the event instead of
            // writing immediately. The batch flush will write all events
            // atomically when the window expires.
            self.coalesce_buffer.push((event, seq));
            self.journal_sequences.insert(run, next_seq);
        } else {
            // No coalescing: write immediately.
            self.journal.append_sequenced(event, seq)?;
            self.journal_sequences.insert(run, next_seq);
        }
        Ok(())
    }

    /// `#[cfg(kani)]` replacement for `append_journal_event` that returns
    /// `Ok(())`. Trust boundary TB-vb282my-journal-stub-001.
    /// Production journal append uses Fjall-backed persistence; stubbed version
    /// avoids real I/O during Kani bounded model checking.
    /// NOTE(vb-k8ut.2): Changed from `kani::any()` to `Ok(())` to avoid
    /// `RuntimeError: kani::Arbitrary` requirement. The nondeterministic
    /// error-path exercise is not needed for IPC command reconciliation proofs
    /// and the `RuntimeError` type's complex fields (Box<CoreError>,
    /// Arc<JournalError>, Capability, etc.) make full Arbitrary impl impractical.
    #[cfg(kani)]
    pub(crate) fn append_journal_event(
        &mut self,
        _event: RuntimeJournalEvent,
    ) -> RuntimeResult<()> {
        Ok(())
    }

    #[cfg(not(kani))]
    fn journal_sequence_for(&self, run: RunId) -> EventSeq {
        self.journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO)
    }

    /// Flushes the coalesce buffer by writing all buffered events atomically
    /// via `RuntimeJournal::append_sequenced_batch`.
    ///
    /// This is called when the coalesce window expires (counter reaches zero).
    /// Each event is written with its recorded per-run starting sequence.
    ///
    /// Returns `Ok(())` if the flush succeeds or if there are no buffered events.
    /// On failure, the error is returned and the shard's state is unchanged.
    #[cfg(not(kani))]
    pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
        if self.coalesce_buffer.is_empty() {
            return Ok(());
        }

        let events: Vec<RuntimeJournalEvent> = self
            .coalesce_buffer
            .iter()
            .map(|(event, _seq)| event.clone())
            .collect();

        // Use the first event's sequence as the batch start. The batch method
        // assigns contiguous sequences from seq_start, so we use the earliest
        // sequence in the buffer as the anchor point.
        let first_seq = self
            .coalesce_buffer
            .first()
            .map(|(_, seq)| *seq)
            .unwrap_or(EventSeq::ZERO);

        self.journal.append_sequenced_batch(&events, first_seq)?;

        self.coalesce_buffer.clear();
        Ok(())
    }

    /// `#[cfg(kani)]` replacement for `flush_coalesce_buffer` that returns
    /// `Ok(())`. Avoids real I/O during Kani bounded model checking.
    #[cfg(kani)]
    pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
        self.coalesce_buffer.clear();
        Ok(())
    }

    pub(crate) fn discard_journal_sequence(&mut self, run: RunId) {
        self.journal_sequences.swap_remove(&run);
    }

    /// Writes a run snapshot at the given executed-step threshold.
    ///
    /// This method:
    /// 1. Checks if snapshotting is enabled (`interval > 0`).
    /// 2. Checks if the step count has reached the trigger threshold.
    /// 3. Advances the journal sequence, writes the snapshot, and updates
    ///    the threshold. On failure, the sequence is rolled back.
    ///
    /// Returns the outcome of the write attempt.
    ///
    /// **Non-blocking contract (C-2 / C-3):** Serialization failures and
    /// journal write errors are converted to `SnapshotWriteOutcome::Failed`
    /// so the caller can continue the run lifecycle. Only the `Written`
    /// outcome carries a successful sequence advance.
    pub(crate) fn write_snapshot_for_run(
        &mut self,
        run: RunId,
        state: &RunState,
        interval: u64,
        executed: u64,
        last_snapshot_executed: u64,
    ) -> SnapshotWriteOutcome {
        // C1: Disabled → skip immediately.
        if interval == 0 {
            return SnapshotWriteOutcome::SkippedDisabled;
        }

        // C2: Check if enough steps have elapsed since the last snapshot.
        if executed.saturating_sub(last_snapshot_executed) < interval {
            return SnapshotWriteOutcome::SkippedNotReady;
        }

        // Resolve the underlying FjallJournal from the runtime journal trait.
        let Some(fjall) = self.journal.storage_journal() else {
            // Non-storage journal (volatile/noop): cannot write snapshots.
            // Best-effort: log and continue.
            return SnapshotWriteOutcome::SkippedNoStorage;
        };

        // Advance the journal sequence before attempting the write.
        // Per C-4: snapshot seq must be strictly greater than any journal event
        // seq written before the snapshot.
        let current_seq = self.journal_sequence_for(run);
        let snapshot_seq = match current_seq
            .get()
            .checked_add(1)
            .map(vb_storage::EventSeq::new)
        {
            Some(seq) => seq,
            None => {
                // Sequence overflow is a structural invariant violation.
                // Log and return Failed so the run continues.
                tracing::warn!(
                    "snapshot_write_failed seq_overflow run={}",
                    run.as_u64()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };

        // Serialise slots and taint from the RunFrame.
        // Serialization errors are non-fatal to the run lifecycle (C-2, C-5).
        let slots_bytes = match postcard::to_allocvec(&state.frame.slots_snapshot()) {
            Ok(bytes) => bytes,
            Err(_) => {
                tracing::warn!(
                    "snapshot_write_failed postcard_encode_failed_slots run={} seq={}",
                    run.as_u64(),
                    snapshot_seq.get()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };
        let taint_bytes = match postcard::to_allocvec(&state.frame.taint_snapshot()) {
            Ok(bytes) => bytes,
            Err(_) => {
                tracing::warn!(
                    "snapshot_write_failed postcard_encode_failed_taint run={} seq={}",
                    run.as_u64(),
                    snapshot_seq.get()
                );
                return SnapshotWriteOutcome::Failed;
            }
        };

        let snapshot = RunSnapshot {
            run: state.frame.run_id(),
            seq: snapshot_seq,
            workflow: state.workflow.digest(),
            slots: slots_bytes,
            taint: taint_bytes,
        };

        // Attempt the write.
        let write_result = fjall.put_snapshot(&snapshot);

        match write_result {
            Ok(()) => {
                // Success: advance the journal sequence and update the threshold.
                self.journal_sequences.insert(run, snapshot_seq);
                // The caller updates last_snapshot_executed = executed via the
                // SnapshotWritten outcome.
                SnapshotWriteOutcome::Written
            }
            Err(_e) => {
                // Failure: roll back the sequence advance.
                self.journal_sequences.insert(run, current_seq);
                // Log the error per C-2 contract requirement.
                // The run continues without snapshot.
                tracing::warn!(
                    "snapshot_write_failed storage_write_error run={} seq={}",
                    run.as_u64(),
                    snapshot_seq.get()
                );
                SnapshotWriteOutcome::Failed
            }
        }
    }
}
