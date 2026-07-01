# Domain Model — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Bead Scope (verbatim)

> Storage key parsing accepts trim/snapshot keys of unbounded length. Add a
> length cap (e.g., `MAX_TRIM_KEY_LEN`, `MAX_SNAPSHOT_KEY_LEN`) and reject
> overlong keys with a typed error.

This domain model covers the **P1 bug** documented as Round 10 issue 7
(`snapshot_tests.rs:208-248`) and the analogous SC-006 / CC-002 surfaces in
`trimming/tests.rs:875-987`. The change is strictly internal to
`crates/vb_storage`; no cross-crate API changes are introduced.

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| **Journal Keyspace** | The two Fjall partitions that store per-run, per-sequence records: `run_event` (prefix `0x11`) and `run_snapshot` (prefix `0x12`). |
| **Journal Key Envelope** | `[prefix:u8][run_id:u64 BE][seq:u64 BE]` — the canonical 17-byte wire form for any record in the journal keyspace. |
| **`JOURNAL_KEY_BYTES`** | The compile-time byte width of a journal key envelope. Equal to `1 + 8 + 8 = 17`. The single source of truth. |
| **`MAX_TRIM_KEY_LEN`** | Public-facing alias for `JOURNAL_KEY_BYTES`, scoped to the **trim** call sites that read raw `run_event` keys. |
| **`MAX_SNAPSHOT_KEY_LEN`** | Public-facing alias for `JOURNAL_KEY_BYTES`, scoped to the **trim** call sites that read raw `run_snapshot` keys. |
| **Trim Scanner** | Any iterator over a journal keyspace prefix in `trimming/logic.rs` (`latest_durable_snapshot_seq`, `trim_events_for_run`, `count_trimmable_events`). |
| **Canonical Key** | A raw key whose byte length equals `MAX_*_KEY_LEN` for its prefix family. |
| **Non-canonical Key** | A raw key whose byte length is `!= MAX_*_KEY_LEN` for its prefix family (shorter OR longer). Includes left-over test artefacts, prefix-collision rows, and corrupt LSMtree payloads. |
| **Fail-Closed** | The scanner terminates with a typed `Err` on the **first** non-canonical observation rather than silently truncating, skipping, or producing a wrong `seq`. |
| **Typed Error** | An error variant that is `pub` (visible to callers) and carries a structured payload identifying the cause; specifically `TrimError::IncompleteTrim { deleted_count: u64 }` carrying the scan's progress counter. |
| **Diagnostic Code** | The 16-bit `DiagnosticCode` registered for the typed error: `0x4102` for `TrimError::IncompleteTrim`, `0x4030` for `JournalError::MalformedKeyspaceRow`. |

---

## Value Objects

### `MAX_TRIM_KEY_LEN`

```rust
pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES; // == 17
```

- **Type**: `usize` const alias.
- **Visibility**: `pub(crate)` (mirrors the surrounding length constants in
  `crates/vb_storage/src/constants.rs:74-80`).
- **Equation**: `MAX_TRIM_KEY_LEN == JOURNAL_KEY_BYTES` is a compile-time
  invariant; the compiler enforces equality because both sides are `usize`
  consts in the same crate.
- **Domain meaning**: The maximum byte length of a `run_event` raw key
  accepted by the trim scanner. **Not** a soft cap — a longer raw key is a
  malformed row and must yield `TrimError::IncompleteTrim`.
- **Co-located with**: `JOURNAL_KEY_BYTES` declaration site
  (`crates/vb_storage/src/constants.rs:74`); see file-pointer index below.

### `MAX_SNAPSHOT_KEY_LEN`

```rust
pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES; // == 17
```

- **Type**: `usize` const alias.
- **Visibility**: `pub(crate)`.
- **Equation**: `MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES`.
- **Domain meaning**: The maximum byte length of a `run_snapshot` raw key
  accepted by the trim scanner.
- **Co-located with**: `MAX_TRIM_KEY_LEN` (same declaration site).

### `JOURNAL_KEY_BYTES` (unchanged)

```rust
pub(crate) const JOURNAL_KEY_BYTES: usize = 17; // source of truth
```

- Still the single source of truth. `MAX_TRIM_KEY_LEN` and
  `MAX_SNAPSHOT_KEY_LEN` are derived aliases — never redefine the value at
  the alias site.

---

## Entities

### `TrimScanner`

- The logical entity covering the three trim functions:
  `latest_durable_snapshot_seq`, `trim_events_for_run`, `count_trimmable_events`.
- **Responsibilities**: iterate a keyspace prefix; verify each raw key's
  length against the named cap; produce a typed `Err` on the first
  non-canonical observation; otherwise produce a typed `Ok` (`Option<seq>`,
  `TrimmedRunResult`, or `u64` count respectively).
- **Forbidden behaviours** (cannot be representable post-fix):
  - `Ok(seq)` from a key whose `len() != MAX_*_KEY_LEN`.
  - Silently truncated or padded keys.
  - Skip-and-continue on non-canonical key (would corrupt the safe-point
    invariant).

### `RawJournalKeyBytes`

- The raw `&[u8]` slice returned by `fjall::UserKey` deref.
- **Domain meaning**: opaque, possibly non-canonical. The trim scanner must
  inspect `.len()` against the cap before any further interpretation.
- **Forbidden post-fix**: any code path that observes `.len() == 17` for a
  non-journal-keyspace prefix (cross-check via `decode_storage_key`).

---

## Invariants

1. **INV-CAP-001 — Encoder/Alias equality**: For all three constants,
   `MAX_TRIM_KEY_LEN == MAX_SNAPSHOT_KEY_LEN == JOURNAL_KEY_BYTES == 17`. The
   compiler enforces this trivially because each alias is a `const` reference
   to the same `usize` literal.
2. **INV-CAP-002 — Trim fails closed on non-canonical length**: For any raw
   key `k` observed by a trim scanner where `k.len() != MAX_*_KEY_LEN`, the
   scanner returns `Err(TrimError::IncompleteTrim { deleted_count })` before
   any side effect (no `batch.remove`, no `batch.commit`).
3. **INV-CAP-003 — Typed error carries progress**: When the scanner aborts,
   the `deleted_count` field of `IncompleteTrim` reflects the number of keys
   that were ALREADY removed prior to the abort. The `count_trimmable_events`
   path carries the partial count up to the abort point.
4. **INV-CAP-004 — Diagnostic code stability**: All `TrimError::IncompleteTrim`
   surfacings from the three call sites continue to map to
   `0x4102` (`TrimError::INCOMPLETE_TRIM_CODE`) — no code-map change.
5. **INV-CAP-005 — Length-check precedes decode**: The trim scanner rejects a
   non-canonical length BEFORE invoking `decode_storage_key`. The secondary
   `decode_storage_key` check at `latest_durable_snapshot_seq:43` is a
   prefix-collision safety net, not a replacement for the length check.

## Forbidden States

| Forbidden State | Why forbidden | Source-of-truth prevention |
|-----------------|---------------|----------------------------|
| `key.len() == 17` accepted as a `RunEvent` key when its first byte is `0x12` | Prefix collision between `run_event` (`0x11`) and `run_snapshot` (`0x12`) | Length check + `decode_storage_key` re-validation at `logic.rs:43` |
| `key.len() > 17` accepted silently | Round 10 issue 7 bug: would let a leftover test artefact masquerade as a durable snapshot and cause wrong-event trimming | `IncompleteTrim` rejection at the three sites |
| `key.len() < 17` accepted silently | Would cause `key.get(9..17)` to return `None` → currently typed as `IncompleteTrim { deleted_count: 0 }`. Post-fix the same `IncompleteTrim` continues to surface | `IncompleteTrim` rejection at the three sites |
| Trim scanner returning `Ok(0)` (no events deleted) when a malformed key was observed | Would silently lose the fail-closed property | `IncompleteTrim` carries `deleted_count > 0` when progress was made; `deleted_count: 0` only when the abort happens at the first observation |

---

## Aggregate Boundaries

The **trim/snapshot keyspace** is the only aggregate touched:

- **Aggregate root**: the Fjall `run_event` and `run_snapshot` partitions.
- **Invariant scope**: every trim scanner touching these partitions MUST
  enforce `key.len() == MAX_*_KEY_LEN` before any state mutation.
- **Out of scope**: the `run_header`, `index_*`, `blob`, `workflow_source`,
  and `compiled_ir` keyspaces — they already use different length constants
  (`RUN_ONLY_KEY_BYTES=9`, `INDEX_STATUS_KEY_BYTES=18`, etc.) and their own
  scanners (e.g., `headers.rs:67`) already use `MalformedKeyspaceRow`.

---

## Aggregate Events (typed from scanner to caller)

| Event | Producer | Consumer | Type |
|-------|----------|----------|------|
| `TrimError::IncompleteTrim { deleted_count }` | trim scanners | `trim_events_for_run`, `count_trimmable_events`, `latest_durable_snapshot_seq` | `TrimError` (0x4102) |
| `JournalError::Trim(Box<TrimError>)` | `trim_eligibility_diagnostic` (line 165, 182) | public API | `JournalError` |

The `From<TrimError> for JournalError` conversion is already in place
(`error/mod.rs:187`); the contract preserves this chain so the existing
trim-diagnostic tests (`trimming/tests.rs:934-987`) continue to assert
`JournalError::Trim(inner) -> inner == TrimError::IncompleteTrim { .. }`.

---

## File-Pointer Index (where each constant/type lives after the fix)

```
crates/vb_storage/src/constants.rs:74-75            # JOURNAL_KEY_BYTES + new aliases
crates/vb_storage/src/trimming/helpers.rs          # 9-byte prefix cursor (co-locate caps here too)
crates/vb_storage/src/trimming/logic.rs:36         # latest_durable_snapshot_seq length check
crates/vb_storage/src/trimming/logic.rs:77         # trim_events_for_run length check
crates/vb_storage/src/trimming/logic.rs:222        # count_trimmable_events length check
crates/vb_storage/src/trimming/mod.rs:51-54        # TrimError::IncompleteTrim shape (preserved)
crates/vb_storage/src/error/mod.rs:187             # From<TrimError> for JournalError (preserved)
crates/vb_storage/src/error/codes.rs:62            # INCOMPLETE_TRIM_CODE = 0x4102 (preserved)
crates/vb_storage/src/snapshot_tests.rs:208-248    # Round 10 issue 7 regression test (preserved + augmented)
crates/vb_storage/src/trimming/tests.rs:875-987    # SC-006 / CC-002 fail-closed tests (preserved + augmented)
```

END OF DOMAIN MODEL.