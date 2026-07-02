# Domain Model — vb-r8oso

**bead_id:** vb-r8oso
**title:** Storage: enforce next-sequence-at-write before durable append (P1 bug)
**owner_stage:** rust-contract
**upstream_artifacts:** `.beads/vb-r8oso/codebase-map.md`, `.beads/vb-r8oso/delivery-scope.jsonl`

This artifact captures the ubiquitous language, value objects, entities, commands,
events, policies, invariants, and forbidden states introduced or hardened by the
bead. It is the source of truth for every later type, error, workflow, boundary,
hazard, proof-seed, and contract clause in `.beads/vb-r8oso/`.

---

## 1. Ubiquitous Language

| Term | Definition |
|---|---|
| **Run** | A single execution of an admitted workflow identified by `RunId(u64)`. A run owns one strictly-contiguous sequence of `JournalEvent`s. |
| **Event sequence (`seq`)** | A `u64`-newtype (`EventSeq`) assigned in monotonically increasing order starting at `ZERO` for the first event of a fresh run. |
| **Journal** | The per-run, append-only log of `JournalEvent`s, durably stored in the `events` Fjall keyspace. |
| **Key** | The 17-byte Fjall key `[0x11][run_id_be_8][seq_be_8]` (prefix `PREFIX_RUN_EVENT = 0x11`, big-endian). |
| **Append path** | Any storage method that writes a `JournalEvent` into the durable log: `append_journaled`, `append_strict`, `append_strict_batch`, `append_unfsynced`, or `JournalWriteBatch::append_event`. |
| **Caller** | Any code outside `vb_storage` that invokes an append path. Today the only legitimate producers are `vb_runtime::shard` (per-run in-memory counter) and a small set of integration-test helpers. |
| **`next_sequence_at_write(run)`** | The function introduced by this bead: returns the `EventSeq` that the *next* successful append for `run` must carry. `ZERO` for a fresh run; otherwise `last_durable_seq.succ()`. |
| **`SequenceMismatch`** | The new typed error emitted when an append supplies a `seq` that disagrees with `next_sequence_at_write(run)` at write time. Distinct from the existing `SequenceGap`, which is read/replay-time only. |
| **`SequenceGap`** | Existing read-side error emitted when an existing on-disk contiguous sequence has a hole. Remains in scope; never appears at write time under the fix. |

## 2. Value Objects (New and Reused)

### 2.1 Reused (no change)

- `RunId(vb_core::RunId)` — opaque `u64` newtype with `ZERO`-rejection guard (`InvalidRunId`).
- `EventSeq(crate::types::EventSeq)` — `u64` newtype with `MAX` sentinel reserved by codec.
- `JournalEvent(crate::events::JournalEvent)` — sum-type of all replay-relevant run events.

### 2.2 New

- **No new value object is introduced.** The bead is type-adding (`SequenceMismatch`) and method-adding (`next_sequence_at_write`), not type-introducing. The return value of `next_sequence_at_write` is the existing `EventSeq`. The `JournalError::SequenceMismatch` variant is keyed by `RunId` and two `EventSeq` fields, all reused.

## 3. Entities

- **`FjallJournal`** — owns the `events` keyspace and the `write_lock`. Adds the new method `next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError>`. Existing append methods are unchanged in signature; their semantics tighten.
- **`ReadOnlyJournal`** — read-only wrapper around `FjallJournal`. Exposes `events_for_run`; does not expose `next_sequence_at_write` (write-adjacent; reserved for writer contexts), but the call site that needs the next-seq for a run after a recovery scan may use the inner journal through a writer context.

## 4. Aggregates

- **Run aggregate root:** the projection `(run_id, sorted_by_seq events for run)`. The aggregate invariant is **strict contiguity**: for every `n ∈ [0, len(events_for_run(run)))`, `events[n].seq() == EventSeq::new(n as u64)`. This aggregate invariant is what `next_sequence_at_write` preserves at write time and what `events_for_run` validates at read time.

## 5. Commands (Write Paths Affected)

Each of the five append paths is a command. Their pre/post-conditions all tighten
identically:

| Command | Invariant added (post-condition) |
|---|---|
| `FjallJournal::append_journaled(event)` | On `Ok(())`, `event.seq()` was either `next_sequence_at_write(event.run_id())` immediately pre-call, OR a re-stamp from the recovery path is explicitly forbidden by the no-silent-rewrite invariant. |
| `FjallJournal::append_strict(event)` | Same as `append_journaled`; atomically also satisfies the SyncAll durability barrier on success. |
| `FjallJournal::append_strict_batch(events)` | Atomically commits the whole slice OR rejects all; on reject, the durable log for any involved run is unchanged. |
| `FjallJournal::append_unfsynced(event)` (`pub(crate)`) | Same invariant; this is the lowest-level write that every other path delegates to. |
| `JournalWriteBatch::append_event(event)` | Same invariant; in-batch duplication and durable duplication checks continue to fire *before* the next-sequence check, per C6 guard precedence. |

The order of guards in C6 is updated by this bead (a new guard slot is inserted
without shifting the existing slots):

1. Key construction (`run_event_key`).
2. Semantic event validation (`event.is_valid()`).
3. **NEW** — `next_sequence_at_write` guard: call `next_sequence_at_write(run)`; reject with `SequenceMismatch { run, expected, actual }` when `actual != expected`.
4. Same-batch duplicate check (HashSet guard).
5. Durable duplicate check (`events.contains_key` → `DuplicateEvent`, abort batch).
6. Count capacity check (`QueueFull`).
7. Per-record encoding (`PayloadTooLarge`).
8. Accumulated byte admission (`JournalBatchBytesExceeded`).
9. Insert into inner `OwnedWriteBatch`.

## 6. Events

- **`Ok(())` on every append path** is itself a confirmation event: the durable log accepted exactly one `(run, seq)` corresponding to `next_sequence_at_write(run)` at the moment of write.
- **`Err(JournalError::SequenceMismatch { run, expected, actual })`** is the new typed negative outcome emitted at write time when the supplied `actual` does not equal `expected == next_sequence_at_write(run)`. This is the only new domain event introduced.

## 7. Policies

- **NP-1 No-Silent-Rewrite Policy.** Append paths MUST reject any `event.seq() != next_sequence_at_write(run)` with `JournalError::SequenceMismatch`. NEVER rewrite `event.seq()` to the expected value. NEVER downgrade the error to a log/Ok-with-modified-record. This is the contract that distinguishes this fix from a "convenience auto-correct" implementation.
- **NP-2 Batch Atomicity Policy.** `append_strict_batch` MUST reject the entire batch atomically when any element's `seq != next_sequence_at_write(run)` after all preceding accepted elements are accounted for. No partial durable commits. The same applies to a `JournalWriteBatch` whose `.commit()` is preceded by one or more successful `append_event` calls and then one `SequenceMismatch`-triggering call.
- **NP-3 Identifier-First Policy.** `RunId::ZERO` (reserved invalid run identifier) continues to be rejected at write time with `JournalError::InvalidRunId`, independent of `next_sequence_at_write`. The new guard does not change identifier semantics.
- **NP-4 Defensive-Lookup Policy.** `next_sequence_at_write` MUST use a key-only `prefix().next_back()` traversal, never decode any event value (`BLAKE3 + postcard` payload). This matches `latest_durable_snapshot_seq` at `crates/vb_storage/src/trimming/logic.rs:26` and keeps the helper `O(1)` per LSM prefix seek.

## 8. Invariants (Domain-Level)

1. **Contiguity at Write.** For every successful `append_x(event)` for run `R`, the durable log for `R` post-call contains `event` as the new tail, and the tail's `seq` satisfies `seq == next_sequence_at_write(R) pre-call`.
2. **Contiguity at Read.** `events_for_run(R)` returns a strictly contiguous `0..N-1` prefix with no holes; a hole, when present, surfaces as `JournalError::SequenceGap`. This invariant is unchanged by the fix and continues to be the read-time report mechanism for any leftover corruption.
3. **Distinct Diagnostics for Write vs. Read Mismatch.** A `seq` disagreement detected at write time is reported as `SequenceMismatch` (code `0x4042`); the same condition observed later during replay is reported as `SequenceGap` (code `0x4009`). The diagnostic semantics are deliberately split because the call-site semantics differ: write-time = caller misbehaviour; replay-time = on-disk corruption.
4. **Deterministic Failure Path.** Under the fix, the diagnostic chain on a buggy caller is: `append_x` → `Err(SequenceMismatch { run, expected, actual })` → log/observe → caller bug fix. The chain never includes a durable half-write followed by a later `SequenceGap`. The chain never includes a silent `seq`-rewrite.

## 9. Forbidden States

- **FS-1** — `events_for_run(run)` returning a non-contiguous prefix where a gap *was created during this process's lifetime*: under the fix, this state is unreachable by means of any in-process append path.
- **FS-2** — A durably-committed event whose `seq` value disagrees with `next_sequence_at_write(run)` at the moment of its commit. Unreachable post-fix for in-process appends; remains reachable only via direct Fjall keyspace manipulation outside the typed API (and is surfaced as `SequenceGap` on the next read).
- **FS-3** — A "repair" outcome that silently rewrites `event.seq()` to match the expected value. Forbidden by NP-1.
- **FS-4** — A partial commit where some events in an `append_strict_batch` land durably and others are rejected. Forbidden by NP-2.
- **FS-5** — A `SequenceMismatch` returned for `RunId::ZERO`. Forbidden: `InvalidRunId` is the typed outcome for run-zero at write time; the next-seq guard is unreachable for invalid identifiers.

## 10. Ubiquitous-Language Rule

When talking to any downstream owner (`test-planner`, `holzman-rust`,
`proof-planner`, `proof-writer`, `black-hat-reviewer`), use the terms from §1
unchanged. Do not paraphrase "the missing-seq error", "the out-of-order error",
or "the appended-with-wrong-number error" — they are all `SequenceMismatch`.

---

## 11. Open Domain Questions

| ID | Question | Owner | Required before |
|---|---|---|---|
| ODQ-1 | Does any downstream caller legitimately append a non-contiguous `seq` (e.g., a crash-replay path that writes a recovered `RunAccepted` with its original seq)? The bead description rules this out, but the implementer MUST grep `append_journaled\|append_strict\|append_unfsynced\|append_event` across `crates/vb_runtime` and `crates/vb_storage::recovery` and report any caller that supplies an `event.seq()` not derived from a fresh per-run counter. | `holzman-rust` | Implementation start. If a legitimate caller exists, the contract here must be revised to widen `SequenceMismatch` into a recoverable subclass or to allow a per-caller opt-in; otherwise the bead proceeds as written. |
| ODQ-2 | Is the JVM/FFI interop expected to write events directly via Fjall `events.insert` bypassing the typed API? *(No evidence so far — assumed no.)* | `holzman-rust` | Implementation start. If yes, the fix is incomplete; either expose a typed wrapper or accept that the guarantee only holds for in-process typed callers. |
| ODQ-3 | Does `next_sequence_at_write` need to be re-exported through `ReadOnlyJournal` so a recovery scan can correlate read tails with writes? Current scope: read-only paths remain unchanged; the wrapper does not need the helper. The recovery path operates on the writer context. *(Resolved: no re-export needed.)* | `rust-contract` (resolved) | — |
| ODQ-4 | Should the diagnostic code `0x4042` be added to the `SymbolicCode::CODE_REGISTRY` list, or is the existing fallback to `INTERNAL_INVARIANT` acceptable for the first release? The other `0x404x` siblings have been registered historically; default plan is to register. Confirm by reading `SymbolicCode::from_static` lookup. | `holzman-rust` | Code-registration test pass. |

## 12. Illegal-State Risks That Remain Representable

After the fix, the following states are STILL representable in the type system
and require the listed defenses (NOT the responsibility of the rust-contract
stage to eliminate — listed for downstream awareness):

| Risk | Defense layer |
|---|---|
| A caller computes `seq` from a stale in-memory counter after a crash-replay finishes | `SequenceMismatch` surfaces the bug at write time; runtime allocator must reset on recovery (out of scope, owned by runtime). |
| Direct Fjall keyspace manipulation bypassing the typed API | `events_for_run` continues to report `SequenceGap` on read; corruption is observable, not silent. |
| A multi-writer scenario where two processes append to the same `(run, seq)` race | Out of scope (process-level write lock is enforced); `SequenceMismatch` or `DuplicateEvent` is the surface. |
| A long-running tail where `last_durable_seq.succ()` overflows `u64::MAX` | `SequenceOverflow` is the existing typed error, mapped by `codec::next_seq`. `next_sequence_at_write` MUST return `Err(JournalError::SequenceOverflow)` in this case, not panic. |
