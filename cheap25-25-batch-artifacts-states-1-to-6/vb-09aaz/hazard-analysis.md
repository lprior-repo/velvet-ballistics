# Hazard Analysis: vb-09aaz — Abort Batch on All Index Key Construction Failures

## H1 Partial Persistence (Master §49 Crash-Consistency Violation)

- **Hazard**: `JournalWriteBatch::append_event` stages the journal event into `self.inner` at append_event.rs:104 BEFORE calling `stage_pending_action_index_op` at L114-115. If the index-key construction step fails (returns `Err(JournalError::KeyCapacity)`), the `?` propagates the typed error WITHOUT setting `self.aborted = true`. A subsequent caller-driven `commit()` then persists the partially-staged batch: the journal event is durable, but the pending-action index mutation is not. The pending-action cursor is then inconsistent with the durable event log.
- **Detection**: any `append_event` call where the event variant implies a pending-action-index mutation AND `index_action_key` returns `Err(KeyCapacity)` AND `commit()` is then called. Under nominal inputs, `index_action_key` always succeeds (the 13-byte fixed-length encoding fits exactly), so the hazard is DEFENSIVE in normal operation but CRITICAL if it ever fires.
- **Contract control**: every fallible step in `append_event` MUST set `self.aborted = true` before propagating the typed error. The G8 step (`stage_pending_action_index_op`) is the missing piece. The fix mirrors the canonical pattern from `putters.rs:188-200, 212-223, 235-247` (28 occurrences total).
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-001` — Verus spec extension for G8 with abort-on-error invariant.
- **Severity**: P1 (per bead title and codebase-map.md Bug Surface section).

## H2 G8 Path Silent Pass-Through (Pre-Fix Bug)

- **Hazard**: the `?` operator at append_event.rs:114-115 silently passes through any error from `stage_pending_action_index_op` without updating batch state. The contract requires that ALL fallible steps set `aborted = true` on Err. The pre-fix code violates this invariant for the G8 path.
- **Detection**: code review of append_event.rs:114-115 against the canonical pattern in putters.rs (28 occurrences of `self.aborted = true`).
- **Contract control**: the fix replaces the `?` with a `map_err` (or equivalent `match`) that sets `aborted = true` first. The doc-comment is updated to enumerate G8 in the Guard Precedence (C6) section and document the new post-condition.
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-002` — Kani/proptest regression test that exercises G8 in abort mode.

## H3 Verifier Spec Drift (GOD RULE 2 Binding)

- **Hazard**: the Verus production mirrors at `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs:78-95` and `_PS_009_production.rs:67-93` enumerate the 7-guard order and declare `JournalError::KeyCapacity` as unreachable in the mirror (PS-008 L174, PS-009 L168-171). With G8 added, `KeyCapacity` becomes reachable from `stage_pending_action_index_op`. The mirrors must be regenerated to match production.
- **Detection**: `bash scripts/check-verus-production-binding.sh` and `bash scripts/check-production-inner-drift.sh` per AGENTS.md mandatory gates.
- **Contract control**: the proof-writer regenerates the mirrors to add a new G8 step (analogous to the existing G5 `encode_ok` abstraction). Recommended approach: add a new `index_key_ok: bool` exec arg to the mirror's `append_event` and a new G8 step that mirrors the abort-on-error pattern.
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-003` — Verus mirror regeneration with G8 guard.

## H4 Guard Precedence (C6) Doc Drift

- **Hazard**: the doc-comment at append_event.rs:18-26 lists 8 guards but only enumerates 7 (it stops at step 7 "Insert into inner OwnedWriteBatch"). The G8 step is not enumerated. The Postconditions (ensures) doc-comment at L33-41 lists the abort-on-error clause for `DuplicateEvent` but not for `KeyCapacity`.
- **Detection**: code review of append_event.rs doc-comment against the canonical Guard Precedence (C6) contract.
- **Contract control**: the fix updates the doc-comment to enumerate G8 alongside G1..G7 and adds a new bullet for the KeyCapacity abort post-condition. This is a documentation-only change.
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-004` — doc-comment update verification (low priority; main proof is the regression test).

## H5 Staged-Event-Keys Insertion Order (Open Domain Decision)

- **Hazard**: `staged_event_keys.insert(key)` at append_event.rs:119 happens AFTER the G8 `?`. If G8 fires, the key is NOT in `staged_event_keys`. A subsequent `append_event` call with the same `(run, seq)` will re-pass G1+G2 (same-batch duplicate check is empty) and only fail at G3 (durable lookup, which won't see the staged-but-uncommitted event). If G8 succeeds on retry (e.g., transient failure), the event commits normally. If G8 fails again, the batch aborts again.
- **Detection**: code review of append_event.rs:119.
- **Contract control**: the contract RECOMMENDS moving `staged_event_keys.insert(key)` to immediately before the G8 call (after `inner.insert` succeeds at G7). This is an open domain decision flagged in `domain-model.md` and `workflow-model.md`. The current behavior is acceptable for the vb-09aaz fix.
- **Proof seed**: this is a follow-up decision, not a contract requirement. May be addressed in a future bead.

## H6 Queued-Writer Path Behavior (Review Only)

- **Hazard**: `JournalWriterQueue::stage_queued_event` (queue/writer/stage.rs:31-74) calls `stage_pending_action_index_op` at L72 with the same `?` propagation pattern. The batch is single-shot (OwnedWriteBatch is dropped on Err, never committed). There is no partial-write hazard at this site.
- **Detection**: code review of queue/writer/stage.rs and queue/writer.rs.
- **Contract control**: the contract explicitly does NOT require fixing the queued-writer path. The current behavior — drop the OwnedWriteBatch on Err, surface the typed error to the runtime — is correct under the no-partial-write policy.
- **Proof seed**: not required for vb-09aaz.

## H7 Direct Path (`append_unfsynced`) Behavior (Review Only)

- **Hazard**: `FjallJournal::append_unfsynced` (journal/internal.rs:50-79) builds a fresh `OwnedWriteBatch` at L74 and commits it at L77. On index-op failure at L76, the batch is dropped, no commit. There is no partial-write hazard at this site.
- **Detection**: code review of journal/internal.rs.
- **Contract control**: the contract does not require fixing this path.
- **Proof seed**: not required for vb-09aaz.

## H8 Concurrency / Async Hazards

- **Hazard**: none. `JournalWriteBatch` is `!Send + !Sync` via `PhantomData<*mut FjallJournal>` (types.rs:18-21). The abort-on-fallible-step invariant is local to one batch handle; no cross-thread aliasing is possible.
- **Detection**: type system (`PhantomData<*mut FjallJournal>` is `!Send + !Sync`).
- **Contract control**: no action required.
- **Proof seed**: not required. No loom model is necessary.

## H9 Test Coverage Gap

- **Hazard**: existing tests in `crates/vb_storage/src/batch/t_append_event.rs` (13 tests) cover happy path, duplicate-event, invalid-event, monotonic len, all-or-nothing commit, and digest verification. NONE of them exercise the G8 KeyCapacity failure path. The 19 tests in `index_maintenance_tests.rs` cover happy-path index maintenance across `append_journaled`, `append_strict`, `append_strict_batch`, `JournalWriteBatch::append_event`, and `JournalWriterQueue::drain_all` but NONE exercise the KeyCapacity failure path either.
- **Detection**: code review of test files.
- **Contract control**: the test-planner MUST add a new test `batch_append_event_index_key_error_aborts_commit` in `batch/t_append_event.rs` that mirrors the existing `batch_index_key_error_aborts_commit` at `batch/t_putters_b.rs:177-209`. The test must assert:
    1. `append_event` returns `Err(JournalError::KeyCapacity)`.
    2. `batch.is_aborted() == true`.
    3. `batch.commit()` returns `Err(JournalError::BatchAborted)`.
    4. No events are durable for the run.
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-005` — proptest that hammers G8 with arbitrary `ActionId/RunId/StepIdx` triples.

## H10 Performance / Migration

- **Hazard**: the fix is a 1-line code change (replacing `?` with `map_err`). No new methods, no new fields, no new error variants. No performance regression expected.
- **Detection**: N/A — trivial change.
- **Contract control**: no action required.
- **Proof seed**: not required. No benchmark is necessary.

## H11 Public API Migration

- **Hazard**: the fix changes the post-condition of `append_event` but NOT the signature. Callers that match on `JournalError::KeyCapacity` continue to compile. The doc-comment adds a new bullet for the G8 post-condition.
- **Detection**: N/A — signature is unchanged.
- **Contract control**: no action required for callers. The doc-comment update is internal.
- **Proof seed**: not required.

## H12 Verifier Spec Mirror Regeneration Race

- **Hazard**: the production-mirror regeneration at
  `verification/verus/production_inner/vb_vzcuf_PS_008_production.rs`
  and `_PS_009_production.rs` is a manual regeneration step. If
  the drift-gate header (`L5-14` for PS-008, `L5-32` for PS-009)
  is not honored, the mirrors will drift from production and the
  Verus `assume_specification` bridge will fail to verify.
- **Detection**: `bash scripts/check-verus-production-binding.sh` and `bash scripts/check-production-inner-drift.sh`.
- **Contract control**: the proof-writer MUST regenerate both mirrors as part of the vb-09aaz fix. The drift-gate header is the binding contract; this contract relies on the AGENTS.md mandatory gates.
- **Proof seed**: `proof-seeds.jsonl:vb-09aaz-PS-003` — Verus mirror regeneration with G8 guard (same as H3).