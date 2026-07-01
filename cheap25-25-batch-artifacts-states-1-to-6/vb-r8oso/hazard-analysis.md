# Hazard Analysis — vb-r8oso

**bead_id:** vb-r8oso
**owner_stage:** rust-contract
**upstream_artifacts:** `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`, `boundary-map.md`

This artifact enumerates the temporal, Rust-core invariant, bounded-state,
refinement, concurrency, unsafe/provenance, hostile-input, performance,
release/API, and migration hazards introduced or affected by the fix. Each
hazard is a seed for downstream proof and test planning.

---

## 1. Hazard Categories (Authoritative Index)

| # | Category | Reference |
|---|---|---|
| H-1 | Temporal / sequence ordering | §2 |
| H-2 | Rust-core invariant | §3 |
| H-3 | Bounded state (overflow) | §4 |
| H-4 | Refinement (helper vs. read path) | §5 |
| H-5 | Concurrency / multi-writer | §6 |
| H-6 | Unsafe / provenance | §7 |
| H-7 | Hostile input / Fjall keyspace | §8 |
| H-8 | Performance / hot path | §9 |
| H-9 | Release / API / cross-crate | §10 |
| H-10 | Migration / upgrade | §11 |
| H-11 | Diagnostic / observability | §12 |

---

## H-1. Temporal / Sequence-Ordering Hazards

### H-1.1 Caller-side counter drift after crash recovery

**Description.** A caller maintains an in-memory per-run `EventSeq`
counter. After a process crash + recovery, the in-memory counter is
re-hydrated from the durable tail. If re-hydration reads `events.last().seq()`
*before* the durable fsync barrier commits the latest event, the
in-memory counter could lag behind the durable tail, causing the next
append to send a stale seq.

**Surface.** Today: the stale-seq append succeeds; the durable log now
has a hole, observable as `SequenceGap` at replay. Under the fix: the
stale-seq append returns `Err(SequenceMismatch { run, expected, actual })`
without writing.

**Mitigation.** The storage fix turns the bug into a typed error. The
runtime allocator's re-hydration logic is out of scope; this contract
guarantees that a buggy allocator will fail loudly, not silently. The
runtime stage must wire `next_sequence_at_write` into the allocator's
re-hydration path.

**Proof seed.** See `proof-seeds.jsonl` PS-1.

### H-1.2 Two-call race on the same `(run, seq)`

**Description.** Two callers compute `next = n` simultaneously and both
attempt an append with `seq = n`. The first wins; the second observes
`DuplicateEvent`. (Multi-writer is not currently supported; the
process-level lock serializes callers.)

**Surface.** Pre-fix: latent race hidden because the second's seq matches
the durable tail, so `events.contains_key(key)` returns `true` and
emits `DuplicateEvent`. Post-fix: same surface; the new guard sees
`expected = n+1` after the first commit and emits `SequenceMismatch`,
NOT `DuplicateEvent`. This is a behavioural surface change.

**Mitigation.** Single-writer assumption holds. The test suite must NOT
assume `DuplicateEvent` for a stale-seq append under the fix — the
test at `crates/vb_storage/src/tests.rs:4585` (which is the
duplicate-after-same-seq case) is unaffected because both appends use
the same seq, and the second append's seq (n) does NOT match the
expected (n+1) post-first-commit. Wait: the test uses
`append_journaled(seq=0)` after `append_journaled(seq=0)` succeeded,
which is exactly the duplicate-after-same-seq case. Under the fix,
the second `append_journaled(seq=0)` will get
`SequenceMismatch { expected: 1, actual: 0 }`, NOT `DuplicateEvent`.

**Important.** The test at line 4585 currently asserts `DuplicateEvent`.
Under the fix it must be split: the duplicate-once case
(two writes of seq=0) becomes `SequenceMismatch { expected: 1, actual: 0 }`.
A separate test asserts `DuplicateEvent` for a true retry-after-commit
scenario where the caller's seq DID match the previous attempt's seq
*and* the durable tail had already advanced past it (e.g., a half-committed
batch leaves the seq visible). The implementer must reclassify this test.

**Proof seed.** See `proof-seeds.jsonl` PS-2.

### H-1.3 Out-of-order append across two runs in a single batch

**Description.** `append_strict_batch([event_run_A_seq_n, event_run_B_seq_m])`
with the per-run running state advancing during iteration.

**Surface.** Pre-fix: only duplicates are caught; cross-run seq drift is
silent. Post-fix: the guard fires independently per run; the
batch's `JournalWriteBatch::append_event` accepts on success and aborts
on mismatch. The batch's overall semantics must reject the entire batch
on the first mismatch, regardless of which run it occurred on.

**Mitigation.** The implementation maintains a "last accepted seq per
run" within the batch iteration and consults `next_sequence_at_write`
on each step. The guarantee is documented in `workflow-model.md` §2.3
and `type-contracts.md` §2.4.

**Proof seed.** See `proof-seeds.jsonl` PS-3.

---

## H-2. Rust-Core Invariant Hazards

### H-2.1 `event.seq()` not derived from `next_sequence_at_write`

**Description.** The invariant is "`event.seq() == next_sequence_at_write(run) pre-call`". A caller that supplies a different `seq` violates the invariant.

**Surface.** Pre-fix: durably accepted; later `SequenceGap`. Post-fix: `SequenceMismatch`.

**Mitigation.** The guard is inserted as C6 step 3 in every append path.

**Proof seed.** See `proof-seeds.jsonl` PS-1.

### H-2.2 `next_sequence_at_write` returns `EventSeq::ZERO` for non-existent runs

**Description.** A run with no events MUST yield `ZERO`, not panic.

**Surface.** `prefix().next_back()` returns `None` for an empty range; the implementation MUST map that to `Ok(EventSeq::ZERO)`.

**Mitigation.** Match arm documented in `boundary-map.md` §4.2.

**Proof seed.** See `proof-seeds.jsonl` PS-4.

### H-2.3 Equal `expected` and `actual` for `SequenceMismatch`

**Description.** The `SequenceMismatch { expected, actual }` constructor pre-condition is `expected != actual`. A buggy implementation that emits the variant when `expected == actual` will fail tests.

**Surface.** If `next_sequence_at_write` is called after the durable tail has already advanced, the implementation might observe `actual == expected == seq_of_durable_tail` and emit the variant with `expected == actual`. This is implementation-bug territory; a test must catch it.

**Mitigation.** Tests in `error_tests::sequence_mismatch_constructor_fields` enforce the pre-condition.

**Proof seed.** See `proof-seeds.jsonl` PS-5.

---

## H-3. Bounded-State Hazards

### H-3.1 `EventSeq::MAX` saturation

**Description.** `next_sequence_at_write` calls `codec::next_seq(seq)` which `checked_add`s on `EventSeq::MAX.get() + 1`. The overflow maps to `SequenceOverflow`, not a panic.

**Surface.** When the durable tail of any run reaches `EventSeq::MAX`, `next_sequence_at_write` MUST return `Err(SequenceOverflow)`. The append path then propagates the error to the caller.

**Mitigation.** Use `codec::next_seq` uniformly; do not introduce a separate `+1` site. Test at
`EventSeq::MAX.get()` boundary is in `event_seq_max_panic_safety` style tests (out of scope for this bead but a candidate for `proof-writer`).

**Proof seed.** See `proof-seeds.jsonl` PS-6.

### H-3.2 Empty result of `prefix().next_back()`

**Description.** `None` from the prefix iterator must be mapped to
`EventSeq::ZERO`. A wrong mapping (e.g., returning `EventSeq::MAX`) would
cause fresh runs to advertise a saturated seq — a non-fatal but very
wrong value.

**Surface.** Map `None` to `Ok(EventSeq::ZERO)` and propagate.

**Mitigation.** Test `next_sequence_at_write_returns_zero_for_fresh_run`.

**Proof seed.** See `proof-seeds.jsonl` PS-4.

### H-3.3 Underflow (negative seq)

**Description.** `EventSeq` is `u64`-backed; there is no negative case. The guard does not need to defend against underflow because `u64::checked_add` cannot underflow (it overflows).

**Surface.** N/A; defense-in-depth only.

**Mitigation.** Use `codec::next_seq` only.

---

## H-4. Refinement Hazards

### H-4.1 `next_sequence_at_write` agrees with `events_for_run`'s view

**Description.** Both methods consult the same Fjall keyspace. They MUST observe consistent state under single-writer semantics. Under multi-writer (out of scope), they could observe different snapshots.

**Surface.** If the helper uses a stale snapshot (e.g., due to a forgotten `prefix()` invocation), the append might emit a `seq` already written. The C6 duplicate check is the second line of defense.

**Mitigation.** Both methods use the same `prefix().next_back()` traversal pattern; each call creates a fresh iterator.

**Proof seed.** See `proof-seeds.jsonl` PS-7.

---

## H-5. Concurrency Hazards

### H-5.1 Lock-free helper vs. locked append

**Description.** `next_sequence_at_write` does not acquire `write_lock`. A concurrent `append_unfsynced` that mutates the keyspace after the helper observes a seq but before the helper returns may cause the helper to return a value that the append then "fails" on (if the append sees an unexpected seq).

**Surface.** The helper's `prefix().next_back()` returns a snapshot from Fjall's MVCC layer; the append, which holds `write_lock`, sees a *committed* snapshot that includes prior writes. The two are consistent within a single writer because LSM-tree reads inside a Fjall transaction can read uncommitted writes via the same transaction handle — but the helper does not use that, so it sees only committed data. Result: a write that observes `expected = n` and matches `actual = n` will succeed; the duplicate check at step 5 then guards against any race.

**Mitigation.** Keep the C6 step-4 same-batch guard and step-5 durable guard; they remain the source of truth for "this key is not already committed".

**Proof seed.** See `proof-seeds.jsonl` PS-8.

### H-5.2 Cross-process writers

**Description.** Out of scope for this bead. The process-level write lock (`ProcessLock`) is the only barrier.

**Surface.** A second process opening the same path will fail with `ProcessLockHeld`; the new guard does not change this.

**Mitigation.** Unchanged.

---

## H-6. Unsafe / Provenance Hazards

### H-6.1 No `unsafe` is introduced

**Description.** The fix is pure safe Rust. `#![forbid(unsafe_code)]` continues to apply.

**Surface.** N/A.

**Mitigation.** N/A.

---

## H-7. Hostile-Input / Fjall-Keyspace Hazards

### H-7.1 Malformed keyspace row

**Description.** A pre-existing malformed row in the `events` keyspace (length != 17, wrong prefix) could confuse `next_sequence_at_write`'s decode of the last key.

**Surface.** `decode_storage_key` returns a typed error; the helper MUST propagate it rather than silently returning a wrong seq.

**Mitigation.** Use `MalformedKeyspaceRow` (existing variant, `0x4030`); propagate via `?`.

**Proof seed.** See `proof-seeds.jsonl` PS-9.

### H-7.2 Concurrent reader / writer

**Description.** `next_sequence_at_write` runs concurrently with `events_for_run`. Both are Fjall snapshot readers. Both can return inconsistent views if the writer commits mid-iteration.

**Surface.** Fjall's MVCC provides snapshot isolation per iterator; a fresh `prefix().next_back()` after a writer commit will see the new state.

**Mitigation.** Document the snapshot semantics in the helper's doc-comment.

**Proof seed.** See `proof-seeds.jsonl` PS-8.

### H-7.3 Wire / disk corruption

**Description.** A forged or corrupted event value can produce a
`ReplayKeyMismatch` or `ReplayEnvelopeSequenceMismatch` (existing). The
new helper reads only keys; a corrupted value cannot change the helper's
output.

**Surface.** The helper is robust against value corruption by design
(key-only lookup).

**Mitigation.** Key-only `prefix().next_back()`.

**Proof seed.** See `proof-seeds.jsonl` PS-7.

---

## H-8. Performance Hazards

### H-8.1 Hot-path cost per append

**Description.** Each append adds one `prefix().next_back()` lookup. The LSM tree has the prefix range bounded by the cursor; the lookup is `O(1)` amortized after the LSM tree has positioned the cursor.

**Surface.** Negligible vs. the existing durability barrier (`SyncAll`). The benchmark in `crates/vb_storage/src/kani_*` already covers the existing path; an additional micro-benchmark is optional.

**Mitigation.** Document the expected perf cost. If a regression is
observed, profile with `cargo flamegraph` and consider caching the
`next_sequence_at_write` value within a `JournalWriteBatch`. Caching is
NOT recommended for v1; the simplest design wins.

**Proof seed.** See `proof-seeds.jsonl` PS-10.

### H-8.2 Allocation pressure from key decoding

**Description.** Each lookup allocates a user-key buffer for the matched item. The allocation is bounded by `JOURNAL_KEY_BYTES = 17`.

**Surface.** Negligible.

**Mitigation.** Use `run_prefix_key` 9-byte buffer (already allocated).

---

## H-9. Release / API / Cross-Crate Hazards

### H-9.1 New `pub fn` on `FjallJournal`

**Description.** Downstream crates (vb_runtime, vb_cli, workspace_tests) gain a new public method. Consumers that exhaustively match on `impl FjallJournal` methods are unaffected (no exhaustive match on methods exists in Rust).

**Surface.** Compatibility is preserved; downstream consumers may add their own calls.

**Mitigation.** Document the helper in `rust-contract/delivery-scope`; downstream consumers should update their allocator re-hydration logic to use this as the source of truth.

### H-9.2 New `JournalError` variant

**Description.** Existing matches over `JournalError` will gain a wildcard or new arm. Rust's exhaustiveness checker will surface the requirement.

**Surface.** All `match` over `JournalError` must add an arm OR use a wildcard. The risk is human: people may forget a wildcard.

**Mitigation.** Run `moon run :lint-src` after the variant is added; the source-lint gate surfaces non-exhaustive matches in the workspace.

### H-9.3 Cross-crate exhaustiveness

**Description.** Tests at `crates/workspace_tests/tests/proptest_error_types_*.rs` reference `JournalError`. The new variant must be added to the test lists.

**Surface.** Stale tests would fail on the next run.

**Mitigation.** Update the affected proptest lists per `codebase-map.md` §6.

### H-9.4 Diagnostic code `0x4042` collision

**Description.** `0x4040` and `0x4041` are taken. `0x4042` is free.

**Surface.** No collision.

**Mitigation.** Confirm via `crates/vb_storage/src/error/codes.rs` during implementation.

---

## H-10. Migration Hazards

### H-10.1 Pre-existing `TailHole` on-disk state

**Description.** A run that was written by a buggy build (pre-fix) may have a hole in its on-disk log. After the fix upgrades the journal, `events_for_run` continues to report `SequenceGap` for that run.

**Surface.** Recovery-driven; not a write-path issue.

**Mitigation.** Recovery code (out of scope) must handle pre-existing
holes. The fix does not change recovery logic.

### H-10.2 Active writers on upgrade

**Description.** A process running a pre-fix binary cannot be hot-swapped with a post-fix binary; the in-memory counters may disagree with the new helper's view after restart.

**Surface.** Restart sequence expected: shutdown pre-fix binary, then start post-fix binary. There is no in-place upgrade.

**Mitigation.** Documented in `migration`-style release notes (out of scope for this contract).

---

## H-11. Diagnostic / Observability Hazards

### H-11.1 Operator logs missing context

**Description.** A `Display` of `SequenceMismatch` that does NOT include `run` would be useless for operator triage.

**Surface.** The `Display` impl MUST mention `run`, `expected`, and `actual`. Tests verify.

**Mitigation.** Format string in `type-contracts.md` §2.2.1.

### H-11.2 Counter underflow / overflow unnoticed

**Description.** If `SequenceOverflow` fires, the operator must notice.

**Surface.** Same as today; `SequenceOverflow` is the existing typed error.

**Mitigation.** Unchanged.

---

## 12. Severity Tagging (For Downstream Stage Routing)

| Hazard | Severity | Required verifier lane (seed-only, planner-owned) |
|---|---|---|
| H-1.1 | high | Rust-local refinement (Verus/Kani); proptest on caller scenarios |
| H-1.2 | high | Rust-local proptest (sequencing); update to line 4585 test |
| H-1.3 | medium | proptest (batch ordering) |
| H-2.1 | high | Behavior tests across all five append paths |
| H-2.2 | low | Unit test `next_sequence_at_write_returns_zero_for_fresh_run` |
| H-2.3 | medium | Unit test `sequence_mismatch_constructor_fields` |
| H-3.1 | medium | Boundary test at `EventSeq::MAX` |
| H-3.2 | low | Unit test `next_sequence_at_write_returns_zero_for_fresh_run` |
| H-4.1 | medium | Refinement test; helper agrees with `events_for_run`'s view |
| H-5.1 | medium | Loom model or `#[cfg(loom)]` test (single-process; multi-writer out of scope) |
| H-7.1 | low | Key-only lookup; key-decode-test on malformed row |
| H-7.2 | medium | Loom model (concurrent read during write) |
| H-8.1 | low | Criterion benchmark (optional) |
| H-9.1 | low | Compile-time check on `pub fn` signature |
| H-9.2 | medium | `cargo build` over the workspace; source-lint gate |

---

## 13. Sealed Hazards (Out of Scope, Listed for Record)

- Multi-process writers (already ruled out by process-lock).
- Fjall LSM tree corruption (out of scope; corruption is observable via `MalformedKeyspaceRow`).
- Network / RPC append path (not present).
- Async / cancellation integration (no new async surface).
- YAML / JSON / HTTP parsing (not present).

---

## 14. Acceptance: Hazard List

The hazard list is complete when a reviewer (1) can identify which guard
mitigates each hazard and (2) can locate the corresponding proof seed in
`proof-seeds.jsonl`. The next stage (`proof-plan-reviewer`) is the
authoritative reviewer of seed completeness.
