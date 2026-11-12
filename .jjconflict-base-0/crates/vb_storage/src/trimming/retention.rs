use crate::trimming::logic::decode_event_seq_for_trim;
use crate::trimming::{TrimError, TrimPolicy, TrimResult};
use crate::{FjallJournal, JournalError};
use fjall::Readable;
use vb_core::{RunId, WorkflowId};

impl FjallJournal {
    /// Checks whether a run has reached a terminal state by scanning its
    /// journal events for a terminal event variant.
    pub(crate) fn has_terminal_event(&self, run: RunId) -> TrimResult<bool> {
        let prefix = crate::keys::run_prefix_key(run)?;
        let snap = self.database.snapshot();

        for item in snap.prefix(&self.events, prefix) {
            let (key, value) = item.into_inner().map_err(TrimError::from)?;
            decode_event_seq_for_trim(key.as_ref(), run, 0)?;
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
