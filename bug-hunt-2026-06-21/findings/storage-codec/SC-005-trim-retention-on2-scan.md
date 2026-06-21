# SC-005: O(N²) run-header and terminal-event scanning in `check_retention_policy`

- **Severity**: High
- **Category**: perf
- **Location**: `crates/vb_storage/src/trimming/logic.rs:260-298` (inner loop); called from `crates/vb_storage/src/trimming/logic.rs:67` via `trim_events_for_run`; orchestrated by `crates/vb_storage/src/trimming/logic.rs:116-130` (`trim_all_eligible_runs`)
- **Confidence**: confirmed

## Description

`check_retention_policy(run, policy)` calls `self.run_headers()` (full keyspace scan) and then for each header of the same workflow calls `self.has_terminal_event(h.run)`, which itself performs a full journal-event prefix scan and decodes each event. `trim_all_eligible_runs` invokes this for every run header in the journal, producing **N × N × M** behavior: N runs × N sibling headers × M journal events per run.

## Evidence

```rust
// crates/vb_storage/src/trimming/logic.rs:260-298
pub(crate) fn check_retention_policy(&self, run: RunId, policy: &TrimPolicy) -> TrimResult<()> {
    if policy.retain_last_n_terminal == 0 { return Ok(()); }
    if !self.has_terminal_event(run)? { return Ok(()); }                  // scan #1

    let Some(header) = self.run_header(run).map_err(TrimError::from)? else { return Ok(()); };
    let all_headers = self.run_headers().map_err(TrimError::from)?;        // <-- full keyspace scan
    let mut terminal_runs: Vec<(RunId, u64)> = Vec::new();

    for h in all_headers {                                                 // <-- O(N)
        if h.workflow_id != header.workflow_id { continue; }
        if self.has_terminal_event(h.run)? {                              // <-- O(M) decode per run
            terminal_runs.push((h.run, h.accepted_at_ms));
        }
    }
    ...
}
```

`has_terminal_event` (line 236-253) iterates all events for the run and decodes each one via `decode_journal_event` to test `is_terminal_event`. There is no terminal-event index.

`trim_all_eligible_runs` (line 116-130) calls `trim_events_for_run` per header, which calls `check_retention_policy` per header. So for N terminal runs in the same workflow, the inner loop re-scans all N runs N times, and each inner scan re-decodes all M events. Total work is **O(N²·M)** per `trim_all_eligible_runs` invocation.

## Adversarial Check

N is bounded by the total number of runs in the journal, which could be thousands to tens of thousands in a production deployment. M (events per run) can be hundreds. The cost is `10_000² × 100 = 10^10` BLAKE3 + postcard decodes — minutes to hours of CPU for a single trim pass. This is not a micro-optimization: it converts a daily trim job into a denial-of-service. The hot-path justification is the N²·M loop body running real codec work (BLAKE3 hash verify + postcard deserialize) on every iteration.

## Suggested Fix

1. Add a `terminal_at_ms: Option<u64>` field (or dedicated terminal-state index keyspace) updated when a terminal event is appended. This makes `has_terminal_event` an O(1) keyspace lookup instead of a full scan.
2. Replace the per-run `check_retention_policy` call from `trim_all_eligible_runs` with a single workflow-grouped pre-pass: scan headers once, group by workflow_id, sort by `accepted_at_ms`, mark the top-K retained runs per workflow, then check the marked set per run in O(1).
3. Memoize `has_terminal_event` results for the duration of a single `trim_all_eligible_runs` invocation.
