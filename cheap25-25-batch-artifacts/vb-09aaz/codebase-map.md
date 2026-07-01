# Codebase Map — vb-09aaz

- bead_id: vb-09aaz
- title: Storage: abort write batch on all index key construction failures (P1 bug)
- description: QueuedWriter write batch currently does not abort when index key construction fails mid-batch, leading to partial writes. Abort the batch on any index key construction failure and surface a typed error.
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz
- captured_at: 2026-07-01T15:30:00Z
- agent: explore (go-skill State 2)
- upstream_main: 2c8ea33c9
- parent_jj_commit: rsvywymk 1d6c017f

## Bug Surface (P1 — partial write hazard)

The QueuedWriter batch commit path in `JournalWriteBatch::append_event` stages a
journal event into `self.inner` BEFORE calling `stage_pending_action_index_op`.
If the index-key construction step fails (e.g. `JournalError::KeyCapacity` from
`index_action_key`), the function returns the error WITHOUT setting
`self.aborted = true`. A subsequent caller-driven `commit()` then commits the
partially-staged batch — the durable event log gains the `run_event` write but
the `index_action` mutation is missing, leaving the pending-action cursor out
of sync with the durable event log. This is the canonical master §49
Crash-Consistency violation the batch's `aborted` flag exists to prevent.

### Primary defect site

- `crates/vb_storage/src/batch/append_event.rs:104-115` — `append_event` stages
  the journal event at L104 (`self.inner.insert(...)`), then calls
  `self.journal.stage_pending_action_index_op(&mut self.inner, event)?` at
  L114-115. The `?` propagates the typed error up, but `self.aborted` is NEVER
  set on the error path. Compare with the sibling `put_status_index` /
  `put_action_index` / `put_workflow_index` paths in
  `crates/vb_storage/src/batch/putters.rs`, which always set
  `self.aborted = true` on any fallible step (28 occurrences across the
  putters file). `append_event` is the lone outlier.

- `crates/vb_storage/src/batch/append_event.rs:62` — the ONLY `aborted = true`
  assignment in `append_event` is for `JournalError::DuplicateEvent` (durable
  duplicate guard). G8 IndexKeyConstruction is unguarded.

- `crates/vb_storage/src/batch/append_event.rs:18-26` — `# Guard Precedence (C6)`
  doc-comment lists 8 guards but only 7 are present in code; the index-key
  construction step is at position 8 but is not enumerated as a guard.

- `crates/vb_storage/src/batch/append_event.rs:33-41` — `# Postconditions (ensures)`
  doc-comment lists: `On DuplicateEvent: batch is aborted`. The
  index-key-construction failure case is undocumented.

### Secondary site (queued-writer path — review, not the bug)

- `crates/vb_storage/src/queue/writer/stage.rs:31-74` — `stage_queued_event`
  also calls `stage_pending_action_index_op` at L72. On error, the `?`
  propagates to `JournalWriterQueue::flush_batch` at
  `crates/vb_storage/src/queue/writer.rs:197-203`, which returns Err without
  committing the `fjall::OwnedWriteBatch`. No partial write is possible here
  because the batch is owned and dropped, not committed. The function does
  however increment the per-flush `staged_keys` HashSet for items 0..N before
  failing on item N, and items in `state.pending` are not drained on failure —
  they will be retried on the next `flush_batch`. The typed error IS surfaced
  to the caller; no abort flag is needed because the batch is single-shot.
  **Review-only — no fix required at this site.**

- `crates/vb_storage/src/journal/internal.rs:50-79` — `append_unfsynced` builds
  a fresh `OwnedWriteBatch` and commits it in the same function (L77). On
  index-op failure at L76, the batch is dropped, no commit. No partial-write
  hazard. **No fix required at this site.**

### Reference abort-on-key-error pattern (already proven correct)

- `crates/vb_storage/src/batch/putters.rs:188-204` (`put_status_index`) — every
  fallible step (key construction, encode) sets `self.aborted = true` before
  returning the typed error. The canonical fix template for `append_event`.

- `crates/vb_storage/src/batch/t_putters_b.rs:177-209`
  (`batch_index_key_error_aborts_commit`) — regression test that exercises
  the abort-on-key-construction-failure contract via `IndexStatusState::Other(0)`,
  which collides with `Submitted` and triggers
  `JournalError::IndexStatusStateCollision`. Asserts:
    1. The put returns the typed error.
    2. `batch.is_aborted()` is `true`.
    3. `batch.commit()` returns `Err(JournalError::BatchAborted)`.
    4. No events are durable for the run (replay is empty).
  An analogous test must be added for `append_event` /
  `stage_pending_action_index_op` failure. No production-side test currently
  exercises that guard.

### Existing test coverage on `append_event`

- `crates/vb_storage/src/batch/t_append_event.rs:4-229` — happy path,
  duplicate-event, invalid-event, monotonic len, all-or-nothing commit, digest
  verification. None of the 13 tests in this file exercises the
  `stage_pending_action_index_op` failure branch.

- `crates/vb_storage/src/batch/t_append_event.rs:38-43` — duplicate event
  asserts `batch.len() == 0` after failed append, but DOES NOT assert
  `is_aborted()` (the test is at the durable-duplicate guard G3, which DOES
  set `aborted = true` per production L62).

- `crates/vb_storage/src/index_maintenance_tests.rs` — 19 tests covering the
  happy path of index maintenance for every action-lifecycle event variant
  across `append_journaled`, `append_strict`, `append_strict_batch`,
  `JournalWriteBatch::append_event`, and `JournalWriterQueue::drain_all`. None
  of these exercise the KeyCapacity path of `index_action_key`.

### Public API surface affected

- `crates/vb_storage/src/batch/append_event.rs:42` —
  `pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError>`
  re-exported via `crates/vb_storage/src/batch/mod.rs` and `crates/vb_storage/src/lib.rs`.
  The fix changes the post-condition (typed error now guarantees
  `is_aborted() == true`) but does NOT change the function signature.

- `crates/vb_storage/src/batch/types.rs:67-70` — `is_aborted()` accessor is
  the canonical way for callers to inspect the abort flag. Used by callers
  to distinguish aborted from empty batches.

- `crates/vb_storage/src/batch/commit.rs:20-26` — `commit()` short-circuits
  when `self.aborted == true`. The fix relies on this existing short-circuit
  to prevent partial persistence.

### Verifier spec drift (GOD RULE 2 binding)

- `verification/verus/vb-vzcuf-PS-008.rs` and `vb-vzcuf-PS-009.rs` — both spec
  files claim `JournalWriteBatch::append_event` as their target. The
  production-mirror comments at
  `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95`
  enumerate the 7-guard order G1..G7 and explicitly stop at G7
  "Mutation: inner.insert(...)". G8 IndexKeyConstruction (the call to
  `stage_pending_action_index_op` at production L114-115) is NOT modeled in
  the Verus mirror. The spec mirror's `append_event` exec body ends at
  `self.inner_len += 1; self.staged_event_keys.insert(key);` and returns Ok,
  with no error variant for `KeyCapacity` from index-key construction.

  Adding G8 to the production guard order requires:
    1. Adding a `KeyCapacity { ... }` (or guarded `IndexKeyConstructionFailed`)
       witness to the spec error enum.
    2. Modeling the index-op outcome as an additional `index_key_ok: bool`
       mirror input (analogous to `encode_ok`).
    3. Updating the guard-order proof to assert G8 sets `aborted = true` on
       Err and preserves state across all earlier-passed guards.
    4. Re-running `bash scripts/verify-verus.sh` and
       `bash scripts/check-verus-production-binding.sh` per the AGENTS.md
       mandatory gates.

- `verification/verus/production_inner/vb_vzcuf_PS_009_production.rs:88` —
  documents `KeyCapacity` as "unreachable in this mirror" (L88 comment) and
  declares it "Unreachable" in the enum body (L181). With the new G8
  guard, KeyCapacity becomes reachable from `stage_pending_action_index_op`,
  and the mirror must be updated to match production.

### Existing Verus production-binding (WEAK)

- `verification/verus/production_inner/vb_vzcuf_PS_004_production.rs`,
  `_PS_006_production.rs`, `_PS_007_production.rs`, `_PS_008_production.rs`,
  `_PS_009_production.rs` — WEAK mirror pattern with drift gate header.
  Drift-gate header at `vb_vzcuf_PS_008_production.rs:5-14` requires
  regeneration whenever production changes. Any change to
  `append_event.rs` (production G1-G8 ordering) forces a regeneration of
  the corresponding `_production.rs` mirror file.

### No-fuzz / No-mutation blast radius

- Differential verification only. This bead modifies one production method
  (append_event) and possibly the queue-writer path. Trim verifier scope to:
    * Verus spec: vb-vzcuf-PS-008 (primary), PS-009 (secondary mirror).
    * Kani harness: any existing harness in `crates/vb_storage/src/kani_*`
      that exercises `JournalWriteBatch::append_event`. Current Kani
      harnesses listed under `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs`
      etc. — re-check after production patch.
    * No fuzz mutation required (no parser / codec change).
    * No loom model change (single-threaded append_event).
    * No Flux refinement change (no refinement types in batch).
    * proptest additions in `crates/vb_storage/tests/proptest_vb_vzcuf_PS_*.rs`.

### Doc references (master plan)

- `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-09aaz/velvet-ballistics-MASTER.md`
  — `vb-09aaz` is referenced by bead-id pattern only. No direct section
  reference in the master plan. Master §49 (Crash-Consistency Rule) is the
  governing contract for atomic batches.

- `docs/storage-journal.md` — describes Fjall journal durability modes and
  the atomicity guarantee. Does not enumerate the index-key-failure abort
  contract explicitly; that contract is enforced by source code invariants
  only.

### Risk tags

- persistence: PARTIAL-WRITE HAZARD. Event staged into `self.inner` but
  index mutation never applied if `KeyCapacity` triggers on
  `stage_pending_action_index_op`. Commit then persists the event without
  the index — the pending-action cursor becomes inconsistent with the
  durable event log. Recovery, inspection, and lookup traffic will see
  the action as still in flight indefinitely.
- concurrency: minor. `JournalWriteBatch` is `!Send + !Sync` by
  `PhantomData<*mut FjallJournal>` (types.rs:18-21, 30) so the bug is
  single-threaded. The queued-writer path's batch is also single-shot.
- public-api: append_event contract changes. Caller-facing doc-comment
  must be updated to enumerate the new post-condition.
- verifier-binding: Verus spec mirror at PS-008 / PS-009 must be
  extended to G8; production mirror comments at
  `_PS_008_production.rs:78-95` and `_PS_009_production.rs:88` need
  regeneration.
- migration: NONE. No schema change, no protocol change.
- user-visible-behavior: NONE under normal operation (the KeyCapacity
  branch is unreachable for valid `ActionId/RunId/StepIdx` triples
  bounded by `INDEX_ACTION_KEY_BYTES = 13`). The bug only triggers
  under degenerate inputs that exceed the 13-byte encoding capacity.

### Files mapped (in-scope)

- `crates/vb_storage/src/batch/append_event.rs` (PRIMARY FIX SITE)
- `crates/vb_storage/src/batch/types.rs` (consumer of `aborted` flag)
- `crates/vb_storage/src/batch/commit.rs` (consumer of `aborted` flag)
- `crates/vb_storage/src/batch/action_index.rs` (review — no change)
- `crates/vb_storage/src/batch/putters.rs` (reference pattern)
- `crates/vb_storage/src/batch/t_append_event.rs` (NEW TEST SITE)
- `crates/vb_storage/src/batch/t_putters_b.rs` (existing reference test)
- `crates/vb_storage/src/batch/mod.rs` (re-export module)
- `crates/vb_storage/src/queue/writer.rs` (queued writer, no fix required)
- `crates/vb_storage/src/queue/writer/stage.rs` (queued writer index step, no fix required)
- `crates/vb_storage/src/journal/internal.rs` (direct path, no fix required)
- `crates/vb_storage/src/error/mod.rs` (`KeyCapacity` variant definition)
- `crates/vb_storage/src/error/codes.rs` (`KeyCapacity` diagnostic code)
- `crates/vb_storage/src/keys.rs:139-155` (`index_action_key` constructor)
- `crates/vb_storage/src/constants.rs:79` (`INDEX_ACTION_KEY_BYTES = 13`)
- `crates/vb_storage/src/lib.rs` (top-level re-exports)
- `verification/verus/vb-vzcuf-PS-008.rs` (spec to extend)
- `verification/verus/vb-vzcuf-PS-009.rs` (spec mirror to extend)
- `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`
  (drift-gated mirror)
- `verification/verus/production_inner/vb_vzcuf_PS_009_production.rs`
  (drift-gated mirror)
- `crates/vb_storage/src/index_maintenance_tests.rs` (existing
  index-maintenance coverage; review only)
- `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs`
  (existing proptest using `is_aborted`; review only)

### Files explicitly excluded

- `crates/vb_runtime/**` — no batch append path; runtime consumes the
  storage API downstream.
- `crates/vb_core/**` — provides `ActionId`, `RunId`, `StepIdx` newtypes
  only; no storage logic.
- `crates/vb_cli/**` — CLI entry points do not touch the batch API.
- `crates/vb_queue_semantics/**` — orchestration, not storage.
- `crates/workspace_tests/tests/vb_*` unrelated contracts — out of scope.

### Open questions / unknown

- UNKNOWN: Does the queued-writer `flush_batch` need to surface the typed
  error to the operator before retrying, or is the current "drop the
  OwnedWriteBatch and let the caller retry" behavior acceptable? The
  current code returns the typed error up to `drain_all`, which surfaces
  it to the runtime caller. Decision: this is acceptable; no change
  needed at this site. If a follow-up bead wants to differentiate
  "permanent" vs "transient" failures, that is a separate concern.
- UNKNOWN: Should the `staged_event_keys` HashSet insertion at
  `append_event.rs:119` (currently AFTER the index-op `?`) be moved
  BEFORE the index-op guard so that subsequent appends with the same
  `(run, seq)` are rejected by the same-batch duplicate guard even when
  the prior append failed at G8? Current behavior: `staged_event_keys`
  is NOT populated on G8 failure, so a follow-up same-key append will
  re-pass G1+G2 and only fail at G3 (durable lookup, which won't see
  the staged-but-uncommitted insert). This is an independent question
  — flag for downstream contract owner.
- UNKNOWN: Does `index_status_key` already follow the abort-on-error
  contract? Yes — see `putters.rs:188-204` (`put_status_index`).
  `index_workflow_key` and `index_action_key` likewise (L212-251).
  The bug is specifically in `append_event`'s G8 path which calls
  `stage_pending_action_index_op` (the internal staging helper at
  `batch/action_index.rs:106-126`) WITHOUT setting `aborted = true`
  on its error return.

### Recommended downstream owners

- rust-contract: enumerate the new G8 guard and its `aborted = true`
  post-condition; recommend the Witness enum extension in the spec.
- proof-planner: extend vb-vzcuf-PS-008 and PS-009 with G8; regenerate
  the production mirrors; plan Verus + Kani lane updates.
- test-planner: add `batch_append_event_index_key_error_aborts_commit`
  (mirroring `batch_index_key_error_aborts_commit` at
  `t_putters_b.rs:177`) and a proptest variant that hammers the guard
  with arbitrary `ActionId/RunId/StepIdx` triples to demonstrate the
  abort invariant under all KeyCapacity-reachable inputs.
- holzman-rust: implement the fix at
  `crates/vb_storage/src/batch/append_event.rs:114-115` by replacing
  the `?` with a `match` that sets `self.aborted = true` on Err and
  returns the typed error; update the doc-comment's Guard Precedence
  (C6) section to add G8 and the Postconditions (ensures) section to
  document the new abort invariant.

