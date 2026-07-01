# Codebase Map — vb-d9ml3 (Storage trim/snapshot key length cap, P1)

> **Bead scope (verbatim)**: "Storage key parsing accepts trim/snapshot keys of unbounded
> length. Add a length cap (e.g., MAX_TRIM_KEY_LEN, MAX_SNAPSHOT_KEY_LEN) and reject
> overlong keys with a typed error."

This map is read-only: it points at the existing trim/snapshot keyspace handling so
downstream contract, proof, test, and implementation agents can plan edits without
re-scanning the whole crate. **No production code is modified by this artifact.**

---

## 1. Crate boundaries

| Concern                  | Crate         | Module path                                  |
|--------------------------|---------------|----------------------------------------------|
| Storage backend (Fjall)  | `vb_storage`  | `crates/vb_storage/src/`                     |
| Trimming policy + logic  | `vb_storage`  | `crates/vb_storage/src/trimming/`            |
| Key encoders/decoders    | `vb_storage`  | `crates/vb_storage/src/keys.rs`              |
| Snapshot read/write API  | `vb_storage`  | `crates/vb_storage/src/snapshots.rs`         |
| Run-header scan API      | `vb_storage`  | `crates/vb_storage/src/headers.rs`           |
| Bounded keyspace preview | `vb_storage`  | `crates/vb_storage/src/preview.rs`           |
| Trimming error taxonomy  | `vb_storage`  | `crates/vb_storage/src/error/mod.rs` + `error/codes.rs` + `trimming/mod.rs` |

The bead is **strictly local to `vb_storage`**. No cross-crate API changes are
required.

---

## 2. Existing key-length constants (`crates/vb_storage/src/constants.rs`)

`pub(crate)` byte-length constants are already defined for every prefix variant:

| Constant                  | Value | Used by                                          |
|---------------------------|-------|--------------------------------------------------|
| `DIGEST_KEY_BYTES`        | `33`  | `WorkflowSource`/`CompiledIr`/`Blob` keys        |
| `RUN_ONLY_KEY_BYTES`      | `9`   | `RunHeader` key                                  |
| `JOURNAL_KEY_BYTES`       | `17`  | **`RunEvent` and `RunSnapshot` keys (the trim/snapshot keyspace)** |
| `INDEX_STATUS_KEY_BYTES`  | `18`  | `IndexStatus` key                                |
| `INDEX_WORKFLOW_KEY_BYTES`| `13`  | `IndexWorkflow`/`IndexAction` keys              |
| `INDEX_ACTION_KEY_BYTES`  | `13`  | `IndexAction` key                                |

There is currently **no public `MAX_TRIM_KEY_LEN` or `MAX_SNAPSHOT_KEY_LEN` alias**.
The trim and snapshot keyspace code paths use the bare integer literal `17`
inside `trimming/logic.rs` lines 36, 77, and 222 (see §3).

`constants.rs` lines of interest:

```rust
74: pub(crate) const JOURNAL_KEY_BYTES: usize = 17;
75: pub(crate) const DIGEST_KEY_BYTES: usize = 33;
76: pub(crate) const RUN_ONLY_KEY_BYTES: usize = 9;
77: pub(crate) const INDEX_STATUS_KEY_BYTES: usize = 18;
78: pub(crate) const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
79: pub(crate) const INDEX_ACTION_KEY_BYTES: usize = 13;
```

---

## 3. Hot call-graph blast radius — where the magic `17` lives

### 3.1 `crates/vb_storage/src/trimming/logic.rs`

| Line | Function                          | Behaviour today                                                                                  |
|------|-----------------------------------|--------------------------------------------------------------------------------------------------|
| 36   | `latest_durable_snapshot_seq`     | `if key.len() != 17 { return Err(TrimError::IncompleteTrim { deleted_count: 0 }); }`              |
| 43–46| `latest_durable_snapshot_seq`     | Calls `decode_storage_key(&key)` then matches on `StorageKey::RunSnapshot` — already typed decode |
| 77   | `trim_events_for_run`             | `if key.len() != 17 { return Err(TrimError::IncompleteTrim { deleted_count }); }` (loop body)     |
| 222  | `count_trimmable_events`          | `if key.len() != 17 { return Err(JournalError::from(TrimError::IncompleteTrim { deleted_count })); }` |

These three sites are the **primary targets** for the `MAX_TRIM_KEY_LEN` /
`MAX_SNAPSHOT_KEY_LEN` named-cap replacement. The current error is already typed
(`TrimError::IncompleteTrim`), wraps to `JournalError::Trim` via
`impl From<TrimError> for JournalError` in `error/mod.rs:187`, and surfaces the
diagnostic code `0x4102` (`TrimError::INCOMPLETE_TRIM_CODE`,
`trimming/mod.rs:62`).

### 3.2 `crates/vb_storage/src/trimming/helpers.rs`

```rust
3: pub(crate) fn snapshot_prefix_key(run: RunId) -> [u8; 9] {
4:     let prefix: [u8; 1] = [crate::constants::PREFIX_RUN_SNAPSHOT];
5:     let run_be: [u8; 8] = run.get().to_be_bytes();
6:     let mut key = [0u8; 9];
7:     let mut pos = 0usize;
8:     for &byte in prefix.iter().chain(run_be.iter()) { ... }
```

Returns the **9-byte** prefix used by `latest_durable_snapshot_seq`. Not directly
touched by the bead, but it is the cursor used when iterating the snapshot
keyspace; introducing a named `MAX_SNAPSHOT_KEY_LEN` should be paired with this
helpers file to keep the prefix and full-length constants co-located.

### 3.3 `crates/vb_storage/src/keys.rs`

The encoder (`run_snapshot_key`, line 86; `sequenced_run_key`, line 480; backed by
`ArrayVec<u8, JOURNAL_KEY_BYTES>`) cannot emit a non-`17`-byte key for the
journal/snapshot variants because the return type is a fixed-size array.
`run_event_key` (line 81) shares that property.

The decoder (`decode_storage_key`, line 346) already enforces
`bytes.len() != expected_len -> KeyDecodeError::KeyLengthMismatch` and
`KeysPrefix::expected_key_len()` (line 256) returns the canonical byte count for
each prefix. **No encoder fix is needed**; the bead is about the read/scanner
side, not about production inserts.

`decode_run_event_key` (line 451) wraps `decode_storage_key` and always collapses
any `KeyDecodeError` into `JournalError::MalformedKeyspaceRow { prefix: PREFIX_RUN_EVENT, expected_len: JOURNAL_KEY_BYTES, actual_len: bytes.len() }`. This is the
**precedent** for the typed-error shape the bead asks for.

---

## 4. Typed-error pattern already in place

The `run_headers` scan (`crates/vb_storage/src/headers.rs:49-82`) is the
canonical example of the typed-error pattern the bead asks for. It uses
`JournalError::MalformedKeyspaceRow` (defined in `error/mod.rs:98-105`,
diagnostic code `0x4030` in `error/codes.rs:95`) carrying
`{prefix, expected_len, actual_len}`:

```rust
67:         if key_len != crate::constants::RUN_ONLY_KEY_BYTES {
68:             return Err(JournalError::MalformedKeyspaceRow {
69:                 prefix: PREFIX_RUN_HEADER,
70:                 expected_len: crate::constants::RUN_ONLY_KEY_BYTES,
71:                 actual_len: key_len,
72:             });
73:         }
```

`preview.rs:146-180` (`malformed_to_journal_error`) translates every
`KeyDecodeError` variant into the same `MalformedKeyspaceRow` shape for the
`FailClosed` policy. The trim path should converge on this shape (the
bead's "typed error" requirement).

---

## 5. Tests that pin the current behaviour (preserved across the change)

| File                                                  | Test                                                                                                                 | Asserts                                                  |
|-------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------|
| `crates/vb_storage/src/snapshot_tests.rs:208-248`     | `latest_durable_snapshot_seq_rejects_malformed_overlong_key` (Round 10 issue 7)                                      | `Err(TrimError::IncompleteTrim { deleted_count: 0 })`    |
| `crates/vb_storage/src/trimming/tests.rs:875-932`     | `trim_events_for_run_fails_closed_on_malformed_event_key` (vb-1rqz7.26 / SC-006) — plants a 9-byte key              | `Err(TrimError::IncompleteTrim { .. })`                  |
| `crates/vb_storage/src/trimming/tests.rs:934-987`     | `trim_eligibility_diagnostic_fails_closed_on_malformed_event_key` (vb-1rqz7.25 / CC-002) — wraps `JournalError::Trim` | `TrimError::IncompleteTrim { .. }` after `From` chain     |
| `crates/vb_storage/src/preview/tests.rs:69-150`       | `preview_keyspace_skips_malformed` and `preview_keyspace_fails_closed`                                               | `MalformedKeyspaceRow { prefix: 0x10, expected_len: 9, actual_len: <n> }` |
| `crates/vb_storage/src/error_code_tests.rs:235-260`   | `count_trimmable_events` error code propagation                                                                     | IncompleteTrim code path                                 |

The 9-byte (short) key planted by the trim tests exercises the same code path
that would reject an overlong key: both fail the `key.len() != 17` check, so
**any combination of "shorter than 17" or "longer than 17" non-canonical keys
hits the same branch.** A new test for an explicit "longer than 17" key under
the snapshot prefix is the natural follow-up to `snapshot_tests.rs:214`.

---

## 6. Downstream / caller surface (no changes expected, but listed for diff awareness)

| Caller                                          | Public API used                    | Notes                                                                  |
|-------------------------------------------------|------------------------------------|------------------------------------------------------------------------|
| `crates/vb_storage/src/snapshots.rs:31-45`      | `run_snapshot_key(...)`            | Encoder side — fixed size by return type, no change needed             |
| `crates/vb_storage/src/journal/replay.rs:104`   | `latest_durable_snapshot_seq(...)` | Returns `TrimError::IncompleteTrim`; beads upstream of trim use only   |
| `crates/vb_runtime/` (cross-crate)              | (does not invoke trim APIs)        | NOT in scope                                                            |
| `crates/vb_cli/` (cross-crate)                  | (does not invoke trim APIs)        | NOT in scope                                                            |

The trim/snapshot keyspace is **owned by `vb_storage`**. No public API change
is required: the typed error already has a stable variant
(`TrimError::IncompleteTrim`) with a stable diagnostic code (`0x4102`).

---

## 7. Risk and verifier lane tags

| Tag                         | Where triggered                                            | Mandatory lane |
|-----------------------------|------------------------------------------------------------|----------------|
| `risk: parser/codec`        | Keys module `decode_storage_key`, `decode_run_event_key`   | proptest + unit |
| `risk: persistence`         | Fjall `put_snapshot`, trim loops reading raw key bytes     | proptest + integration |
| `risk: public API`          | `TrimError::IncompleteTrim` shape; diagnostic code 0x4102  | unit            |
| `risk: error taxonomy`      | New `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` constants and the typed path that consumes them | unit |
| `risk: fuzz`                | NOT in scope (pure type-level + numeric-only field)       | not required    |
| `risk: concurrency`         | NOT in scope (no shared state)                             | not required    |
| `risk: temporal`            | NOT in scope (no recovery from wrong snapshot)             | not required    |

Kani lanes are **not** required: this is a numeric/bounds refinement against an
already-bounded `JOURNAL_KEY_BYTES = 17`. The Kani harness family
(`kani_vb_vzcuf_ps*.rs`) and `kani_vbjpq733_proofs.rs` may be augmented only if
introducing a public newtype — none is currently planned.

---

## 8. Open questions the planner must resolve

1. **Re-use `JOURNAL_KEY_BYTES` or alias?** The bead text suggests
   `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN`. Three options:
   - (a) Add the two new caps as `pub const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;`
     and `pub const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;` aliases in
     `constants.rs` (declaration site, with docs explaining "trim keys for journal
     events" and "snapshot keys").
   - (b) Replace `key.len() != 17` with `key.len() != JOURNAL_KEY_BYTES` (no
     new symbols, but addresses the "magic 17" critique).
   - (c) Both — add the aliases and rewrite the call sites in
     `trimming/logic.rs` to use them.
   Planner should pick the option that best matches the bead wording
   ("MAX_TRIM_KEY_LEN, MAX_SNAPSHOT_KEY_LEN") — likely option (a) or (c).
2. **Typed-error target**: confirm whether the existing
   `TrimError::IncompleteTrim { deleted_count }` already satisfies "typed
   error", or whether the planner wants the branch to also surface a
   `JournalError::MalformedKeyspaceRow` analogue (mirroring `run_headers`).
   The existing `IncompleteTrim` is **already typed** (thiserror, carrying
   `deleted_count`), and its diagnostic code is `0x4102`. The `run_headers`
   precedent uses `0x4030` (`MALFORMED_KEYSPACE_ROW_CODE`).
3. **Truncate vs. reject** on overlong? The existing tests reject (fail closed);
   the planner must keep that semantic, not silently truncate to 17 bytes.

---

## 9. UNKNOWN / MISSING

- **UNKNOWN**: Whether the bead ships a new public type (e.g.,
  `pub struct TrimKeyLen(usize)` newtype). The verifier-only lane list above
  assumes no newtype. Confirm with the orchestrator before the proof lane.
- **MISSING**: No `Cargo.toml` change is expected; this is internal-only.
- **MISSING**: No `Moonfile` / `.moon/` task should change.

---

## 10. Quick file-pointer index

```
crates/vb_storage/src/constants.rs                                # add MAX_TRIM_KEY_LEN / MAX_SNAPSHOT_KEY_LEN
crates/vb_storage/src/keys.rs                                    # decoder already enforces (no change)
crates/vb_storage/src/keys/tests.rs                              # existing length tests
crates/vb_storage/src/trimming/mod.rs                            # TrimError::IncompleteTrim shape
crates/vb_storage/src/trimming/logic.rs                          # lines 36, 77, 222 — magic-17 sites
crates/vb_storage/src/trimming/helpers.rs                        # 9-byte prefix (co-locate with caps)
crates/vb_storage/src/trimming/tests.rs                          # lines 875-987 — failing-closed tests
crates/vb_storage/src/snapshots.rs                               # put_snapshot / snapshot (no encoder change)
crates/vb_storage/src/snapshot_tests.rs                          # line 208-248 — overlong regression
crates/vb_storage/src/headers.rs                                  # lines 49-82 — typed-error precedent
crates/vb_storage/src/preview.rs                                 # lines 146-180 — KeyDecodeError->MalformedKeyspaceRow
crates/vb_storage/src/error/mod.rs                               # MalformedKeyspaceRow variant (line 101)
crates/vb_storage/src/error/key_decode.rs                        # KeyDecodeError enum
crates/vb_storage/src/error/codes.rs                             # diagnostic codes (0x4030 / 0x4102)
```

END OF CODEBASE MAP.
