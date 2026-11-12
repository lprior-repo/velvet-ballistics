use crate::keys::decode_storage_key;
use crate::trimming::helpers::snapshot_prefix_key;
use crate::trimming::{
    TrimBlocker, TrimDiagnostic, TrimEligibility, TrimError, TrimPolicy, TrimResult, TrimStatus,
    TrimmedRunResult,
};
use crate::types::StorageKey;
use crate::{EventSeq, FjallJournal, JournalError};
use fjall::Readable;
use vb_core::RunId;

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
        // Decode via the typed key helper so the scan rejects short,
        // overlong, wrong-prefix, wrong-run, and reserved-sentinel keys
        // instead of treating a malformed prefix collision as durable.
        decode_snapshot_seq_for_trim(key.as_ref(), run).map(Some)
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
            let seq = decode_event_seq_for_trim(key.as_ref(), run, deleted_count)?;

            if seq < cutoff_seq {
                key_buf.clear();
                key_buf.extend_from_slice(key.as_ref());
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
            let seq =
                decode_event_seq_for_trim(key.as_ref(), run, count).map_err(JournalError::from)?;

            if seq < safe_point {
                count = count.saturating_add(1);
            }
        }

        Ok(count)
    }
}

fn decode_snapshot_seq_for_trim(key: &[u8], run: RunId) -> TrimResult<EventSeq> {
    match decode_storage_key(key) {
        Ok(StorageKey::RunSnapshot { run: key_run, seq }) if key_run == run => Ok(seq),
        _ => Err(TrimError::IncompleteTrim { deleted_count: 0 }),
    }
}

pub(crate) fn decode_event_seq_for_trim(
    key: &[u8],
    run: RunId,
    deleted_count: u64,
) -> TrimResult<EventSeq> {
    match decode_storage_key(key) {
        Ok(StorageKey::RunEvent { run: key_run, seq }) if key_run == run => Ok(seq),
        _ => Err(TrimError::IncompleteTrim { deleted_count }),
    }
}
