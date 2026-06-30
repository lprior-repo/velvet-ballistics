use crate::trimming::helpers::snapshot_prefix_key;
use crate::trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};
use crate::{EventSeq, FjallJournal, JournalError};
use fjall::Readable;
use vb_core::{RunId, WorkflowId};

impl FjallJournal {
    /// Reads only the highest-sequence snapshot key for the given run. The
    /// snapshot keyspace is sorted in big-endian order on `(run, seq)` so the
    /// maximum-seq entry is the last item in the reverse prefix scan; this
    /// avoids decoding every snapshot's value (BLAKE3 + postcard) on every
    /// trim pass.
    ///
    /// Returns `None` if no snapshot exists.
    ///
    /// (vb-n65x4 / SC-004: re-applies the perf fix from commit `7586b096f`
    /// that was reverted in `944b95d5c`. The previous full-prefix loop
    /// paid `O(N_snapshots × MAX_SNAPSHOT_BYTES)` of BLAKE3+postcard per
    /// trim call. The key-only `next_back()` lookup is `O(1)` once the
    /// LSM tree has the prefix cursor positioned.)
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

        // CC-003 fix: hoist a single scratch key buffer outside the trim
        // loop and reuse it for every `batch.remove` call. `key` here is
        // a `fjall::UserKey` (= `Slice`), which is `ByteView`-backed and
        // already cheap to inspect; copying its bytes once into a reusable
        // `Vec<u8>` and passing `as_slice()` to `OwnedWriteBatch::remove`
        // avoids both the per-iteration `Vec<u8>` heap allocation
        // (previously `key.to_vec()`) and the Arc-cheap `key.clone()`
        // overhead from SC-008. `&[u8]` implements `Into<UserKey>` via
        // `lsm_tree::Slice`, so the call site stays one expression.
        let mut key_buf: Vec<u8> = Vec::with_capacity(64);
        for item in self.events.prefix(prefix_key) {
            let key = item.key().map_err(TrimError::from)?;
            if key.len() < 17 {
                return Err(TrimError::IncompleteTrim { deleted_count });
            }
            let slice = key
                .get(9..17)
                .ok_or(TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_bytes: [u8; 8] = slice
                .try_into()
                .map_err(|_| TrimError::IncompleteTrim { deleted_count: 0 })?;
            let seq_u64 = u64::from_be_bytes(seq_bytes);

            if seq_u64 < cutoff_seq.get() {
                key_buf.clear();
                key_buf.extend_from_slice(&key);
                batch.remove(&self.events, key_buf.as_slice());
                deleted_count = deleted_count.saturating_add(1);
            }
        }

        if deleted_count == 0 {
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
                return Err(JournalError::from(TrimError::IncompleteTrim {
                    deleted_count: count,
                }));
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

    /// Computes the set of terminal run IDs that are retained by the
    /// retention policy, without performing any trim.
    ///
    /// This is a thin I/O wrapper: it fetches all run headers, asks
    /// `Self::has_terminal_event` for each one (which scans the durable
    /// journal), groups the terminal runs by workflow, and delegates the
    /// pure compute to
    /// [`retained_terminal_runs_top_n`](Self::retained_terminal_runs_top_n).
    ///
    /// **Short-circuit:** when `policy.retain_last_n_terminal == 0`, the
    /// returned set is empty without scanning journal events.
    ///
    /// **Post-trim semantics:** terminal detection reads the current journal
    /// state, so any events deleted by a prior trim are no longer observed.
    /// A run whose terminal events were trimmed therefore does not appear
    /// in the returned set.
    pub fn compute_retained_terminal_runs(
        &self,
        policy: &TrimPolicy,
    ) -> Result<std::collections::HashSet<RunId>, JournalError> {
        if policy.retain_last_n_terminal == 0 {
            return Ok(std::collections::HashSet::new());
        }

        let headers = self.run_headers()?;
        let mut by_workflow: std::collections::BTreeMap<WorkflowId, Vec<(RunId, u64)>> =
            std::collections::BTreeMap::new();
        for h in headers {
            if self.has_terminal_event(h.run).map_err(JournalError::from)? {
                by_workflow
                    .entry(h.workflow_id)
                    .or_default()
                    .push((h.run, h.accepted_at_ms));
            }
        }

        Ok(Self::retained_terminal_runs_top_n(
            by_workflow,
            policy.retain_last_n_terminal,
        ))
    }

    /// Pure compute: returns the set of retained terminal runs from a
    /// pre-grouped `by_workflow` map, taking the top-N per workflow by
    /// `accepted_at_ms` descending.
    ///
    /// This is the testable core of [`Self::compute_retained_terminal_runs`];
    /// it has no I/O, no I/O-error channel, and a fixed bounded loop per
    /// workflow. Splitting I/O from compute (Farley imperative-shell rule)
    /// lets the grouping + sort + take-N logic be unit-tested without a
    /// Fjall journal.
    ///
    /// `retain_last_n_terminal == 0` yields an empty set without reading
    /// the input (preserves the short-circuit semantics of
    /// `compute_retained_terminal_runs`).
    pub fn retained_terminal_runs_top_n(
        mut by_workflow: std::collections::BTreeMap<WorkflowId, Vec<(RunId, u64)>>,
        retain_last_n_terminal: u32,
    ) -> std::collections::HashSet<RunId> {
        if retain_last_n_terminal == 0 {
            return std::collections::HashSet::new();
        }

        let retain_count = usize::try_from(retain_last_n_terminal).unwrap_or(usize::MAX);
        let mut retained: std::collections::HashSet<RunId> = std::collections::HashSet::new();
        for runs in by_workflow.values_mut() {
            // Newest terminal run first (descending by accepted_at_ms).
            runs.sort_by_key(|(_, ts)| std::cmp::Reverse(*ts));
            for (run, _) in runs.iter().copied().take(retain_count) {
                retained.insert(run);
            }
        }
        retained
    }
}
