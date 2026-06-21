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
    /// Reads only the highest-sequence snapshot key for the given run. The
    /// snapshot keyspace is sorted in big-endian order on `(run, seq)` so the
    /// maximum-seq entry is the last item in the reverse prefix scan; this
    /// avoids decoding every snapshot's value (BLAKE3 + postcard) on every
    /// trim pass.
    ///
    /// Returns `None` if no snapshot exists.
    pub fn latest_durable_snapshot_seq(&self, run: RunId) -> TrimResult<Option<EventSeq>> {
        let prefix_key = snapshot_prefix_key(run);
        let Some(item) = self.run_snapshot.prefix(prefix_key).next_back() else {
            return Ok(None);
        };
        let (key, _) = item.into_inner().map_err(TrimError::from)?;
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
        Ok(Some(EventSeq::new(key_seq_u64)))
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
            // vb-1rqz7.26 / SC-006: a key shorter than the run-event contract
            // (17 bytes) is by definition corruption; fail closed instead of
            // silently leaving the row un-trimmable.
            if key.len() < 17 {
                return Err(TrimError::IncompleteTrim { deleted_count });
            }
            let slice = key
                .get(9..17)
                .ok_or(TrimError::IncompleteTrim { deleted_count })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| TrimError::IncompleteTrim { deleted_count })?;
            let seq_u64 = u64::from_be_bytes(seq_bytes);

            if seq_u64 < cutoff_seq.get() {
                // vb-1rqz7.35 / SC-008: `key.to_vec()` allocates a 17-byte
                // buffer per removed event. The allocation is bounded by
                // the per-run event count, dominated by Fjall's LSM delete
                // path, and has no evidence-backed hot-path cost in
                // production. A borrowed-slice delete API would require
                // upstream Fjall support; until that exists and a
                // benchmark demonstrates the savings, we keep the allocation.
                batch.remove(&self.events, key.to_vec());
                deleted_count = deleted_count.saturating_add(1);
            }
        }

        if deleted_count == 0 {
            // vb-1rqz7.27 / SC-007: a zero-delete trim must always report NoOp
            // and skip the empty batch commit, regardless of the
            // `skip_noop_runs` policy knob. An empty commit pays WAL/LSM cost
            // without producing any durable change.
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
        let mut results = Vec::with_capacity(headers.len());
        let retained = self.compute_retained_terminal_runs(&policy)?;

        for header in headers {
            // vb-1rqz7.30 / SC-005: the batch API consults a precomputed
            // retention set instead of re-deriving it per run (which would
            // re-scan every header + every terminal run's events for each
            // input run).
            if retained.contains(&header.run) {
                continue;
            }
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
        let mut runs = Vec::with_capacity(headers.len());
        let mut total_runs: u64 = 0;
        let mut eligible_runs: u64 = 0;
        let mut blocked_runs: u64 = 0;
        let mut total_events_trimmable: u64 = 0;
        let retained = self
            .compute_retained_terminal_runs(&policy)
            .map_err(JournalError::from)?;

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
                    return Err(JournalError::from(e));
                }
            };

            // vb-1rqz7.30 / SC-005: consult the precomputed retained set
            // instead of re-deriving per-run retention.
            if retained.contains(&header.run) {
                blocked_runs = blocked_runs.saturating_add(1);
                runs.push(TrimEligibility::Blocked {
                    run: header.run,
                    blocker: TrimBlocker::RetentionPolicy {
                        retain_last_n_terminal: policy.retain_last_n_terminal,
                    },
                });
                continue;
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
            // vb-1rqz7.25 / CC-002: a key shorter than the run-event contract
            // (17 bytes) is by definition corruption; fail closed instead of
            // silently treating it as nothing.
            if key.len() < 17 {
                return Err(JournalError::from(TrimError::IncompleteTrim { deleted_count: count }));
            }
            let slice = key.get(9..17).ok_or_else(|| {
                JournalError::from(TrimError::IncompleteTrim { deleted_count: count })
            })?;
            let seq_bytes: [u8; 8] = slice.try_into().map_err(|_| {
                JournalError::from(TrimError::IncompleteTrim { deleted_count: count })
            })?;
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

    /// Computes the set of runs protected by retention policy in a single pass.
    ///
    /// vb-1rqz7.30 / SC-005: replaces the per-call quadratic scan in
    /// `check_retention_policy` with a workflow-grouped pre-pass that runs
    /// once per batch trim invocation. Each run's terminal status is read
    /// exactly once, headers are scanned exactly once, and the resulting
    /// set is reused by every per-run decision.
    pub(crate) fn compute_retained_terminal_runs(
        &self,
        policy: &TrimPolicy,
    ) -> TrimResult<std::collections::HashSet<RunId>> {
        use std::collections::HashMap;

        let retain_count =
            usize::try_from(policy.retain_last_n_terminal).unwrap_or(usize::MAX);
        if retain_count == 0 {
            return Ok(std::collections::HashSet::new());
        }

        let headers = self.run_headers().map_err(TrimError::from)?;
        // Map workflow_id -> terminal runs sorted newest-first.
        let mut grouped: HashMap<vb_core::WorkflowId, Vec<(RunId, u64)>> =
            HashMap::with_capacity(headers.len());
        for header in headers {
            if !self.has_terminal_event(header.run)? {
                continue;
            }
            grouped
                .entry(header.workflow_id)
                .or_default()
                .push((header.run, header.accepted_at_ms));
        }

        let mut retained = std::collections::HashSet::new();
        for runs in grouped.values_mut() {
            runs.sort_by_key(|(_, ts)| std::cmp::Reverse(*ts));
            for (run, _) in runs.iter().take(retain_count) {
                retained.insert(*run);
            }
        }
        Ok(retained)
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
        let mut terminal_runs: Vec<(RunId, u64)> = Vec::with_capacity(all_headers.len());

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
