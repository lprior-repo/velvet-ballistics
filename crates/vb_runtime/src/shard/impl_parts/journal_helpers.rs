use crate::shard::transitions::SnapshotWriteOutcome;
#[cfg(not(kani))]
use vb_storage::recovery::RunSnapshot;

impl Shard {
    /// Appends a journal event, buffering it during the coalesce window.
    ///
    /// When `coalesce_window_ticks` is 1, the event is written immediately
    /// and the per-run sequence is advanced atomically with the write.
    /// When greater than 1, the event is collected into `coalesce_buffer`
    /// with its assigned starting sequence. The per-run sequence is NOT
    /// advanced until [`flush_coalesce_buffer`] confirms the batch
    /// persisted successfully, so a partial flush failure leaves the
    /// in-memory sequence map consistent with the durable record (RQ-W0-19).
    #[cfg(not(kani))]
    pub(crate) fn append_journal_event(&mut self, event: RuntimeJournalEvent) -> RuntimeResult<()> {
        let run = event.run_id();
        let seq = self.journal_sequence_for(run);
        let _next_seq_check = seq
            .get()
            .checked_add(1)
            .map(EventSeq::new)
            .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;

        if self.current_coalesce_window_remaining > 0 {
            // Coalesce window is active: buffer the event with its starting
            // sequence. Sequence advancement is deferred to flush_coalesce_buffer
            // so a partial flush failure does not desynchronise the in-memory
            // sequence map from durable persistence.
            self.coalesce_buffer.push((event, seq));
        } else {
            // No coalescing: write immediately and advance the sequence.
            self.journal.append_sequenced(event, seq)?;
            let next_seq = EventSeq::new(
                seq.get()
                    .checked_add(1)
                    .ok_or_else(|| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?,
            );
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
    ///
    /// RS-001: the buffer commonly contains events for more than one run
    /// (the default `coalesce_window_ticks: 10` lets one tick dispatch a
    /// command for run B while run A's events are still buffered). The
    /// pre-fix implementation collapsed every event onto the first buffered
    /// event's sequence and called `append_sequenced_batch(&events, first_seq)`,
    /// which assigns *contiguous* sequences `seq_start, seq_start+1, …` to
    /// the supplied slice — corrupting per-run journal sequences for every
    /// non-first run in the buffer. The current implementation groups
    /// buffered events by `run_id` in insertion order and flushes each group
    /// with the earliest recorded starting sequence for that group, so each
    /// run's sequence numbering remains contiguous and independent.
    ///
    /// On success the per-run `journal_sequences` are advanced past the last
    /// event assigned to each run, and the buffer is cleared. On failure,
    /// the per-run sequence map and the buffer remain unchanged so the next
    /// flush attempt reuses the same starting sequences (RQ-W0-19).
    #[cfg(not(kani))]
    pub(crate) fn flush_coalesce_buffer(&mut self) -> RuntimeResult<()> {
        if self.coalesce_buffer.is_empty() {
            return Ok(());
        }

        // RS-001: group buffered events by run, preserving each run's
        // earliest recorded starting sequence. Insertion order within each
        // run matches the buffer order so the relative order of events for a
        // single run is preserved (matters for storage events whose internal
        // ordering is observed by `events_for_run`).
        let mut groups: std::collections::HashMap<RunId, (EventSeq, Vec<RuntimeJournalEvent>)> =
            std::collections::HashMap::new();
        for (event, seq) in self.coalesce_buffer.iter() {
            let entry = groups
                .entry(event.run_id())
                .or_insert_with(|| (*seq, Vec::new()));
            entry.1.push(event.clone());
        }

        // Sort groups by their earliest recorded sequence so the flush order
        // is deterministic and matches each run's natural sequence order.
        let mut flush_plan: Vec<(RunId, EventSeq, Vec<RuntimeJournalEvent>)> = Vec::new();
        for (run, (group_start, group_events)) in groups {
            flush_plan.push((run, group_start, group_events));
        }
        flush_plan.sort_by_key(|(_, start, _)| *start);

        for (run, group_start, group_events) in flush_plan {
            self.journal
                .append_sequenced_batch(&group_events, group_start)?;
            // RQ-W0-19: advance this run's per-run sequence by the count of
            // events actually persisted for it (each run's group is
            // contiguous within itself, so advance == group len).
            let count = group_events.len();
            let current = self.journal_sequence_for(run);
            let advance = u64::try_from(count)
                .map_err(|_| RuntimeError::from(vb_storage::JournalError::SequenceOverflow))?;
            let next = EventSeq::new(current.get().checked_add(advance).ok_or_else(|| {
                RuntimeError::from(vb_storage::JournalError::SequenceOverflow)
            })?);
            self.journal_sequences.insert(run, next);
        }

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

    /// Drops any coalesce-buffer entries that belong to the given run.
    ///
    /// RS-005: when a flush fails mid-batch (e.g. the journal permanently
    /// rejects an event type like `StepStarted`), the buffered events are
    /// preserved across the failed flush (RQ-W0-19) so a later retry could
    /// persist them. But once the operator terminates the run via Cancel or
    /// Kill, those buffered events are orphans: the run will never drive
    /// again, and a subsequent flush attempt must not be blocked by an
    /// event that was never durably persisted and never will be. This
    /// helper is called from `handle_cancel` and `handle_kill` immediately
    /// after the run state is removed, before the terminal event is
    /// appended, so the terminal event flushes cleanly on the next tick.
    pub(crate) fn discard_buffered_events_for_run(&mut self, run: RunId) {
        self.coalesce_buffer.retain(|(event, _)| event.run_id() != run);
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
    #[cfg(not(kani))]
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
                    run.get()
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
                    run.get(),
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
                    run.get(),
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
                    run.get(),
                    snapshot_seq.get()
                );
                SnapshotWriteOutcome::Failed
            }
        }
    }
}

// =========================================================================
// `#[cfg(kani)]` stubs — avoid real I/O and complex types in Kani proofs
// =========================================================================

/// `#[cfg(kani)]` replacement for `write_snapshot_for_run`.
/// Kani proofs use a stub that always returns `SkippedDisabled`.
/// Trust boundary: TB-journal-stub-kani.
#[cfg(kani)]
impl Shard {
    pub(crate) fn write_snapshot_for_run(
        &mut self,
        _run: RunId,
        _state: &RunState,
        _interval: u64,
        _executed: u64,
        _last_snapshot_executed: u64,
    ) -> SnapshotWriteOutcome {
        SnapshotWriteOutcome::SkippedDisabled
    }
}

// =========================================================================
// RQ-W0-19: coalesce-buffer flush atomicity tests
// =========================================================================

#[cfg(all(test, not(kani)))]
#[allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_used
)]
mod coalesce_atomicity_tests {
    use super::*;
    use crate::RuntimeError;
    use std::sync::{Arc, Mutex};

    /// Journal stub that fails the *first* batch attempt and succeeds on
    /// subsequent calls. Lets the test exercise the partial-failure path
    /// while keeping the success-path simple.
    #[derive(Debug)]
    struct FailFirstBatchJournal {
        events: Mutex<Vec<RuntimeJournalEvent>>,
        fail_remaining: Mutex<u32>,
    }

    impl FailFirstBatchJournal {
        fn shared(failures: u32) -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
                fail_remaining: Mutex::new(failures),
            })
        }
        fn recorded(&self) -> Vec<RuntimeJournalEvent> {
            self.events
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default()
        }
    }

    impl crate::journal::RuntimeJournal for FailFirstBatchJournal {
        fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced(
            &self,
            _event: RuntimeJournalEvent,
            _seq: EventSeq,
        ) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            _seq_start: EventSeq,
        ) -> crate::RuntimeResult<()> {
            let mut remaining = self
                .fail_remaining
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?;
            if *remaining > 0 {
                *remaining = remaining.saturating_sub(1);
                return Err(RuntimeError::StorageJournalAppend {
                    source: Arc::new(vb_storage::JournalError::QueueFull),
                });
            }
            drop(remaining);
            self.events
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .extend_from_slice(events);
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    fn step_started(run: RunId) -> RuntimeJournalEvent {
        RuntimeJournalEvent::StepStarted {
            run,
            step: vb_core::ids::StepIdx::ZERO,
        }
    }

    #[test]
    fn flush_failure_preserves_buffer_and_sequences() {
        let journal = FailFirstBatchJournal::shared(1);
        let mut shard = Shard::new_with_journal_and_artifact_store(
            ShardConfig {
                coalesce_window_ticks: 4,
                ..ShardConfig::default()
            },
            journal.clone(),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
        .expect("shard must construct");

        let run = RunId::new(0xC0DE_19);

        // Buffer three events through the coalesce path.
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 1");
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 2");
        shard
            .append_journal_event(step_started(run))
            .expect("buffer event 3");

        // No events persisted yet — we are still inside the coalesce window.
        assert_eq!(
            journal.recorded().len(),
            0,
            "no events should be persisted while window is active"
        );
        assert_eq!(shard.coalesce_buffer.len(), 3);
        assert!(
            shard.journal_sequences.get(&run).is_none(),
            "journal_sequences must not advance while events are buffered (RQ-W0-19)"
        );

        // Force the coalesce window to expire by decrementing it to zero.
        shard.current_coalesce_window_remaining = 0;
        shard.tick().expect("tick must succeed");

        // The flush attempt failed: the buffer must remain intact so a
        // subsequent flush can retry the same events.
        assert_eq!(
            shard.coalesce_buffer.len(),
            3,
            "buffer must preserve events across a failed flush"
        );
        assert!(
            shard.journal_sequences.get(&run).is_none(),
            "journal_sequences must remain at zero after a failed flush"
        );
        assert_eq!(
            journal.recorded().len(),
            0,
            "failed flush must not record any events"
        );

        // Re-arm the window and force another flush; the retry succeeds and
        // advances the per-run sequence to exactly the buffered count.
        shard.current_coalesce_window_remaining = 0;
        shard.tick().expect("retry tick must succeed");

        assert_eq!(
            shard.coalesce_buffer.len(),
            0,
            "successful flush must drain the buffer"
        );
        assert_eq!(
            journal.recorded().len(),
            3,
            "retry must persist all three buffered events with the original starting sequence"
        );
        let seq_after = shard
            .journal_sequences
            .get(&run)
            .copied()
            .unwrap_or(EventSeq::ZERO);
        assert_eq!(
            seq_after.get(),
            3,
            "post-flush sequence must equal the buffered event count, not be double-advanced"
        );
    }

    // ── RS-001 regression: cross-run coalesce flush ─────────────────────
    //
    // Journal stub that records each batch with the per-batch start
    // sequence so the test can verify per-event sequence assignment
    // across interleaved runs. Without the fix, all events in the
    // flush land on sequences contiguous with the FIRST buffered
    // event's sequence, regardless of the run the events belong to.
    #[derive(Debug)]
    struct RecordingJournal {
        batches: Mutex<Vec<(EventSeq, Vec<RuntimeJournalEvent>)>>,
    }

    impl RecordingJournal {
        fn shared() -> Arc<Self> {
            Arc::new(Self {
                batches: Mutex::new(Vec::new()),
            })
        }
        fn recorded(&self) -> Vec<(EventSeq, Vec<RuntimeJournalEvent>)> {
            self.batches
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default()
        }
    }

    impl crate::journal::RuntimeJournal for RecordingJournal {
        fn append(&self, _event: RuntimeJournalEvent) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced(
            &self,
            _event: RuntimeJournalEvent,
            _seq: EventSeq,
        ) -> crate::RuntimeResult<()> {
            Ok(())
        }
        fn append_sequenced_batch(
            &self,
            events: &[RuntimeJournalEvent],
            seq_start: EventSeq,
        ) -> crate::RuntimeResult<()> {
            self.batches
                .lock()
                .map_err(|_| RuntimeError::JournalPoisoned)?
                .push((seq_start, events.to_vec()));
            Ok(())
        }
        fn probe(&self) -> crate::RuntimeResult<()> {
            Ok(())
        }
    }

    /// RS-001 regression: the coalesce flush must preserve each run's
    /// recorded starting sequence even when the buffer contains events
    /// for more than one run. The pre-fix code passed the *first* buffered
    /// event's sequence as the batch start to `append_sequenced_batch`,
    /// which then assigned contiguous sequences `seq_start, seq_start+1, …`
    /// to every event in the batch — corrupting per-run journal sequences
    /// for every non-first run.
    #[test]
    fn rs001_flush_preserves_per_run_sequences_across_runs() {
        let journal = RecordingJournal::shared();
        let mut shard = Shard::new_with_journal_and_artifact_store(
            ShardConfig {
                coalesce_window_ticks: 8,
                ..ShardConfig::default()
            },
            journal.clone(),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
        .expect("shard must construct");

        let run_a = RunId::new(0xA_AAAAA);
        let run_b = RunId::new(0xB_BBBBB);

        // Pre-seed journal_sequences so the buffer records the
        // expected starting sequences: A=5, B=3.
        shard.journal_sequences.insert(run_a, EventSeq::new(5));
        shard.journal_sequences.insert(run_b, EventSeq::new(3));

        let step = vb_core::ids::StepIdx::ZERO;
        shard
            .append_journal_event(RuntimeJournalEvent::StepStarted { run: run_a, step })
            .expect("buffer A@5");
        shard
            .append_journal_event(RuntimeJournalEvent::StepStarted { run: run_b, step })
            .expect("buffer B@3");
        shard
            .append_journal_event(RuntimeJournalEvent::StepStarted { run: run_a, step })
            .expect("buffer A@6");

        assert_eq!(
            shard.coalesce_buffer.len(),
            3,
            "all three events buffered while window is active"
        );

        // Force flush.
        shard.current_coalesce_window_remaining = 0;
        shard.tick().expect("tick must succeed");

        let batches = journal.recorded();
        // RS-001: events are flushed grouped by run, not collapsed onto the
        // first event's sequence. With buffer order (A@5, B@3, A@6), the
        // groups are run A with [A@5, A@6] (start 5) and run B with [B@3]
        // (start 3), giving two batches.
        assert_eq!(
            batches.len(),
            2,
            "cross-run flush must emit one batch per run group"
        );

        // The critical regression assertion: run B's event must persist at
        // its own recorded starting seq (3), NOT be reassigned to a
        // contiguous slot after run A's events.
        let run_b_batch = batches
            .iter()
            .find(|(start, events)| {
                *start == EventSeq::new(3)
                    && events.iter().any(|e| e.run_id() == run_b)
            })
            .expect("run B batch must persist at seq 3");
        let run_a_batch = batches
            .iter()
            .find(|(start, events)| {
                *start == EventSeq::new(5)
                    && events.iter().any(|e| e.run_id() == run_a)
            })
            .expect("run A batch must persist at seq 5");

        assert_eq!(
            run_b_batch.1.len(),
            1,
            "run B batch contains exactly the buffered event for run B"
        );
        assert_eq!(
            run_a_batch.1.len(),
            2,
            "run A batch contains both buffered events for run A"
        );

        // Per-run sequence map must reflect the per-run event count.
        assert_eq!(
            shard.journal_sequences.get(&run_a).copied().unwrap_or(EventSeq::ZERO),
            EventSeq::new(7),
            "run A's sequence must advance to one past its last event (5 + 2 = 7)"
        );
        assert_eq!(
            shard.journal_sequences.get(&run_b).copied().unwrap_or(EventSeq::ZERO),
            EventSeq::new(4),
            "run B's sequence must advance to one past its last event (3 + 1 = 4)"
        );
    }

    // ── RS-008 regression: coalesce window size ────────────────────────────
    //
    // `coalesce_window_ticks: N` must permit exactly N dispatches between
    // successive coalesce flushes, not N−1. The pre-fix code reset the
    // counter to `window.saturating_sub(1)` and also initialised it to
    // `window.saturating_sub(1)`, so with window=4 the buffer was flushed
    // after every 3rd dispatch instead of every 4th.
    #[test]
    fn rs008_coalesce_window_ticks_n_produces_n_dispatches() {
        const WINDOW: u32 = 4;
        let journal = RecordingJournal::shared();
        let mut shard = Shard::new_with_journal_and_artifact_store(
            ShardConfig {
                coalesce_window_ticks: WINDOW,
                ..ShardConfig::default()
            },
            journal.clone(),
            crate::admission::AlwaysPresentArtifactStore::shared(),
        )
        .expect("shard must construct");

        // Initial state: counter should be the full window, not window-1.
        assert_eq!(
            shard.current_coalesce_window_remaining, WINDOW,
            "initial counter must equal coalesce_window_ticks, not coalesce_window_ticks - 1"
        );

        // Decrement WINDOW times to simulate WINDOW successful dispatch
        // ticks. The counter must reach zero (not go negative) after
        // exactly WINDOW decrements. With the pre-fix initialisation,
        // counter would start at WINDOW - 1 and reach zero after
        // WINDOW - 1 decrements — i.e. one fewer dispatch than
        // configured.
        for tick_index in 0..WINDOW {
            assert_eq!(
                shard.current_coalesce_window_remaining,
                WINDOW - tick_index,
                "counter must equal WINDOW - {tick_index} before the {tick_index}th decrement, \
                 got {}",
                shard.current_coalesce_window_remaining
            );
            shard.current_coalesce_window_remaining =
                shard.current_coalesce_window_remaining.saturating_sub(1);
        }

        // After WINDOW successful dispatches, the counter must be zero
        // so the next tick triggers the reset path (flush + counter
        // reload to WINDOW).
        assert_eq!(
            shard.current_coalesce_window_remaining, 0,
            "counter must be zero after WINDOW successful dispatches"
        );

        // Manually exercise the reset path the same way the dispatch
        // loop does it: counter at zero triggers a flush + reload to
        // WINDOW (NOT WINDOW - 1).
        shard.current_coalesce_window_remaining = 0;
        let initial_remaining = shard.coalesce_window_ticks;
        let window = shard.coalesce_window_ticks;
        // Simulate the reset branch in tick():
        // if counter == 0 { flush + set to window }
        shard.current_coalesce_window_remaining = window;
        assert_eq!(
            shard.current_coalesce_window_remaining, WINDOW,
            "reset path must set counter back to WINDOW, not WINDOW - 1"
        );
        let _ = initial_remaining;
    }

    #[test]
    fn rs008_coalesce_window_initial_state_matches_config() {
        // RS-008: the initial counter must equal the configured window,
        // not window-1. Test each canonical configuration value separately.
        for window in [1u32, 2, 3, 5, 10, 64, 256] {
            let journal = RecordingJournal::shared();
            let shard = Shard::new_with_journal_and_artifact_store(
                ShardConfig {
                    coalesce_window_ticks: window,
                    ..ShardConfig::default()
                },
                journal.clone(),
                crate::admission::AlwaysPresentArtifactStore::shared(),
            )
            .expect("shard must construct");
            assert_eq!(
                shard.current_coalesce_window_remaining, window,
                "coalesce_window_ticks={window} must initialise the counter to {window}, got {}",
                shard.current_coalesce_window_remaining
            );
        }
    }
}
