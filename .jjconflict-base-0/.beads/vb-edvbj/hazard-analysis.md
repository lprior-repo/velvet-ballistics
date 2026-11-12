# Hazard Analysis — vb-edvbj

## H-1 — Temporal / recovery correctness (RESUMED replay classification)

**Class:** temporal workflow / replay hazard.
**Risk tags:** `temporal`, `persistence`, `recovery`.

The buggy fallback at `chunk_002.rs:295-302` causes `RuntimeJournalEvent::Resumed { run, timestamp }` to be persisted to Fjall as `JournalEvent::RunFailedEvent { run, seq, attempt: 1 }`. Recovery code at:

- `crates/vb_storage/src/recovery/replay/observation/normalize.rs:126-127`
- `crates/vb_storage/src/journal/incident.rs:203` (`JournalEvent::RunResumed { .. } => LifecycleState::Active`)

depends on the run being stored with the correct terminal-class classification. The bug mis-classifies successful resumes as failures — i.e. a successful recovery becomes indistinguishable from a real failure on subsequent replays.

After the fix (Option A only), `Resumed` events through `StorageRuntimeJournal` will surface `Err(UnmappedRuntimeJournalEvent)` and **NOT** be persisted at all. Recovery that depends on `Resumed` reaching storage therefore now errors out, which is correctly typed (the operator sees a clear `unmapped runtime journal event: Resumed` error) but the recovery path is *not yet* restored to the pre-bug semantics. This is a known regression introduced by Option A that Option B (adding an explicit `Resumed → RunResumed` mapping to `boundary_storage_event`) was meant to address.

**Mitigation (deferred to a follow-up bead — out of scope for vb-edvbj):** add `Resumed { run, timestamp } → Some(JournalEvent::RunResumed { run, seq, timestamp: DateTime::<Utc>::from_timestamp(timestamp as i64, 0).single().ok_or(RuntimeError::EncodeFailed)? })` in `boundary_storage_event`. Until that follow-up lands, **storage-journaled** resume flows are blocked at `append_sequenced`. **Volatile-journaled** resume flows are unaffected (the bug does not reach `VolatileRuntimeJournal::append`).

**Proof lane:** temporal-replay lane, with Verus + Kani harnesses for the storage-event path (see `proof-seeds.jsonl`).

## H-2 — Diagnostic-code collision (pre-existing, latent)

**Class:** public-API / diagnostic correctness.
**Risk tags:** `public_api`, `telemetry`.

`crates/vb_runtime/src/error/diagnostics.rs` already registers `0x201F` for **two** variants: `ADMISSION_CAPABILITY_COUNT_MISMATCH_CODE` (line 33) and `INTROSPECTION_EPOCH_EXHAUSTED_CODE` (line 44). This is a pre-existing latent bug not in scope for vb-edvbj. This bead reserves `0x2020` for `UNMAPPED_RUNTIME_JOURNAL_EVENT_CODE`, deliberately stepping past the duplicate to avoid making the collision worse.

**Mitigation:** downstream proof-review / black-hat reviewer MUST surface this as a `finding/v1` row with `disposition: blocker` or open a separate bead. Do not silently reuse `0x2020` for a different purpose; do not modify lines 33 / 44 in this bead.

**Proof lane:** none for this bead (the collision is being passed through, not introduced).

## H-3 — Verus mirror drift

**Class:** verifier binding correctness.
**Risk tags:** `verification_artifacts`.

The Verus mirror `verification/verus/extern_storage_kind_family.rs` re-implements `MirrorJournalEvent::RunResumed { run, seq }` (line 370-373 of the mirror file). This mirror binds the production `JournalEvent::RunResumed { run, seq, timestamp: DateTime<Utc> }` (production `events.rs:290-297`). The fix does NOT change the production shape; therefore the mirror remains binding.

However, the drift check at the bottom of the mirror file (`prod_methods_drift_check_mirror`, lines 670-695) does not currently exercise the `UnmappedRuntimeJournalEvent` path — it is a runtime-only error, not a storage event. No mirror update is required.

**Mitigation:** proof-writer MUST run `bash scripts/check-verus-production-binding.sh` after the implementation lands; the mirror's `journal_replay` / `kind_family` proofs are unchanged. This bead REQUIRES the gate to be re-run (not skipped).

**Proof lane:** Verus production-binding gate. Not a behavior change of the spec.

## H-4 — Type-safety: `&'static str` vs runtime-allocated identifier

**Class:** type contract / API stability.
**Risk tags:** `public_api`, `parser/codec`.

The contract dictates `event_kind: &'static str`. This is safe **only** if the dispatcher knows the variant as a literal identifier at compile time. Since `RuntimeJournalEvent` is `#[non_exhaustive]`, a future added variant could be unhandled if the dispatcher maintains a hand-rolled `match` on `&event` for `event_kind`. If the dispatcher pattern-matches `&event` to map to a `&'static str` and adds new variant arms only when the helpers are updated, this is correct; if the dispatch defaults to `"Unknown"` for any new variant, the future variant fires `UnmappedRuntimeJournalEvent { event_kind: "Unknown" }` which is non-debuggable.

**Mitigation:** the implementation MUST add an explicit arm to `runtime_journal_event_kind(&event)` per variant. The dispatcher's `match &event` for variant classification MUST be the SAME match as the helpers, ensuring that adding a variant forces all helpers and the dispatcher to compile together. A test (out of scope for this bead, but downstream worth requesting) MUST enumerate every variant and assert `event_kind != "Unknown"`.

**Proof lane:** Rust-local; Verus refinement on the variant-name mapping function (see `proof-seeds.jsonl` `RUNTIME-EVENT-KIND-COMPLETENESS`).

## H-5 — `Option::None` propagation across per-layer helpers

**Class:** Rust core invariant / bounded state.
**Risk tags:** `parser/codec`, `public_api`.

Each layer helper returns `Option<JournalEvent>`; today, all three return `None` only for `Resumed`. After the fix, the dispatcher's `None` propagation becomes a typed-error trigger. This is intentionally tight, but it relies on the invariant "all three helpers return `Some` for non-`Resumed` variants" holding. If a future variant is added and the dispatcher routes it to a layer that returns `Some` while another layer would have returned `Some` too, the dispatch layer is well-defined; if the routing logic is wrong, multiple helpers may each produce a `Some` and the dispatcher must still pick one — exactly today (the dispatcher picks one based on the `match &event` arm).

**Mitigation:** the existing `clone_for_dispatch` test counter (`STORAGE_EVENT_CLONE_COUNT`) at `chunk_002.rs:310-312` MUST continue to enforce "exactly one clone per dispatch" after the fix.

**Proof lane:** Rust-local Kani harness on `storage_event` that asserts at most one helper call per dispatch.

## H-6 — Existing test parity

**Class:** test-coverage regression risk.
**Risk tags:** `test-coverage`, `public_api`.

Tests at `crates/vb_runtime/src/journal/tests/chunk_001.rs:73-260`, `chunk_002.rs:1-500`, `chunk_003.rs:1-398`, `chunk_004.rs:0-1206`, and downstream `tests/recovery_*`, `tests/vb_h6ix_*`, `tests/durability_matrix_*` MUST continue to pass after the fix. None hits the buggy fallback (per `codebase-map.md`); all hit explicit helper arms. The fix does not modify any explicit arm. If any of these tests fail, the dispatcher change broke a previously-correct path and MUST be reverted.

**Mitigation:** downstream test-writer MUST add `re_019_resumed_does_not_fabricate_run_failed` (per `delivery-scope.jsonl` row 13). The test asserts `Err(UnmappedRuntimeJournalEvent { event_kind: "Resumed" })` for `Resumed` against a Fjall-backed `StorageRuntimeJournal` and never observes a `RunFailedEvent` for the input `seq`.

**Proof lane:** behavior-test (proptest + unit).

## H-7 — Concurrency / cancellation / async

**Class:** async / concurrency.
**Risk tags:** none.

`storage_event` is synchronous, `&event`-borrowing, no shared state, no `unsafe`, no async. This hazard class is empty for vb-edvbj. Recorded for completeness.

## H-8 — Performance / release

**Class:** performance.
**Risk tags:** `performance`.

The fix replaces a single `match` with `match + Err` allocation-free path. Cost is one extra static-message dispatch (Display) when the error fires; ordinary-path cost is unchanged. No release hazard.

## H-9 — Public-API compatibility

**Class:** release / API.
**Risk tags:** `public_api`.

`RuntimeError` is `#[non_exhaustive]`. Adding `UnmappedRuntimeJournalEvent` is a non-breaking add within a `#[non_exhaustive]` enum. However:

- Existing code that uses `match self { .. }` exhaustively on the full `RuntimeError` will fail to compile until the new arm is added. There are likely several internal matches. The proof-plan-reviewer MUST scan `crates/vb_runtime/src/**/*.rs` for exhaustive `match RuntimeError` and ensure arms are added.
- `RuntimeError` derives `Clone`; the new variant must also clone (it does — `&'static str` is `Copy`).

**Mitigation:** `bash scripts/check-nightly-features.sh` and any internal-completeness check over `match RuntimeError` MUST be re-run before landing.

## H-10 — Recovery-correctness dependency (call-graph blast radius)

The contract focus emphasizes: "recovery correctness depends on no silent fail". This is restated as: under Option A, recovery for storage-journaled runs now surfaces a typed error rather than silently corrupting state. This is correct behaviour. The follow-up Option-B fix is what restores replay coverage. Until that lands, every test that expects `Resumed` to round-trip through `StorageRuntimeJournal` MUST be updated to expect `Err(UnmappedRuntimeJournalEvent)` — both `crates/vb_runtime/tests/durable_resume_red_phase.rs` and any `recovery_*` test that goes through `StorageRuntimeJournal` (not `VolatileRuntimeJournal`).

**Mitigation:** test-writer MUST update the affected tests or scope them to `VolatileRuntimeJournal` only, and add a TODO reference to the Option-B follow-up bead.
