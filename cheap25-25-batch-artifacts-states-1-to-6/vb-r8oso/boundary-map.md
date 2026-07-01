# Boundary Map — vb-r8oso

**bead_id:** vb-r8oso
**owner_stage:** rust-contract
**upstream_artifacts:** `domain-model.md`, `type-contracts.md`, `workflow-model.md`, `error-taxonomy.md`

This artifact fixes the functional-core / imperative-shell / parser /
storage / async boundaries for the fix. It tells downstream
implementations where the new code lives, what it must not import, and
which existing boundaries are deliberately unchanged.

---

## 1. Layer Diagram

```
                +---------------------------------------------------------------+
                |                       RUNTIME CRATES                            |
                | vb_runtime  (chunk_001.rs, engine/action.rs)                   |
                | vb_cli      (lifecycle.rs, run_cancel_ops.rs)                  |
                | workspace_tests/*.rs  (test helpers, all *event appenders)     |
                +-----------------------------+---------------------------------+
                                          |
                                          |  calls FjallJournal::append_x
                                          |  passes journal_event.seq() = N
                                          v
                +---------------------------------------------------------------+
                |                          STORAGE CRATE                        |
                |                            vb_storage                          |
                |                                                                   |
                |  IMPERATIVE SHELL: (write paths - append.rs, internal.rs,       |
                |                         batch/append_event.rs)                    |
                |     - all guards from C6 precedence                              |
                |     - NEW: next_sequence_at_write guard (step 3)                |
                |                                                                   |
                |  PUBLIC API (public_api.rs):                                     |
                |     - wrappers: open_store, append_journal_event, read_run_events|
                |     - NEW: pub fn next_sequence_at_write(journal, run)           |
                |                                                                   |
                |  FUNCTIONAL CORE:                                                |
                |     - codec::next_seq (succ with overflow detection)             |
                |     - keys::run_event_key, run_prefix_key                         |
                |     - decoding helpers                                           |
                |                                                                   |
                |  STORAGE BOUNDARY:                                               |
                |     - FjallJournal::events (Keyspace)                            |
                |     - prefix().next_back() — key-only lookup                     |
                |     - write_lock (Mutex)                                         |
                +-----------------------------+---------------------------------+
                                          |
                                          v
                +---------------------------------------------------------------+
                |                       FJALL BACKEND                            |
                |   keyspace events  (LSM tree)                                   |
                |   prefix [0x11][run_be_8] -> events (sorted by seq_be_8)        |
                +---------------------------------------------------------------+
```

## 2. Pure Functional Core (No I/O, No Time, No Network)

| Helper | Module | Notes |
|---|---|---|
| `codec::next_seq(seq) -> Result<EventSeq, JournalError>` | `crates/vb_storage/src/codec/mod.rs:153` | `checked_add`, maps overflow to `SequenceOverflow`. Pure; reused by the new method. |
| `keys::run_event_key(run, seq)` | `crates/vb_storage/src/keys.rs:81` | Pure; reused. |
| `keys::run_prefix_key(run)` | `crates/vb_storage/src/keys.rs:524` | Pure 9-byte `[0x11][run_be_8]` prefix. Reused by the new helper. |
| `keys::journal_key(run, seq)` (private) | `crates/vb_storage/src/keys.rs` | Pure; reused. |

The new code does not add to the pure-core surface because the helper
itself is a thin composition: pure `next_seq` over a key-only Fjall LSM
prefix scan.

## 3. Imperative Shell — Write Paths

### 3.1 Files Touched

| File | Purpose | New insertion point |
|---|---|---|
| `crates/vb_storage/src/journal/internal.rs` | `append_unfsynced` — lowest-level write. Acquired lock guard, then C6 guards. | After `event.is_valid()` check, before `if self.events.contains_key(key)`. |
| `crates/vb_storage/src/journal/append.rs` | `append_journaled`, `append_strict`, `append_strict_batch`, `persist_strict` — public write API. | Doc-comments only; behavior change is inherited from `append_unfsynced` and `JournalWriteBatch::append_event`. |
| `crates/vb_storage/src/batch/append_event.rs` | `JournalWriteBatch::append_event` — per-event batch stage. | Insertion as new C6 step 3 between `event.is_valid()` and the same-batch duplicate check. |
| `crates/vb_storage/src/public_api.rs` | Convenience wrappers. | New wrapper `pub fn next_sequence_at_write(journal, run)`. |

### 3.2 Common Insertion Pattern

The new guard insertion is identical across all five append paths:

```rust
// AFTER event.is_valid() (which may emit InvalidEvent),
// BEFORE same-batch duplicate check:
let expected = self.next_sequence_at_write(event.run_id())?;
if event.seq() != expected {
    // batch path additionally sets self.aborted = true before returning.
    return Err(JournalError::SequenceMismatch {
        run: event.run_id(),
        expected,
        actual: event.seq(),
    });
}
```

### 3.3 Locking

- The new helper `next_sequence_at_write` does not require `write_lock`; it uses an LSM-tree snapshot read that is internally consistent.
- The five append paths continue to acquire `write_lock` for the duration of their C6 step sequence. The new guard sits inside the locked region for the direct `append_unfsynced` path; for `JournalWriteBatch::append_event`, the batch's stage lock is unaffected.
- Concurrent readers (`events_for_run`) and the helper see a consistent snapshot; there is no torn-read concern at the LSM-tree level.

## 4. Storage Boundary

### 4.1 Fjall Keyspace

| Keyspace | Key | Value | Used by |
|---|---|---|---|
| `events` | `[0x11][run_id_be_8][seq_be_8]` | postcard-encoded `JournalEvent` | All append paths, `events_for_run`, recovery, `next_sequence_at_write`. |

### 4.2 New Lookup Pattern

The new helper mirrors `trimming::logic::latest_durable_snapshot_seq`:

```rust
pub fn next_sequence_at_write(&self, run: RunId) -> Result<EventSeq, JournalError> {
    let prefix = run_prefix_key(run)?;
    let Some(item) = self.events.prefix(prefix).next_back() else {
        return Ok(EventSeq::ZERO);
    };
    let (key, _) = item.into_inner().map_err(...)?;
    // parse key tail seq field; keys are exactly 17 bytes (verified by key shape).
    match decode_storage_key(&key)? {
        StorageKey::RunEvent { seq, .. } => codec::next_seq(seq),
        _ => Err(JournalError::MalformedKeyspaceRow { ... }),
    }
}
```

The `prefix().next_back()` traversal:
- avoids full-prefix iteration (`O(N)` per call is avoided),
- avoids event-value decode (`BLAKE3 + postcard` cost avoided),
- preserves LSM-tree consistency (`Readable::prefix` returns a snapshot),
- is independent of any `write_lock` acquisition.

### 4.3 No Multi-Writer Boundaries

Cross-process writers are intentionally not supported. The
process-level lock is the only serialization point. The helper's
correctness holds under single-process writers only. Multi-writer
expansion is out of scope and would require additional invariants
(e.g., per-`(run, seq)` reservation).

## 5. Async Shell

No new async surface. The fix is fully synchronous. The existing
`JournalWriterQueue` (which is async) continues to delegate to the
synchronous append path; the queue's batching behaviour is unaffected
beyond inheriting the new guard through `append_strict_batch` /
`append_unfsynced`.

## 6. Runtime / CLI Caller Boundary

### 6.1 Producers (Existing and Verified-Unchanged Semantically)

| File | Function | Today's seq source |
|---|---|---|
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:194` | `append_journal_event(event)` | `self.journal_sequence_for(run)` which returns the in-memory per-run counter |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:224` | `journal_sequence_for(run)` | `self.journal_sequences.get(&run).copied().unwrap_or(EventSeq::ZERO)` — infaillibly defaults to `ZERO` |
| `crates/vb_runtime/src/engine/action.rs:150` | action completion path | local `next_seq` over in-memory state |
| `crates/vb_cli/src/lifecycle.rs:106,191,272,366` | CLI cancel/lifecycle | derived from `events_for_run(...).last().map(seq)` + 1 |
| `crates/vb_cli/src/run_cancel_ops.rs:42` | CLI cancel ops | `events.last().map(seq)` derived |
| `crates/workspace_tests/tests/journal_side_index_contracts.rs` (and 9 sibling tests) | test helpers | direct calls with caller-supplied seq; out of production path |

The contract demands that under the fix:

- Production runtime callers emit a `seq` that always equals
  `next_sequence_at_write(run)` immediately before their append call,
  because their `seq` is derived from the in-memory counter (which
  itself tracks the same number, modulo crash-recovery hydration that
  is out of scope).
- Test helpers that supply an arbitrary `seq` will fail in the updated
  tests at `crates/vb_storage/src/tests.rs:1737` and `:4612` — this is
  intentional and the helpers must be updated.

### 6.2 Open Caller Audit (Open Question ODQ-1)

The contract assumes no downstream caller writes a non-contiguous
`seq`. The implementer MUST grep across `crates/vb_runtime` and
`crates/vb_storage::recovery` for `append_journaled`/`append_strict`/
`append_unfsynced`/`append_event` callers and report findings before
closing the bead. If a legitimate caller exists, the contract widens
(see domain-model.md §11 ODQ-1).

## 7. Parser / Codec Boundary

### 7.1 Wire / Disk Format

- No on-disk format change. The `events` keyspace key/value schema is
  unchanged.
- No postcard schema change. `JournalEvent` is unchanged.
- No codec migration path is added; the new helper uses existing
  `keys::run_prefix_key` and `decode_storage_key`.

### 7.2 Hostile Input Boundary

- The fix introduces NO new wire-input or yaml/json/http parsing.
- All existing parser boundaries (`postcard::from_bytes`,
  `KeyspaceScanPolicy`, `decode_record`) are unchanged.

## 8. Time / Clock / Network / FFI

- No new time, clock, network, or FFI surface.
- No `unsafe` introduced (`#![forbid(unsafe_code)]` at the crate root).

## 9. Test and Kani Harness Boundary

### 9.1 Cargo Feature

`crates/vb_storage/Cargo.toml` adds:

```toml
[features]
kani-sequence-at-write = []

[lib]
# kani_sequence_at_write module is registered in lib.rs only when kani + this feature are both set.
```

### 9.2 Module Registration

`crates/vb_storage/src/lib.rs` adds:

```rust
#[cfg(all(kani, feature = "kani-sequence-at-write"))]
pub mod kani_sequence_at_write;
```

The harness module follows the existing pattern (`kani_vb_vzcuf_ps001.rs` through
`kani_vb_vzcuf_ps009.rs`). It emits at least two harnesses:

- `kani_next_sequence_at_write_returns_succ` — over a stored run prefix
  of arbitrary `n ∈ [0, 4]` events, asserts
  `next_sequence_at_write(run) == EventSeq::new(n)`.
- `kani_next_sequence_at_write_returns_zero_for_empty` — over an empty
  run, asserts `next_sequence_at_write(run) == EventSeq::ZERO`.

These belong to `proof-writer` stage output; this contract only fixes the
**boundary** (where the file lives, what feature gates it).

## 10. Boundary Acceptance Checklist

For every reviewer downstream, the contract is met when:

1. The five append paths emit `SequenceMismatch` for caller-supplied `seq != expected`, never `SequenceGap`.
2. The five append paths do NOT silently rewrite `event.seq()`.
3. `next_sequence_at_write` returns `ZERO` for a fresh run.
4. `next_sequence_at_write` returns `last.succ()` for a run with `n ≥ 1` events.
5. `next_sequence_at_write` does not decode any event value.
6. `next_sequence_at_write` does not hold `write_lock`.
7. The guard does not introduce any new wire/codec/time/async boundary.
8. The new Kani harness group is gated behind the `kani-sequence-at-write` Cargo feature and registered under `cfg(all(kani, feature = ...))`.

Violations of any of the above are contract breaches and must be reported in the next reviewer stage.
