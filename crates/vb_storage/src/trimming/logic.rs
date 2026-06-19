use crate::codec::decode_record;
use crate::constants::{MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES};
use crate::trimming::helpers::snapshot_prefix_key;
use crate::trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};
use crate::{EventSeq, FjallJournal, JournalError};
use fjall::Readable;
use vb_core::RunId;

impl FjallJournal {
    /// Returns the latest durable snapshot sequence for a run.
    ///
    /// Scans all snapshots for the given run and returns the one with the
    /// highest sequence number. Returns `None` if no snapshot exists.
    pub fn latest_durable_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>> {
        let prefix_key = snapshot_prefix_key(run);
        let mut latest: Option<EventSeq> = None;

        for item in self.run_snapshot.prefix(prefix_key) {
            let (key, value) = item.into_inner().map_err(TrimError::from)?;
            if key.len() < 17 {
                return Err(TrimError::IncompleteTrim { deleted_count: 0 });
            }
            let slice = key
                .get(9..17)
                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| TrimError::IncompleteTrim { deleted_count: 0 })?;
            let key_seq_u64 = u64::from_be_bytes(seq_bytes);
            let key_seq = EventSeq::new(key_seq_u64);
            let (_, snapshot): (_, crate::recovery::RunSnapshot) =
                decode_record(value.as_ref(), MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)
                    .map_err(TrimError::from)?;
            if snapshot.run != run {
                return Err(TrimError::Journal(JournalError::WrongRun {
                    expected: run,
                    actual: snapshot.run,
                }));
            }
            if snapshot.seq != key_seq {
                return Err(TrimError::Journal(JournalError::SequenceGap {
                    expected: key_seq,
                    actual: snapshot.seq,
                }));
            }
            latest = Some(match latest {
                Some(current) if current.get() >= key_seq_u64 => current,
                _ => key_seq,
            });
        }

        Ok(latest)
    }

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
            let (_, event) = crate::codec::decode_journal_event(
                value.as_ref(),
                crate::constants::MAGIC_JOURNAL_EVENT,
                crate::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            )
            .map_err(TrimError::from)?;
            if crate::recovery::replay::is_terminal_event(&event) {
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

        let position = terminal_runs
            .iter()
            .position(|(r, _)| *r == run)
            .unwrap_or(terminal_runs.len());

        let retain_count = usize::try_from(policy.retain_last_n_terminal).unwrap_or(usize::MAX);
        if position < retain_count {
            return Err(TrimError::RetentionPolicyBlocks { run });
        }

        Ok(())
    }
}
