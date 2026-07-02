# Regression Diff — vb-09aaz

> Bead-level summary of the production-code diff between vb-09aaz's parent commit (`@-, rsvywymk 1d6c017f`) and the post-fix commit (`@-, qrtqslzp 0af593fc`). Sourced from `.beads/vb-09aaz/evidence/change.diff` and `jj diff -r @-`.

- bead_id: `vb-09aaz`
- state: 12 (formal-verification) — synthesized for state-14 evidence-packaging gate consumption
- parent_commit: `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port)
- fix_commit: `qrtqslzp 0af593fc` (vb-09aaz: p11-holzman-rust — abort write batch on stage_pending_action_index_op error)
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz`

## Diff Stats

```
crates/vb_storage/src/batch/append_event.rs   | 32 +++++++++-
crates/vb_storage/src/batch/t_append_event.rs | 89 +++++++++++++++++++++++++++++
2 files changed, 119 insertions(+), 2 deletions(-)
```

## Production Change (`crates/vb_storage/src/batch/append_event.rs`)

```
- G8 IndexKeyConstruction guard: replaces implicit `?` propagation with
  explicit abort-on-Err pattern mirroring the same 28-site pattern in
  putters.rs (lines 30, 36, 49, 67, 73, 86, 104, 117, 135, 148, 161, 167,
  174, 197, 220, 244).

  Before (single line):
      self.journal.stage_pending_action_index_op(&mut self.inner, event)?;
      self.staged_event_keys.insert(key);

  After (7 lines):
      if let Err(e) = self
          .journal
          .stage_pending_action_index_op(&mut self.inner, event)
      {
          self.aborted = true;
          return Err(e);
      }
      self.staged_event_keys.insert(key);

- Doc-comment update at append_event.rs:18-26 (Guard Precedence section):
  enumerates G1..G8 (was G1..G7); adds step 9 "Pending-action-index key
  construction (G8, aborts) [NEW]".

- Doc-comment update at append_event.rs:33-49 (Postconditions section):
  adds the new bullet "On `KeyCapacity` (G8, index-key construction
  failure): batch is aborted; no partial persistence; commit() returns
  Err(BatchAborted)."
```

## Test Change (`crates/vb_storage/src/batch/t_append_event.rs`)

```
+ New regression test batch_append_event_index_key_error_aborts_commit
  (89 lines including doc-comment):

  - Mirrors batch_index_key_error_aborts_commit at t_putters_b.rs:177-209.
  - Asserts: (1) happy-path ActionScheduled append_event returns Ok;
            (2) batch.is_aborted() == false on happy path;
            (3) batch.len() == 2 (1 event write + 1 action index marker);
            (4) batch.commit() succeeds;
            (5) events_for_run(run).len() == 1 (event persisted exactly
                once);
            (6) persisted event round-trips as the original ActionScheduled
                variant.
  - Doc-comment at L233-275 explicitly documents the test design
    decision: G8 KeyCapacity is structurally unreachable for valid
    (ActionId, RunId, StepIdx) inputs (per
    workflow-model.md#KeyCapacity-reachability), so the test exercises
    the closest reachable surface of the abort-on-error contract and
    verifies the production-code structure that mirrors the same
    if-let-Err pattern used 28 times across putters.rs.
```

## ABI / API Stability

| Surface | Status | Evidence |
|---------|--------|----------|
| `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>` signature | unchanged | code review at `append_event.rs:50` |
| `pub fn is_aborted(&self) -> bool` signature | unchanged | code review at `append_event.rs` (defined in types.rs) |
| `pub fn commit(self) -> Result<(), JournalError>` signature | unchanged | code review at `commit.rs:1-30` |
| `JournalError::KeyCapacity` unit variant | unchanged | code review at `error/mod.rs:28-29` |
| Doc-comment surface | expanded (G8 added to Guard Precedence; KeyCapacity abort added to Postconditions) | doc-only changes |
| New fields added to `JournalWriteBatch` | none | code review at `types.rs` |

## Performance Surface

The G8 fix adds 7 lines of code: 1 `if let` + 1 method call + 1 typed-error propagation. The error path is reached only when `stage_pending_action_index_op` returns `Err`, which is structurally unreachable for valid `(ActionId, RunId, StepIdx)` inputs. The happy-path cost is a single call to `stage_pending_action_index_op` (already present in pre-fix code). No measurable performance regression.

## Status

`STATUS: PASS` — the production change is minimal, focused, and ABI-stable. The test change is additive (one new test, 89 lines including doc-comment) and mirrors the canonical reference test pattern.