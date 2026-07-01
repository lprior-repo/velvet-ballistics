# Codebase Map — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-r8oso
**captured_at:** 2026-07-01
**controller:** femdation

## 1. Problem Statement (verbatim)

> Storage append path may write a journal event with a sequence that doesn't match `next_sequence_at_write()`. Enforce strict sequence check at append time, reject mismatch with typed error, no silent rewrite.

The bead description in `bd show vb-r8oso` further refines the audit finding:

> `append_strict` can durably ACK sequence gaps that only fail later during replay or recovery.

In other words: today the storage append path accepts any `JournalEvent.seq()` value. The only write-time check is `events.contains_key(key)` for duplicate-key detection, which does NOT reject `seq` values that skip ahead (gaps) or repeat an already-claimed `seq` after a partially-failed commit. The current behavior writes the event durably, then surfaces the inconsistency only at replay / recovery time as `JournalError::SequenceGap`.

## 2. Scope Boundary

The fix MUST be a write-time guard in `crates/vb_storage`. It MUST NOT silently rewrite `event.seq()` — the bead explicitly forbids silent rewrite. It MUST reject with a NEW typed error variant (currently no write-time sequence-mismatch variant exists).

Out of scope (call-graph verified):

- Trimming (snapshot cutoff) — read-only.
- `RecoveryRuntimeSummary::last_seq` derivation — read-only.
- Verus/Flux proofs — must be re-bound only if a new public API surfaces; the new method is read-only on the durable log so existing proofs remain applicable.
- Runtime-side sequence allocation (`journal_sequence_for` on the runtime shard) — the *caller* of storage. The storage fix is defense-in-depth, not a substitute for the in-memory allocator.

## 3. Production Surface — Append Entry Points (touched by fix)

All five append paths in `vb_storage` write a `JournalEvent` with a caller-supplied `seq()` and currently perform only a `contains_key(key)` duplicate check.

### 3.1 `crates/vb_storage/src/journal/append.rs`

```rust
pub fn append_journaled(&self, event: &JournalEvent) -> Result<(), JournalError>      // line 7
pub fn append_strict(&self, event: &JournalEvent) -> Result<(), JournalError>          // line 35
pub fn append_strict_batch(&self, events: &[JournalEvent]) -> Result<(), JournalError> // line 69
pub fn persist_strict(&self) -> Result<(), JournalError>                               // line 81 (durability barrier only)
```

Today each of these calls into the lower-level `append_unfsynced` / `JournalWriteBatch::append_event`, neither of which validates `event.seq()` against the durable tail for the run.

### 3.2 `crates/vb_storage/src/journal/internal.rs`

```rust
pub(crate) fn append_unfsynced(&self, event: &JournalEvent) -> Result<(), JournalError>   // line 50
pub(crate) fn append_queued_unfsynced(&self, event: &JournalEvent) -> Result<(), JournalError> // line 91 (test-only)
```

`append_unfsynced` is the lowest-level write: it builds the Fjall key from `(run_id, seq)`, checks `events.contains_key(key)` for duplicate, encodes the record, and commits the batch (event + pending-action-index mutation) atomically. **This is where the new `next_sequence_at_write` guard must be inserted** so all higher-level entry points inherit it.

### 3.3 `crates/vb_storage/src/batch/append_event.rs`

```rust
pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> // line 42
```

The batch path's `append_event` is reached by `append_strict`, `append_strict_batch`, and (via `internal.rs`) `append_unfsynced`. It currently performs same-batch `HashSet<key>` duplicate detection plus a durable `events.contains_key(key)` check, but no sequence-tail check. The guard must also live here so the batch and the strict/journalled single-event paths agree.

### 3.4 Public re-exports

- `crates/vb_storage/src/public_api.rs:50 pub fn append_journal_event(journal, event) -> journal.append_journaled(event)` — public wrapper that must inherit the guard.
- `crates/vb_storage/src/journal/readonly.rs:70 pub fn events_for_run(&self, run)` — read-side; unchanged by the fix but used by the new `next_sequence_at_write` helper.

## 4. New Public API the Fix Must Introduce

### 4.1 `FjallJournal::next_sequence_at_write(run: RunId) -> Result<EventSeq, JournalError>`

Required semantics:

- Returns `EventSeq::ZERO` (i.e., `0`) for a run with **zero** stored events. The next event for a fresh run is conventionally `seq = 0`.
- Otherwise returns `last_durable_event_seq(run).succ()`, where `last_durable_event_seq` is the maximum seq currently present in the `events` keyspace for this run.
- Returns `JournalError::SequenceOverflow` (existing variant) if `succ()` overflows `u64::MAX`.
- The lookup MUST be key-only (max-seq prefix `next_back()`) to avoid decoding every event value, mirroring `latest_durable_snapshot_seq` at `crates/vb_storage/src/trimming/logic.rs:26`.

### 4.2 New `JournalError::SequenceMismatch { run: RunId, expected: EventSeq, actual: EventSeq }`

To be added in `crates/vb_storage/src/error/mod.rs`. Diagnostic code to be assigned from the `0x404x` reserved block (sibling of `REPLAY_KEY_MISMATCH_CODE = 0x4040` and `REPLAY_ENVELOPE_SEQUENCE_MISMATCH_CODE = 0x4041` already in `crates/vb_storage/src/error/codes.rs`). Symbolic code: `JOURNAL_SEQUENCE_MISMATCH_AT_WRITE`.

The error is fundamentally **different** from `SequenceGap { expected, actual }` (which is read-time / replay-only):

- `SequenceGap` is reported by `events_for_run` when the *durable* tail has a missing seq; it can also fire when a stale out-of-order seq was already accepted (current bug).
- `SequenceMismatch` is reported by the write path at the moment of the offending write; it prevents the gap from ever being durably created.

Both must remain in the error enum; the bug fix adds `SequenceMismatch` rather than overloading `SequenceGap` because the call site differs (write vs. read) and the diagnostic semantics differ (caller misbehaviour vs. on-disk corruption).

## 5. Existing Helpers to Reuse

| Helper | Location | Used For |
|---|---|---|
| `codec::next_seq(seq)` | `crates/vb_storage/src/codec/mod.rs:153` | `succ()` with `SequenceOverflow` mapping |
| `keys::run_event_key(run, seq)` | `crates/vb_storage/src/keys.rs:81` | key construction |
| `keys::run_prefix_key(run)` | `crates/vb_storage/src/keys.rs` (used by `trimming/logic.rs:60`) | Fjall `prefix()` iterator range |
| `trimming::logic::latest_durable_snapshot_seq` | `crates/vb_storage/src/trimming/logic.rs:26` | Reference pattern: `prefix().next_back()` key-only lookup |
| `JournalWriteBatch::append_event` C6 guard precedence | `crates/vb_storage/src/batch/append_event.rs:18-26` | Where to insert the new guard (after step 4 "durable duplicate check" or before step 5 "count capacity") |
| `codec::decode_journal_event` | `crates/vb_storage/src/codec/mod.rs` (used in `internal.rs:99`) | If `next_sequence_at_write` decodes the tail key for validation, use this |

## 6. Existing Tests That Demonstrate the Bug

These tests currently encode the **broken** behavior. Implementation must update them to expect the new typed error, and add at least one new positive test that proves the guard fires.

| File:Line | Test | Current (broken) behaviour |
|---|---|---|
| `crates/vb_storage/src/tests.rs:1737` | `append_strict_rejects_out_of_order_sequence` | Asserts `append_strict(&event2 with seq=2)` after seq=0 succeeds, then `events_for_run` returns `SequenceGap`. Under the fix, the `append_strict(&event2)` call must itself return `SequenceMismatch`. |
| `crates/vb_storage/src/tests.rs:4585` | `adversarial_append_duplicate_sequence_rejected_with_exact_fields` | Uses `append_journaled`. Continues to assert `DuplicateEvent` (unchanged by fix; the guard sits *above* the duplicate check). |
| `crates/vb_storage/src/tests.rs:4612` | `adversarial_read_events_with_sequence_gap_returns_exact_gap` | Same pattern as above; still asserts `SequenceGap` on `events_for_run` but the underlying `append_journaled(seq=5)` after seq=0 must now fail with `SequenceMismatch`. |
| `crates/vb_storage/src/journal/tests.rs:409` | `append_strict_rejects_duplicate_event` | Unchanged; duplicate check still fires. |

## 7. Test Additions Required for Closure

A new behavior test suite MUST cover:

1. **`append_strict_rejects_sequence_skipped_with_typed_error`** — append seq=0, attempt `append_strict(seq=2)`, assert `Err(JournalError::SequenceMismatch { run, expected: 1, actual: 2 })`. Mirror for `append_journaled` and `append_strict_batch`.
2. **`append_strict_rejects_sequence_at_zero_for_run_with_history`** — append seq=0..5, attempt `append_strict(seq=0)` (rewind), assert `SequenceMismatch { expected: 6, actual: 0 }`.
3. **`append_strict_accepts_first_seq_for_fresh_run`** — fresh journal, `append_strict(seq=0)` succeeds (the guard is `next_sequence_at_write() == 0`).
4. **`next_sequence_at_write_returns_zero_for_fresh_run`** — public API contract.
5. **`next_sequence_at_write_returns_last_plus_one_after_writes`** — public API contract.
6. **`append_strict_batch_rejects_on_first_mismatch_atomically`** — batch with `[seq=3, seq=4]` after seq=2 stored; whole batch rejected with `SequenceMismatch`; `events_for_run` shows no new events written.
7. **`append_unfsynced_uses_next_sequence_at_write_guard`** — same guard reachable via the lower-level path (covers `runtime::append_sequenced`).
8. **Kani-style harness (optional, gate-gated)** — `next_sequence_at_write` over a stored run prefix returns `last_seq.succ()` or `0`. Likely lives under a new `kani-sequence-at-write` Cargo feature in `crates/vb_storage/Cargo.toml` per AGENTS.md kani-harness-isolation rule.

## 8. Public API / Re-export Audit

The new `next_sequence_at_write` method is `pub fn` on `FjallJournal`. It needs to be re-exported in:

- `crates/vb_storage/src/journal/mod.rs` (already has `pub use self::core::{EventReplayLimit, FjallJournal};` at line 26 — new method is auto-available since it's `impl FjallJournal`).
- `crates/vb_storage/src/public_api.rs` — adds `pub fn next_sequence_at_write(journal, run) -> Result<EventSeq, JournalError>` wrapper for consistency with `append_journal_event`.

No `pub use` change needed in `lib.rs`; the type `FjallJournal` is already exported at line 219.

## 9. Callers (Downstream Surface)

Producers of `JournalEvent` that pass a `seq` value into the storage append path. Implementation must NOT change these; the storage guard is defense-in-depth.

| File | Function | seq source |
|---|---|---|
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:194` | `append_journal_event(event)` | `self.journal_sequence_for(run)` (in-memory allocator) |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:224` | `journal_sequence_for(run)` | `self.journal_sequences.get(&run).copied().unwrap_or(EventSeq::ZERO)` |
| `crates/vb_runtime/src/engine/action.rs:150` | action completion path | local `next_seq` calculation from in-memory state |
| `crates/vb_cli/src/lifecycle.rs:106,191,272,366` | CLI cancel/lifecycle | `journal.events_for_run(...).last().map(seq)` |
| `crates/vb_cli/src/run_cancel_ops.rs:42` | CLI cancel ops | `events.last()` |
| `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs:133` | test helper `append_event` | calls `vb_storage::append_journal_event` with caller-supplied seq |
| `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs:501` | test helper `append_event` | same |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` and 9 sibling test files | various test helpers | direct `append_journaled` / `append_strict` with caller-supplied seq |

These callers must remain correct under the fix: their per-run in-memory counter is the *intended* `next_sequence_at_write` value, so the storage guard will accept their writes and reject only real bugs (e.g., race between two callers, replay-driven append after a crash, replay test that supplies out-of-order seq).

## 10. Error Code Registry Updates

`crates/vb_storage/src/error/codes.rs`:

- Add `pub const SEQUENCE_MISMATCH_AT_WRITE_CODE: DiagnosticCode = DiagnosticCode::new(0x4042);` (next free `0x404x` slot).
- Add match arm in `diagnostic_code()`: `Self::SequenceMismatch { .. } => Self::SEQUENCE_MISMATCH_AT_WRITE_CODE,`
- Add match arm in `symbolic_code()`: `Self::SequenceMismatch { .. } => "JOURNAL_SEQUENCE_MISMATCH_AT_WRITE",`

The symbolic code MUST be added to the `CODE_REGISTRY` (location TBD — search `SymbolicCode::from_static`). Verify by running `bash scripts/check-journal-error-codes.sh` (or equivalent) post-change.

## 11. Kani / Verus / Flux / proptest Coverage Plan

Per AGENTS.md "Differential verification" and "no blind verification mutations" rules, scope-trim the verification lane to the call-graph blast radius of this bead (one new read-only helper + one new error variant + four call sites in `vb_storage`):

| Lane | Action | Location |
|---|---|---|
| Kani | Add a `next_sequence_at_write` harness group behind feature `kani-sequence-at-write`. Mirror pattern of `kani-vb-vzcuf` (already in `Cargo.toml`). Harness: prefix iterator returns the right max-seq under small trees (0, 1, 3, 7 events). | new file `crates/vb_storage/src/kani_sequence_at_write.rs`, feature in `Cargo.toml` |
| Verus | None new. Existing `recovery_types_spec.rs` binds `RecoveryRuntimeSummary::last_seq`; it remains consistent because `last_seq` is computed by replay, not by `next_sequence_at_write`. |
| Flux | None new. No new refinement boundary. |
| proptest | Add `proptest_journal_sequence_at_write` covering: random sequence of appends, asserting the prefix of stored events matches `[0, 1, ..., n-1]` for `n` accepted writes, and any attempt to insert `seq != n` yields `SequenceMismatch`. Mirrors pattern in `crates/vb_storage/tests/proptest_journal_error_codes.rs`. |
| fuzz | Update `fuzz/tests/proptest_journal_error_exhaustiveness.rs:106` arm list to include `JournalError::SequenceMismatch { .. }`. Update `fuzz/src/journal_target/errors.rs:46` arm list. Update `fuzz/fuzz_targets/journal_decode.rs:126` and `fuzz/fuzz_targets/decode_record.rs:119` arm lists. |

## 12. Risk Tags

| Tag | Justification |
|---|---|
| **persistence** | A new write-time guard durably rejects sequences. If the guard is wrong, it could reject legitimate writes after a crash-replay path. |
| **public API** | Adds one `pub fn` on `FjallJournal` and one `JournalError` variant; both reach downstream crates. |
| **concurrency** | The guard must be atomic relative to the same-batch `HashSet<key>` check and the Fjall `contains_key` check. `next_sequence_at_write` itself does not need a write lock if it uses a key-only prefix iterator (Fjall LSM tree consistency). |
| **migration** | None. New error variant is additive. |
| **performance** | `next_sequence_at_write` adds one Fjall `prefix().next_back()` lookup per write (single key read, no decode). Negligible vs. existing durability barrier. |

## 13. Existing Proof / Test Bridges That Must Remain Intact

- `crates/vb_storage/src/kani_record_kind.rs:295,326,342` — PO-KANI-005-H1/H2/H3 cover replay sequence contiguity and duplicate detection. Unaffected.
- `crates/vb_storage/verification/verus/recovery_types_spec.rs:50` — `recovery_runtime_summary_inv` (first_seq <= last_seq, etc.). Unaffected; the new error variant is write-time, not part of the summary.
- `crates/vb_storage/src/kani_hydrate_proofs.rs:234,247,264,281` — PO-VB-STORAGE-013/014/015 cover `hydrate_snapshot_tail_seq_after_snapshot`. Unaffected.

## 14. Open Questions / UNKNOWN

- **UNKNOWN**: Is there a write-side flow that legitimately appends a non-contiguous seq? E.g., a recovery path that writes a recovered `RunAccepted` event with the original seq. The audit (per bd description) implies no such flow exists, but the implementer must audit `runtime::replay_resume`, `runtime::hydrate_run_frame`, and `storage::recovery::recover_full_journal` for any caller that writes rather than reads. **Action**: implementer greps `append_journaled|append_strict|append_unfsynced|append_event` across `crates/vb_runtime` and `crates/vb_storage::recovery` and reports any caller that supplies an `event.seq()` not derived from a fresh counter before closing the bead.
- **UNKNOWN**: Whether `next_sequence_at_write` should be infallible when no events exist (returning `EventSeq::ZERO`) or should it explicitly return `None` and force callers to compare against `EventSeq::ZERO`. Recommend infallible-with-default for ergonomic parity with `journal_sequence_for` on the runtime shard.

## 15. Recommended Downstream Owners

| Stage | Owner | Notes |
|---|---|---|
| Contract (rust-contract) | TBD | One new method, one new error variant. Document in `contracts/vb_storage.md`. |
| Test plan (test-planner) | TBD | Eight new behavior tests + updated tests listed in §6/§7. |
| Implementation (holzman-rust) | TBD | Insert guard in three call sites: `append_unfsynced`, `JournalWriteBatch::append_event`, and re-derive `latest_durable_event_seq`. |
| Proof plan (proof-planner) | TBD | One Kani feature-group harness + proptest addition. |
| Black-hat / truth-serum | TBD | Must verify (a) no silent rewrite; (b) error variant carries `{run, expected, actual}` with exact fields; (c) raw evidence from crash-reopen test that the new guard catches the regression. |

## 16. Acceptance Gate (per bead §4)

Per bead Section 4 — `cargo test -p vb_storage --lib -- --nocapture` plus `moon ci`. Targeted closures (cheaper than full `moon ci`) are acceptable for the in-workspace path; final evidence pack must include raw stdout/stderr and exit status of:

```bash
cargo test -p vb_storage --lib append_strict_rejects_sequence_skipped
cargo test -p vb_storage --lib next_sequence_at_write
cargo test -p vb_storage --lib --features kani-sequence-at-write
cargo test -p vb_storage --test proptest_journal_error_codes
moon run :nightly-feature-gate
```

`moon ci` is the canonical final gate.

## 17. Files Map Summary

| Path | Role | Touched by fix |
|---|---|---|
| `crates/vb_storage/src/journal/append.rs` | Public append entry points | Yes (insert guard delegation) |
| `crates/vb_storage/src/journal/internal.rs` | `append_unfsynced` — lowest-level write | Yes (primary guard insertion) |
| `crates/vb_storage/src/batch/append_event.rs` | Batch path's per-event append | Yes (guard consistency) |
| `crates/vb_storage/src/journal/mod.rs` | Module index + re-exports | Possibly (new helper visibility) |
| `crates/vb_storage/src/public_api.rs` | Public wrappers | Yes (optional convenience wrapper) |
| `crates/vb_storage/src/error/mod.rs` | `JournalError` enum | Yes (new variant) |
| `crates/vb_storage/src/error/codes.rs` | Diagnostic + symbolic codes | Yes (new entries) |
| `crates/vb_storage/src/keys.rs` | Key construction | Unchanged (reused) |
| `crates/vb_storage/src/codec/mod.rs` | `next_seq`, decode helpers | Unchanged (reused) |
| `crates/vb_storage/src/trimming/logic.rs` | Reference pattern: `latest_durable_snapshot_seq` | Unchanged (read-only reference) |
| `crates/vb_storage/src/tests.rs` | Existing tests demonstrating bug | Yes (assertion updates + new tests) |
| `crates/vb_storage/src/journal/tests.rs` | Journal-level tests | Possibly (new test additions) |
| `crates/vb_storage/src/error_code_tests.rs` | Code-registration tests | Yes (extend with new code) |
| `crates/vb_storage/src/error_tests.rs` | Variant display tests | Yes (extend) |
| `crates/vb_storage/src/lib.rs` | Module visibility | Possibly (new kani feature) |
| `crates/vb_storage/Cargo.toml` | Cargo features | Yes (new `kani-sequence-at-write` feature, optional) |
| `crates/vb_storage/tests/proptest_journal_error_codes.rs` | Code-level proptest | Yes (exhaustiveness update) |
| `crates/vb_storage/src/tests.rs` (§6 list) | Existing tests must update | Yes (assertion update) |
| `fuzz/src/journal_target/errors.rs` | Fuzz error-class matrix | Yes (add new variant) |
| `fuzz/fuzz_targets/journal_decode.rs` | Fuzz harness arm list | Yes (add new variant) |
| `fuzz/fuzz_targets/decode_record.rs` | Fuzz harness arm list | Yes (add new variant) |
| `fuzz/tests/proptest_journal_error_exhaustiveness.rs` | Proptest exhaustiveness | Yes (add new variant) |
| `crates/workspace_tests/tests/proptest_error_types_registration.rs` | Cross-crate error registration | Yes (add new variant) |
| `crates/workspace_tests/tests/proptest_error_types_nonzero_codes.rs` | Cross-crate code non-zero | Yes (add new variant) |

## 18. Excluded Paths (verified not in scope)

- `crates/vb_storage/src/recovery/**` — replay-time; only reads `events_for_run`, no write.
- `crates/vb_storage/src/trimming/**` — read-only snapshot+event iteration.
- `crates/vb_storage/src/admission/**` — admit path writes a `RunHeaderRecord` and `CompiledIrRecord`, not a `JournalEvent`; out of scope.
- `crates/vb_storage/src/preview/**` — read-only doctor preview.
- `crates/vb_storage/src/queue/**` — `JournalWriterQueue` delegates to `journal.flush_batch(journal)` which calls the append paths above. No direct edit; the queue's batching behaviour continues to be subject to the same guard via the lower-level calls.
- `crates/vb_runtime/src/journal/**` — runtime-side `append_sequenced` is unchanged; it calls the storage append which now guards.
- `crates/vb_storage/verification/verus/**` — no spec change; existing recovery summary specs unaffected.
