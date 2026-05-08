//! Journal trimming with retention policy.
//!
//! Periodically trims old journal events to reduce storage while preserving
//! events needed for recovery. Events older than a confirmed snapshot are
//! eligible for trimming.

use vb_core::RunId;

use crate::{EventSeq, FjallJournal, JournalError};

/// Retention policy for journal trimming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrimPolicy {
    /// If true, skip runs that have no events to trim (no-op runs).
    pub skip_noop_runs: bool,
}

impl Default for TrimPolicy {
    fn default() -> Self {
        Self {
            skip_noop_runs: true,
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
    /// No confirmed snapshot found for run.
    #[error("no confirmed snapshot for run {run:?}")]
    NoSnapshot {
        /// Run without a snapshot.
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
    pub const NO_SNAPSHOT_CODE: vb_core::DiagnosticCode = vb_core::DiagnosticCode::new(0x4101);
    pub const INCOMPLETE_TRIM_CODE: vb_core::DiagnosticCode = vb_core::DiagnosticCode::new(0x4102);

    #[must_use]
    pub const fn diagnostic_code(&self) -> vb_core::DiagnosticCode {
        match self {
            Self::Fjall(_) => JournalError::FJALL_CODE,
            Self::Journal(_) => JournalError::FJALL_CODE,
            Self::NoSnapshot { .. } => Self::NO_SNAPSHOT_CODE,
            Self::IncompleteTrim { .. } => Self::INCOMPLETE_TRIM_CODE,
        }
    }
}

/// Result type for trim operations.
pub type TrimResult<T> = Result<T, TrimError>;

impl FjallJournal {
    /// Returns the latest confirmed snapshot sequence for a run.
    ///
    /// Scans all snapshots for the given run and returns the one with the
    /// highest sequence number. Returns `None` if no snapshot exists.
    pub fn latest_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>> {
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
    /// Removes events with sequence numbers less than the latest confirmed
    /// snapshot sequence. If no snapshot exists for the run, returns an error.
    ///
    /// Trimming is idempotent: subsequent trims of an already-trimmed run
    /// are a no-op.
    pub fn trim_events_for_run(
        &self,
        run: RunId,
        policy: TrimPolicy,
    ) -> TrimResult<TrimmedRunResult> {
        let Some(cutoff_seq) = self.latest_snapshot_seq(run)? else {
            return Err(TrimError::NoSnapshot { run });
        };

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

    /// Trims journal events for all runs with confirmed snapshots.
    ///
    /// Iterates over all run headers and trims events for each run that
    /// has a confirmed snapshot. Runs without snapshots are skipped.
    pub fn trim_all_eligible_runs(&self, policy: TrimPolicy) -> TrimResult<Vec<TrimmedRunResult>> {
        let headers = self.run_headers()?;
        let mut results = Vec::new();

        for header in headers {
            match self.trim_events_for_run(header.run, policy) {
                Ok(result) => results.push(result),
                Err(TrimError::NoSnapshot { .. }) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(results)
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
    use vb_core::{RunId, StepIdx, WorkflowDigest, WorkflowId};

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
            matches!(result, Err(TrimError::NoSnapshot { .. })),
            "should error when no snapshot exists, got {:?}",
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
    fn latest_snapshot_seq_returns_highest_seq() {
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

        let latest = journal.latest_snapshot_seq(run).expect("should succeed");
        assert_eq!(latest, Some(EventSeq::new(5)), "latest should be seq 5");
    }

    #[test]
    fn latest_snapshot_seq_returns_none_for_no_snapshots() {
        let (_temp, journal) = temp_journal();
        let run = RunId::new(800);

        let latest = journal.latest_snapshot_seq(run).expect("should succeed");
        assert_eq!(latest, None);
    }
}
