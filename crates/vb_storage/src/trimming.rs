#![forbid(unsafe_code)]
//! Journal trimming with retention policy.
//!
//! Periodically trims old journal events to reduce storage while preserving
//! events needed for recovery. Events older than a confirmed snapshot are
//! eligible for trimming.

use vb_core::RunId;

use crate::{EventSeq, FjallJournal, JournalError, JournalEvent};

use fjall::Readable;

/// Retention policy for journal trimming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimPolicy {
    /// If true, skip runs that have no events to trim (no-op runs).
    pub skip_noop_runs: bool,
    /// Number of most-recent terminal runs per workflow to retain.
    /// A run is eligible for trimming only if it is NOT among the
    /// `retain_last_n_terminal` most recent terminal runs for its workflow.
    pub retain_last_n_terminal: u32,
}

impl Default for TrimPolicy {
    fn default() -> Self {
        Self {
            skip_noop_runs: true,
            retain_last_n_terminal: 10,
        }
    }
}

/// Errors that can occur during journal trimming.
#[derive(Debug, thiserror::Error)]
pub enum TrimError {
    /// Fjall operation failed.
    #[error("fjall operation failed: {0}")]
    Fjall(#[from] fjall::Error),
    /// Journal operation failed.
    #[error("journal error: {0}")]
    Journal(#[from] JournalError),
    /// No durable snapshot found for run.
    #[error("no durable snapshot for run {run:?}")]
    NoDurableSnapshot {
        /// Run without a durable snapshot.
        run: RunId,
    },
    /// Retention policy blocks trimming this terminal run.
    #[error("retention policy blocks trim for run {run:?}")]
    RetentionPolicyBlocks {
        /// Run blocked by retention policy.
        run: RunId,
    },
    /// Trim operation was interrupted.
    #[error("trim operation incomplete")]
    IncompleteTrim {
        /// Number of events deleted before interruption.
        deleted_count: u64,
    },
}

impl TrimError {
    pub const NO_DURABLE_SNAPSHOT_CODE: vb_core::DiagnosticCode =
        vb_core::DiagnosticCode::new(0x4101);
    pub const RETENTION_POLICY_BLOCKS_CODE: vb_core::DiagnosticCode =
        vb_core::DiagnosticCode::new(0x4103);
    pub const INCOMPLETE_TRIM_CODE: vb_core::DiagnosticCode = vb_core::DiagnosticCode::new(0x4102);

    #[must_use]
    pub const fn diagnostic_code(&self) -> vb_core::DiagnosticCode {
        match self {
            Self::Fjall(_) => JournalError::FJALL_CODE,
            Self::Journal(_) => JournalError::FJALL_CODE,
            Self::NoDurableSnapshot { .. } => Self::NO_DURABLE_SNAPSHOT_CODE,
            Self::RetentionPolicyBlocks { .. } => Self::RETENTION_POLICY_BLOCKS_CODE,
            Self::IncompleteTrim { .. } => Self::INCOMPLETE_TRIM_CODE,
        }
    }
}

/// Result type for trim operations.
pub type TrimResult<T> = Result<T, TrimError>;

impl FjallJournal {
    /// Returns the latest durable snapshot sequence for a run.
    ///
    /// Scans all snapshots for the given run and returns the one with the
    /// highest sequence number. Returns `None` if no snapshot exists.
    pub fn latest_durable_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>> {
        let prefix_key = snapshot_prefix_key(run);
        let mut latest: Option<EventSeq> = None;

        for item in self.run_snapshot.prefix(prefix_key) {
            let key = item.key().map_err(TrimError::from)?;
            if key.len() < 17 {
                continue;
            }
            let slice = key
                .get(9..17)
                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_u64 = u64::from_be_bytes(seq_bytes);
            let seq = EventSeq::new(seq_u64);
            latest = Some(match latest {
                Some(current) if current.get() >= seq_u64 => current,
                _ => seq,
            });
        }

        Ok(latest)
    }

    /// Trims journal events for a specific run.
    ///
    /// Removes events with sequence numbers less than the latest durable
    /// snapshot sequence. If no durable snapshot exists for the run, returns an error.
    ///
    /// Trimming is idempotent: subsequent trims of an already-trimmed run
    /// are a no-op.
    pub fn trim_events_for_run(
        &self,
        run: RunId,
        policy: TrimPolicy,
    ) -> TrimResult<TrimmedRunResult> {
        let Some(cutoff_seq) = self.latest_durable_snapshot_seq(run)? else {
            return Err(TrimError::NoDurableSnapshot { run });
        };

        self.check_retention_policy(run, &policy)?;

        let prefix_key = crate::keys::run_prefix_key(run)?;
        let mut batch = self.database.batch();
        let mut deleted_count: u64 = 0;

        for item in self.events.prefix(prefix_key) {
            let key = item.key().map_err(TrimError::from)?;
            if key.len() < 17 {
                continue;
            }
            let slice = key
                .get(9..17)
                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_u64 = u64::from_be_bytes(seq_bytes);

            if seq_u64 < cutoff_seq.get() {
                batch.remove(&self.events, key.to_vec());
                deleted_count = deleted_count.saturating_add(1);
            }
        }

        if deleted_count == 0 && policy.skip_noop_runs {
            return Ok(TrimmedRunResult {
                run,
                deleted_count: 0,
                cutoff_seq,
                status: TrimStatus::NoOp,
            });
        }

        batch.commit()?;

        Ok(TrimmedRunResult {
            run,
            deleted_count,
            cutoff_seq,
            status: TrimStatus::Trimmed,
        })
    }

    /// Trims journal events for all runs with durable snapshots.
    ///
    /// Iterates over all run headers and trims events for each run that
    /// has a durable snapshot. Runs without durable snapshots or runs
    /// blocked by retention policy are skipped.
    pub fn trim_all_eligible_runs(&self, policy: TrimPolicy) -> TrimResult<Vec<TrimmedRunResult>> {
        let headers = self.run_headers()?;
        let mut results = Vec::new();

        for header in headers {
            match self.trim_events_for_run(header.run, policy) {
                Ok(result) => results.push(result),
                Err(TrimError::NoDurableSnapshot { .. }) => continue,
                Err(TrimError::RetentionPolicyBlocks { .. }) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(results)
    }
    /// Non-destructive trim eligibility diagnostic.
    ///
    /// Scans all runs and reports eligibility WITHOUT deleting anything.
    /// Safe to run during incident triage.
    pub fn trim_eligibility_diagnostic(
        &self,
        policy: TrimPolicy,
    ) -> Result<TrimDiagnostic, JournalError> {
        let headers = self.run_headers()?;
        let mut runs = Vec::new();
        let mut total_runs: u64 = 0;
        let mut eligible_runs: u64 = 0;
        let mut blocked_runs: u64 = 0;
        let mut total_events_trimmable: u64 = 0;

        for header in headers {
            total_runs = total_runs.saturating_add(1);

            let safe_point = match self.latest_durable_snapshot_seq(header.run) {
                Ok(Some(seq)) => seq,
                Ok(None) => {
                    blocked_runs = blocked_runs.saturating_add(1);
                    runs.push(TrimEligibility::Blocked {
                        run: header.run,
                        blocker: TrimBlocker::NoDurableSnapshot,
                    });
                    continue;
                }
                Err(e) => {
                    // Convert TrimError to JournalError for the public API
                    return Err(JournalError::from(e));
                }
            };

            // Check retention policy (reuses internal logic)
            match self.check_retention_policy(header.run, &policy) {
                Ok(()) => {}
                Err(TrimError::RetentionPolicyBlocks { .. }) => {
                    blocked_runs = blocked_runs.saturating_add(1);
                    runs.push(TrimEligibility::Blocked {
                        run: header.run,
                        blocker: TrimBlocker::RetentionPolicy {
                            retain_last_n_terminal: policy.retain_last_n_terminal,
                        },
                    });
                    continue;
                }
                Err(e) => return Err(JournalError::from(e)),
            }

            // Count trimmable events for this run
            let events_trimmable = self.count_trimmable_events(header.run, safe_point)?;

            eligible_runs = eligible_runs.saturating_add(1);
            total_events_trimmable = total_events_trimmable.saturating_add(events_trimmable);

            runs.push(TrimEligibility::Eligible {
                run: header.run,
                safe_point,
                events_trimmable,
            });
        }

        Ok(TrimDiagnostic {
            runs,
            total_runs,
            eligible_runs,
            blocked_runs,
            total_events_trimmable,
        })
    }

    /// Counts events with sequence numbers less than the safe point.
    fn count_trimmable_events(
        &self,
        run: RunId,
        safe_point: EventSeq,
    ) -> Result<u64, JournalError> {
        let prefix_key = crate::keys::run_prefix_key(run)?;
        let snap = self.database.snapshot();
        let mut count: u64 = 0;

        for item in snap.prefix(&self.events, prefix_key) {
            let key = item.key().map_err(JournalError::from)?;
            if key.len() < 17 {
                continue;
            }
            let slice = key.get(9..17).ok_or_else(|| {
                JournalError::from(TrimError::IncompleteTrim { deleted_count: 0 })
            })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| JournalError::from(TrimError::IncompleteTrim { deleted_count: 0 }))?;
            let seq_u64 = u64::from_be_bytes(seq_bytes);

            if seq_u64 < safe_point.get() {
                count = count.saturating_add(1);
            }
        }

        Ok(count)
    }

    /// Checks whether a run has reached a terminal state by scanning its
    /// journal events for a terminal event variant.
    pub(crate) fn has_terminal_event(&self, run: RunId) -> TrimResult<bool> {
        let prefix = crate::keys::run_prefix_key(run)?;
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, prefix) {
            let value = item.value().map_err(TrimError::from)?;
            let (_, event) = crate::codec::decode_record::<JournalEvent>(
                value.as_ref(),
                crate::constants::MAGIC_JOURNAL_EVENT,
                crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )
            .map_err(TrimError::from)?;
            if crate::recovery::replay::core::is_terminal_event(&event) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Verifies retention policy for terminal runs.
    ///
    /// If the run is terminal and is among the `retain_last_n_terminal`
    /// most recent terminal runs for its workflow, returns
    /// `TrimError::RetentionPolicyBlocks`.
    pub(crate) fn check_retention_policy(&self, run: RunId, policy: &TrimPolicy) -> TrimResult<()> {
        if policy.retain_last_n_terminal == 0 {
            return Ok(());
        }

        if !self.has_terminal_event(run)? {
            return Ok(());
        }

        let Some(header) = self.run_header(run).map_err(TrimError::from)? else {
            return Ok(());
        };

        let all_headers = self.run_headers().map_err(TrimError::from)?;
        let mut terminal_runs: Vec<(RunId, u64)> = Vec::new();

        for h in all_headers {
            if h.workflow_id != header.workflow_id {
                continue;
            }
            if self.has_terminal_event(h.run)? {
                terminal_runs.push((h.run, h.accepted_at_ms));
            }
        }

        terminal_runs.sort_by_key(|(_, ts)| std::cmp::Reverse(*ts));

        let Some(position) = terminal_runs.iter().position(|(r, _)| *r == run) else {
            return Ok(());
        };

        let retain_count = match usize::try_from(policy.retain_last_n_terminal) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        };
        if position < retain_count {
            return Err(TrimError::RetentionPolicyBlocks { run });
        }

        Ok(())
    }
}

/// Result of trimming events for a single run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimmedRunResult {
    /// The run that was trimmed.
    pub run: RunId,
    /// Number of events deleted.
    pub deleted_count: u64,
    /// The snapshot sequence that served as the cutoff.
    pub cutoff_seq: EventSeq,
    /// Outcome status of the trim operation.
    pub status: TrimStatus,
}

/// Status of a trim operation for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimStatus {
    /// Events were deleted.
    Trimmed,
    /// No events were eligible for deletion.
    NoOp,
}

/// Per-run trim eligibility status (non-destructive diagnostic).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrimEligibility {
    /// Run can be trimmed up to the safe point.
    Eligible {
        /// The run identifier.
        run: RunId,
        /// Highest event sequence covered by a durable snapshot.
        safe_point: EventSeq,
        /// Number of events that would be deleted if trimmed.
        events_trimmable: u64,
    },
    /// Run cannot be trimmed due to a blocker.
    Blocked {
        /// The run identifier.
        run: RunId,
        /// The reason trimming is blocked.
        blocker: TrimBlocker,
    },
}

/// Reason a run cannot be trimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrimBlocker {
    /// No durable snapshot exists for this run.
    NoDurableSnapshot,
    /// Retention policy protects this terminal run.
    RetentionPolicy {
        /// The retention count that blocked this run.
        retain_last_n_terminal: u32,
    },
}

/// Aggregate trim diagnostic for all runs in the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrimDiagnostic {
    /// Per-run eligibility results.
    pub runs: Vec<TrimEligibility>,
    /// Total number of runs in the journal.
    pub total_runs: u64,
    /// Number of runs eligible for trimming.
    pub eligible_runs: u64,
    /// Number of runs blocked from trimming.
    pub blocked_runs: u64,
    /// Total events that would be deleted if all eligible runs were trimmed.
    pub total_events_trimmable: u64,
}

fn snapshot_prefix_key(run: RunId) -> [u8; 9] {
    let prefix: [u8; 1] = [crate::constants::PREFIX_RUN_SNAPSHOT];
    let run_be: [u8; 8] = run.get().to_be_bytes();
    let mut key = [0u8; 9];
    let mut pos = 0usize;
    for &byte in prefix.iter().chain(run_be.iter()) {
        if let Some(slot) = key.get_mut(pos) {
            *slot = byte;
        }
        pos = pos.saturating_add(1);
    }
    key
}

#[cfg(test)]
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
mod tests {
    use super::*;
    use crate::{EventSeq, JournalEvent, RunHeaderRecord, RunSnapshot, constants::DIGEST_BYTES};
    use vb_core::{RunId, SlotIdx, StepIdx, WorkflowDigest, WorkflowId};

    fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
        let temp = tempfile::tempdir().expect("tempdir creation should succeed");
        let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
        (temp, journal)
    }

    fn make_event(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunAccepted {
            run,
            seq: EventSeq::new(seq),
            workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
        }
    }

    fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
        JournalEvent::StepStarted {
            run,
            seq: EventSeq::new(seq),
            step: StepIdx::new(step),
        }
    }

    fn make_run_finished(run: RunId, seq: u64) -> JournalEvent {
        JournalEvent::RunFinished {
            run,
            seq: EventSeq::new(seq),
            result: SlotIdx::new(0),
        }
    }

    fn write_header(journal: &FjallJournal, run: RunId, digest: WorkflowDigest) {
        let header = RunHeaderRecord {
            run,
            workflow_id: WorkflowId::new(0),
            compiled_digest: digest,
            status: 0,
            accepted_at_ms: 0,
        };
        journal
            .put_run_header(&header)
            .expect("header write should succeed");
    }

    fn write_header_with_workflow(
        journal: &FjallJournal,
        run: RunId,
        workflow_id: WorkflowId,
        digest: WorkflowDigest,
        accepted_at_ms: u64,
    ) {
        let header = RunHeaderRecord {
            run,
            workflow_id,
            compiled_digest: digest,
            status: 0,
            accepted_at_ms,
        };
        journal
            .put_run_header(&header)
            .expect("header write should succeed");
    }

    #[test]
    fn trim_given_run_with_events_seq_0_to_9_and_snapshot_at_seq_5_trims_0_to_4() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(100);
        let digest = WorkflowDigest::from_bytes([0xAB; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..10u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(5),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        let result = journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("trim should succeed");

        assert_eq!(result.run, run);
        assert_eq!(result.deleted_count, 5, "should delete events 0-4");
        assert_eq!(result.cutoff_seq, EventSeq::new(5));
        assert_eq!(result.status, TrimStatus::Trimmed);

        let remaining = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(remaining.len(), 5, "should preserve events 5-9");
        for event in &remaining {
            assert!(
                event.seq().get() >= 5,
                "event seq {} should be >= 5",
                event.seq().get()
            );
        }
    }

    #[test]
    fn trim_given_run_already_trimmed_is_noop() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(200);
        let digest = WorkflowDigest::from_bytes([0xCD; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..6u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(5),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        let result1 = journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("first trim should succeed");
        assert_eq!(result1.deleted_count, 5);
        assert_eq!(result1.status, TrimStatus::Trimmed);

        let result2 = journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("second trim should succeed");
        assert_eq!(result2.deleted_count, 0);
        assert_eq!(result2.status, TrimStatus::NoOp);
    }

    #[test]
    fn trim_given_run_with_no_snapshot_returns_error() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(300);

        let events = [make_event(run, 0), make_step_started(run, 1, 0)];
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");

        let result = journal.trim_events_for_run(run, TrimPolicy::default());
        assert!(
            matches!(result, Err(TrimError::NoDurableSnapshot { .. })),
            "should error when no durable snapshot exists, got {:?}",
            result
        );
    }

    #[test]
    fn trim_preserves_run_header_and_snapshot() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(400);
        let digest = WorkflowDigest::from_bytes([0xEF; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..3u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header(&journal, run, digest);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(2),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("trim should succeed");

        let header = journal
            .run_header(run)
            .expect("header lookup should succeed");
        assert!(header.is_some(), "run header should be preserved");

        let snap = journal
            .snapshot(run, EventSeq::new(2))
            .expect("snapshot lookup should succeed");
        assert!(snap.is_some(), "snapshot should be preserved");
    }

    #[test]
    fn trim_all_eligible_runs_skips_runs_without_snapshots() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(500);
        let run_b = RunId::new(600);
        let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

        let events_a = [make_event(run_a, 0), make_step_started(run_a, 1, 0)];
        journal
            .append_strict_batch(&events_a)
            .expect("batch A should succeed");
        write_header(&journal, run_a, digest);

        let events_b = [make_event(run_b, 0), make_step_started(run_b, 1, 0)];
        journal
            .append_strict_batch(&events_b)
            .expect("batch B should succeed");
        write_header(&journal, run_b, digest);

        let snapshot_a = RunSnapshot {
            run: run_a,
            seq: EventSeq::new(1),
            workflow: digest,
            slots: vec![],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot_a)
            .expect("snapshot A should succeed");

        let results = journal
            .trim_all_eligible_runs(TrimPolicy::default())
            .expect("trim_all should succeed");

        assert_eq!(results.len(), 1, "only run A should be trimmed");
        assert_eq!(results[0].run, run_a);
        assert_eq!(results[0].deleted_count, 1);

        let remaining_a = journal
            .events_for_run(run_a)
            .expect("replay A should succeed");
        assert_eq!(remaining_a.len(), 1);

        let remaining_b = journal
            .events_for_run(run_b)
            .expect("replay B should succeed");
        assert_eq!(remaining_b.len(), 2, "run B should be untouched");
    }

    #[test]
    fn latest_durable_snapshot_seq_returns_highest_seq() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(700);
        let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);

        let snapshots = [
            RunSnapshot {
                run,
                seq: EventSeq::new(3),
                workflow: digest,
                slots: vec![],
                taint: vec![],
            },
            RunSnapshot {
                run,
                seq: EventSeq::new(1),
                workflow: digest,
                slots: vec![],
                taint: vec![],
            },
            RunSnapshot {
                run,
                seq: EventSeq::new(5),
                workflow: digest,
                slots: vec![],
                taint: vec![],
            },
        ];

        for snap in &snapshots {
            journal.put_snapshot(snap).expect("snapshot should succeed");
        }

        let latest = journal
            .latest_durable_snapshot_seq(run)
            .expect("should succeed");
        assert_eq!(latest, Some(EventSeq::new(5)), "latest should be seq 5");
    }

    #[test]
    fn latest_durable_snapshot_seq_returns_none_for_no_snapshots() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(800);

        let latest = journal
            .latest_durable_snapshot_seq(run)
            .expect("should succeed");
        assert_eq!(latest, None);
    }
    #[test]
    fn trim_preserves_events_at_or_after_snapshot() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(900);
        let digest = WorkflowDigest::from_bytes([0xEF; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..4u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(2),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        let result = journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("trim should succeed");
        assert_eq!(result.deleted_count, 2, "should delete events 0-1");
        assert_eq!(result.status, TrimStatus::Trimmed);

        let remaining = journal.events_for_run(run).expect("replay should succeed");
        assert_eq!(remaining.len(), 2, "should preserve events 2-3");
        for event in &remaining {
            assert!(
                event.seq().get() >= 2,
                "event seq {} should be >= 2",
                event.seq().get()
            );
        }
    }

    #[test]
    fn terminal_retention_blocks_recent_terminal_runs() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(1000);
        let workflow_id = WorkflowId::new(1);
        let digest = WorkflowDigest::from_bytes([0x11; DIGEST_BYTES]);

        // Create a terminal run: events 0..=3 with RunFinished at seq 3
        let events: Vec<JournalEvent> = (0..4u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else if i == 3 {
                    make_run_finished(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header_with_workflow(&journal, run, workflow_id, digest, 1000);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(2),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        // With retention policy of 5, this run should be blocked because
        // it is the only terminal run for this workflow (position 0 < 5)
        let policy = TrimPolicy {
            skip_noop_runs: true,
            retain_last_n_terminal: 5,
        };
        let result = journal.trim_events_for_run(run, policy);
        assert!(
            matches!(result, Err(TrimError::RetentionPolicyBlocks { .. })),
            "recent terminal run should be blocked by retention, got {:?}",
            result
        );
    }

    #[test]
    fn terminal_retention_allows_older_terminal_runs() {
        let (_temp, journal) = temp_journal();
        let workflow_id = WorkflowId::new(1);
        let digest = WorkflowDigest::from_bytes([0x22; DIGEST_BYTES]);

        // Create 5 terminal runs for the same workflow, all with snapshots
        for run_id in 1u64..=5 {
            let run = RunId::new(run_id);
            let events: Vec<JournalEvent> = (0..3u64)
                .map(|i| {
                    if i == 0 {
                        make_event(run, i)
                    } else if i == 2 {
                        make_run_finished(run, i)
                    } else {
                        make_step_started(run, i, i as u16 - 1)
                    }
                })
                .collect();
            journal
                .append_strict_batch(&events)
                .expect("batch should succeed");
            write_header_with_workflow(&journal, run, workflow_id, digest, run_id * 100);
            let snapshot = RunSnapshot {
                run,
                seq: EventSeq::new(1),
                workflow: digest,
                slots: vec![0u8],
                taint: vec![],
            };
            journal
                .put_snapshot(&snapshot)
                .expect("snapshot should succeed");
        }

        // With retention policy of 3, runs 1 and 2 (oldest) should be trimmable
        let policy = TrimPolicy {
            skip_noop_runs: true,
            retain_last_n_terminal: 3,
        };

        // Run 1 (accepted_at_ms=100) is the oldest, should be allowed
        let result = journal
            .trim_events_for_run(RunId::new(1), policy)
            .expect("oldest terminal run should be trimmable");
        assert_eq!(result.status, TrimStatus::Trimmed);
        assert_eq!(result.deleted_count, 1);
    }

    #[test]
    fn non_terminal_runs_ignore_retention_policy() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(2000);
        let workflow_id = WorkflowId::new(2);
        let digest = WorkflowDigest::from_bytes([0x33; DIGEST_BYTES]);

        // Non-terminal run: events 0..3, no terminal event
        let events: Vec<JournalEvent> = (0..3u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header_with_workflow(&journal, run, workflow_id, digest, 2000);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(1),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        // Even with retention policy, non-terminal runs should trim normally
        let policy = TrimPolicy {
            skip_noop_runs: true,
            retain_last_n_terminal: 0,
        };
        let result = journal
            .trim_events_for_run(run, policy)
            .expect("non-terminal run should trim");
        assert_eq!(result.status, TrimStatus::Trimmed);
        assert_eq!(result.deleted_count, 1);
    }

    #[test]
    fn replay_equivalence_after_trim() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(3000);
        let digest = WorkflowDigest::from_bytes([0x44; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..6u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal
            .put_snapshot(&snapshot)
            .expect("snapshot should succeed");

        // Trim removes events 0..2
        let trim_result = journal
            .trim_events_for_run(run, TrimPolicy::default())
            .expect("trim should succeed");
        assert_eq!(trim_result.deleted_count, 3, "should delete events 0-2");

        // After trim, replay from snapshot yields tail events 3..5
        let after_trim = journal
            .events_for_run(run)
            .expect("replay after trim should succeed");
        assert_eq!(after_trim.len(), 3, "should preserve events 3-5");
        for (i, event) in after_trim.iter().enumerate() {
            let expected_seq = 3 + i as u64;
            assert_eq!(
                event.seq().get(),
                expected_seq,
                "event at index {} should have seq {}",
                i,
                expected_seq
            );
        }

        // Verify trimmed events are actually gone by trying to read them directly
        for seq in 0..3u64 {
            let key = crate::keys::run_event_key(run, EventSeq::new(seq)).expect("key ok");
            assert!(
                journal.events.get(key).expect("get ok").is_none(),
                "event seq {} should be deleted",
                seq
            );
        }
    }

    #[test]
    fn trim_policy_default_includes_retention() {
        let policy = TrimPolicy::default();
        assert!(policy.skip_noop_runs);
        assert_eq!(policy.retain_last_n_terminal, 10);
    }

    #[test]
    fn no_durable_snapshot_error_has_correct_diagnostic_code() {
        let err = TrimError::NoDurableSnapshot { run: RunId::new(1) };
        assert_eq!(err.diagnostic_code(), TrimError::NO_DURABLE_SNAPSHOT_CODE);
    }

    #[test]
    fn retention_policy_blocks_error_has_correct_diagnostic_code() {
        let err = TrimError::RetentionPolicyBlocks { run: RunId::new(1) };
        assert_eq!(
            err.diagnostic_code(),
            TrimError::RETENTION_POLICY_BLOCKS_CODE
        );
    }

    // -----------------------------------------------------------------------
    // Trim Eligibility Diagnostic — Red Phase Tests (vb-zo9d)
    // -----------------------------------------------------------------------

    #[test]
    fn diagnostic_returns_eligible_and_blocked_runs() {
        let (_temp, journal) = temp_journal();
        let run_a = RunId::new(10_000);
        let run_b = RunId::new(10_001);
        let digest = WorkflowDigest::from_bytes([0x55; DIGEST_BYTES]);

        // Run A: has events and snapshot
        let events_a: Vec<JournalEvent> = (0..6u64)
            .map(|i| {
                if i == 0 {
                    make_event(run_a, i)
                } else {
                    make_step_started(run_a, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events_a)
            .expect("batch A should succeed");
        write_header(&journal, run_a, digest);
        let snapshot_a = RunSnapshot {
            run: run_a,
            seq: EventSeq::new(3),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot_a).expect("snapshot A ok");

        // Run B: has events but NO snapshot
        let events_b = [make_event(run_b, 0), make_step_started(run_b, 1, 0)];
        journal
            .append_strict_batch(&events_b)
            .expect("batch B should succeed");
        write_header(&journal, run_b, digest);

        let diag = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("diagnostic should succeed");

        assert_eq!(diag.total_runs, 2, "should report 2 total runs");
        assert_eq!(diag.eligible_runs, 1, "run A should be eligible");
        assert_eq!(diag.blocked_runs, 1, "run B should be blocked");

        let eligible = diag
            .runs
            .iter()
            .find(|r| matches!(r, TrimEligibility::Eligible { run, .. } if *run == run_a));
        assert!(
            eligible.is_some(),
            "run A should be Eligible, got {:?}",
            diag.runs
        );

        let blocked = diag
            .runs
            .iter()
            .find(|r| matches!(r, TrimEligibility::Blocked { run, .. } if *run == run_b));
        assert!(
            blocked.is_some(),
            "run B should be Blocked, got {:?}",
            diag.runs
        );
    }

    #[test]
    fn diagnostic_reports_correct_safe_point_and_trimmable_count() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(11_000);
        let digest = WorkflowDigest::from_bytes([0x66; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..10u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header(&journal, run, digest);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(5),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("snapshot ok");

        let diag = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("diagnostic should succeed");

        let eligible = diag.runs.iter().find_map(|r| match r {
            TrimEligibility::Eligible {
                run: r,
                safe_point,
                events_trimmable,
            } if *r == run => Some((*safe_point, *events_trimmable)),
            _ => None,
        });
        assert!(
            eligible.is_some(),
            "run should be eligible, got {:?}",
            diag.runs
        );
        let (safe_point, trimmable) = eligible.unwrap();
        assert_eq!(safe_point, EventSeq::new(5), "safe point should be seq 5");
        assert_eq!(trimmable, 5, "should report 5 trimmable events (0-4)");
        assert_eq!(
            diag.total_events_trimmable, 5,
            "aggregate trimmable should be 5"
        );
    }

    #[test]
    fn diagnostic_blocks_run_without_durable_snapshot() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(12_000);
        let digest = WorkflowDigest::from_bytes([0x77; DIGEST_BYTES]);

        let events = [make_event(run, 0), make_step_started(run, 1, 0)];
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header(&journal, run, digest);
        // No snapshot written

        let diag = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("diagnostic should succeed");

        assert_eq!(diag.total_runs, 1);
        assert_eq!(diag.eligible_runs, 0);
        assert_eq!(diag.blocked_runs, 1);

        let blocked = diag.runs.first().expect("should have one run result");
        assert!(
            matches!(
                blocked,
                TrimEligibility::Blocked {
                    run: r,
                    blocker: TrimBlocker::NoDurableSnapshot,
                } if *r == run
            ),
            "run should be blocked by NoDurableSnapshot, got {:?}",
            blocked
        );
    }

    #[test]
    fn diagnostic_blocks_recent_terminal_run_under_retention() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(13_000);
        let workflow_id = WorkflowId::new(3);
        let digest = WorkflowDigest::from_bytes([0x88; DIGEST_BYTES]);

        // Terminal run: events 0..=3 with RunFinished at seq 3
        let events: Vec<JournalEvent> = (0..4u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else if i == 3 {
                    make_run_finished(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header_with_workflow(&journal, run, workflow_id, digest, 13_000);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(2),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("snapshot ok");

        let policy = TrimPolicy {
            skip_noop_runs: true,
            retain_last_n_terminal: 5,
        };
        let diag = journal
            .trim_eligibility_diagnostic(policy)
            .expect("diagnostic should succeed");

        assert_eq!(diag.total_runs, 1);
        assert_eq!(diag.eligible_runs, 0);
        assert_eq!(diag.blocked_runs, 1);

        let blocked = diag.runs.first().expect("should have one run result");
        assert!(
            matches!(
                blocked,
                TrimEligibility::Blocked {
                    run: r,
                    blocker: TrimBlocker::RetentionPolicy {
                        retain_last_n_terminal: 5,
                    },
                } if *r == run
            ),
            "run should be blocked by RetentionPolicy, got {:?}",
            blocked
        );
    }

    #[test]
    fn diagnostic_allows_non_terminal_run_despite_retention() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(14_000);
        let workflow_id = WorkflowId::new(4);
        let digest = WorkflowDigest::from_bytes([0x99; DIGEST_BYTES]);

        // Non-terminal run: no RunFinished event
        let events: Vec<JournalEvent> = (0..3u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header_with_workflow(&journal, run, workflow_id, digest, 14_000);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(1),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("snapshot ok");

        let policy = TrimPolicy {
            skip_noop_runs: true,
            retain_last_n_terminal: 10,
        };
        let diag = journal
            .trim_eligibility_diagnostic(policy)
            .expect("diagnostic should succeed");

        assert_eq!(diag.total_runs, 1);
        assert_eq!(diag.eligible_runs, 1);
        assert_eq!(diag.blocked_runs, 0);

        let eligible = diag.runs.first().expect("should have one run result");
        assert!(
            matches!(
                eligible,
                TrimEligibility::Eligible {
                    run: r,
                    safe_point: EventSeq(1),
                    events_trimmable: 1,
                } if *r == run
            ),
            "non-terminal run should be eligible, got {:?}",
            eligible
        );
    }

    #[test]
    fn diagnostic_does_not_delete_events() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(15_000);
        let digest = WorkflowDigest::from_bytes([0xAA; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..5u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header(&journal, run, digest);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(3),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("snapshot ok");

        let before = journal
            .events_for_run(run)
            .expect("events before diagnostic");
        assert_eq!(before.len(), 5);

        let _diag = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("diagnostic should succeed");

        let after = journal
            .events_for_run(run)
            .expect("events after diagnostic");
        assert_eq!(
            after.len(),
            5,
            "diagnostic must not delete events, before={} after={}",
            before.len(),
            after.len()
        );
    }

    #[test]
    fn diagnostic_is_idempotent() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(16_000);
        let digest = WorkflowDigest::from_bytes([0xBB; DIGEST_BYTES]);

        let events: Vec<JournalEvent> = (0..4u64)
            .map(|i| {
                if i == 0 {
                    make_event(run, i)
                } else {
                    make_step_started(run, i, i as u16 - 1)
                }
            })
            .collect();
        journal
            .append_strict_batch(&events)
            .expect("batch should succeed");
        write_header(&journal, run, digest);

        let snapshot = RunSnapshot {
            run,
            seq: EventSeq::new(2),
            workflow: digest,
            slots: vec![0u8],
            taint: vec![],
        };
        journal.put_snapshot(&snapshot).expect("snapshot ok");

        let diag1 = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("first diagnostic should succeed");
        let diag2 = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("second diagnostic should succeed");

        assert_eq!(diag1.total_runs, diag2.total_runs);
        assert_eq!(diag1.eligible_runs, diag2.eligible_runs);
        assert_eq!(diag1.blocked_runs, diag2.blocked_runs);
        assert_eq!(diag1.total_events_trimmable, diag2.total_events_trimmable);
        assert_eq!(diag1.runs.len(), diag2.runs.len());
        for (a, b) in diag1.runs.iter().zip(diag2.runs.iter()) {
            assert_eq!(a, b, "diagnostic results should be identical across calls");
        }
    }

    #[test]
    fn diagnostic_returns_empty_for_empty_journal() {
        let (_temp, journal) = temp_journal();

        let diag = journal
            .trim_eligibility_diagnostic(TrimPolicy::default())
            .expect("diagnostic should succeed");

        assert_eq!(diag.total_runs, 0);
        assert_eq!(diag.eligible_runs, 0);
        assert_eq!(diag.blocked_runs, 0);
        assert_eq!(diag.total_events_trimmable, 0);
        assert!(diag.runs.is_empty());
    }
}
