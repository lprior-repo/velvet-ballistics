# Boundary Map — Storage Trim/Snapshot Key Length Cap (vb-d9ml3)

## Overview

The bead is strictly **internal to `vb_storage`** with a single pure-core
change (named constants) and an imperative-shell change (replace three magic
literals). No cross-crate, async, network, FFI, or unsafe boundary is
touched.

```
                     +-----------------------------+
                     |  vb_runtime (upstream)      |
                     |  does NOT call trim APIs    |
                     +-----------------------------+
                                     |
                                     |  (no call edge)
                                     v
+-------------------------------------------------------------+
|                    vb_storage crate                          |
|                                                             |
|  +----------------------+   +---------------------------+   |
|  |  Pure core           |   |  Imperative shell         |   |
|  |  - constants.rs      |   |  - trimming/logic.rs      |   |
|  |    JOURNAL_KEY_BYTES |   |    latest_durable_        |   |
|  |    MAX_TRIM_KEY_LEN  |<--+    snapshot_seq:36        |   |
|  |    MAX_SNAPSHOT_     |   |    trim_events_for_       |   |
|  |      KEY_LEN         |   |      run:77               |   |
|  |  - keys.rs           |   |    count_trimmable_       |   |
|  |    encode (frozen)   |   |      events:222           |   |
|  |    decode (frozen)   |   |  - trimming/helpers.rs    |   |
|  +----------------------+   |    snapshot_prefix_key    |   |
|                             +---------------------------+   |
|                                       |                     |
|                                       v                     |
|                             +---------------------------+   |
|                             |  Storage boundary         |   |
|                             |  Fjall (run_event,        |   |
|                             |  run_snapshot partitions) |   |
|                             +---------------------------+   |
+-------------------------------------------------------------+
                                     |
                                     v
                             +---------------------------+
                             |  Disk (LSM tree)          |
                             +---------------------------+
```

---

## Boundary 1 — Pure Core: Constants

**Location**: `crates/vb_storage/src/constants.rs:74-79`

**Responsibility**: Declare the canonical byte lengths for each keyspace.
The post-fix module contains:

```rust
pub(crate) const JOURNAL_KEY_BYTES: usize = 17;       // source of truth
pub(crate) const MAX_TRIM_KEY_LEN: usize = JOURNAL_KEY_BYTES;       // NEW
pub(crate) const MAX_SNAPSHOT_KEY_LEN: usize = JOURNAL_KEY_BYTES;   // NEW
pub(crate) const DIGEST_KEY_BYTES: usize = 33;
pub(crate) const RUN_ONLY_KEY_BYTES: usize = 9;
pub(crate) const INDEX_STATUS_KEY_BYTES: usize = 18;
pub(crate) const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
pub(crate) const INDEX_ACTION_KEY_BYTES: usize = 13;
pub(crate) const _RUN_EVENT_PREFIX_BYTES: usize = 9;
```

**Boundary property**: 100% pure. No I/O, no time, no random, no FFI.
Compile-time-evaluated. No side effects.

**I/O surface**: zero.

**Testable via**: `cargo check` (compile-time equality).

---

## Boundary 2 — Imperative Shell: Trim Scanners

**Location**: `crates/vb_storage/src/trimming/logic.rs:26-241`

**Responsibility**: Iterate a Fjall keyspace prefix; verify each raw key's
length against the named cap; produce a typed error on the first
non-canonical observation.

**Inputs**:
- `&self` (immutable borrow of `FjallJournal`).
- `run: RunId`.
- `policy: TrimPolicy` (for `trim_events_for_run` and
  `trim_eligibility_diagnostic`).

**Outputs**:
- `TrimResult<Option<EventSeq>>` from `latest_durable_snapshot_seq`.
- `TrimResult<TrimmedRunResult>` from `trim_events_for_run`.
- `Result<u64, JournalError>` from `count_trimmable_events`.

**Side effects**: 
- `trim_events_for_run` may commit an LSM batch on `Ok`.
- All other paths are read-only.

**Length-check sites** (the three sites to be patched):

| Site | Function | Line | Cap to use |
|------|----------|------|------------|
| S1 | `latest_durable_snapshot_seq` | `logic.rs:36` | `MAX_SNAPSHOT_KEY_LEN` |
| S2 | `trim_events_for_run` | `logic.rs:77` | `MAX_TRIM_KEY_LEN` |
| S3 | `count_trimmable_events` | `logic.rs:222` | `MAX_TRIM_KEY_LEN` |

**Forbidden at these sites**:

| Forbidden | Why |
|-----------|-----|
| `key.len() != 17` (magic literal) | Loses the named-cap invariant; bypasses the alias chain. |
| `key.len() < MAX_*_KEY_LEN \|\| key.len() > MAX_*_KEY_LEN` (verbose) | Functionally equivalent to `!=`; less idiomatic. |
| `key.len() < JOURNAL_KEY_BYTES` (under-cap) | Asymmetric: would let an overlong key pass. |
| `key.len() <= MAX_*_KEY_LEN` (off-by-one) | Would let a too-long key pass. |
| Panicking (`assert_eq!`, `unreachable!`) | Holzmann-Rust forbids panic at storage boundary. |

---

## Boundary 3 — Storage Boundary: Fjall

**Location**: `crates/vb_storage/src/keys.rs:86` (encoder),
`crates/vb_storage/src/trimming/logic.rs:74, 217` (scanner iterators).

**Responsibility**: Translate between `StorageKey` enum and the Fjall
byte-slice representation.

**Contract**:

- **Encoder side** (`run_event_key`, `run_snapshot_key`): return type is
  `[u8; JOURNAL_KEY_BYTES]`. By the type system, the encoder **cannot**
  emit a non-canonical length. No change is required at the encoder.
- **Decoder side** (`decode_storage_key`): already enforces
  `bytes.len() == expected_len` via `KeyDecodeError::KeyLengthMismatch`
  (`keys.rs:349-355`). No change is required at the decoder.
- **Iterator side** (Fjall `prefix()` cursors): returns raw `(key, value)`
  pairs whose `key.len()` may be ANY value, including values that violate
  the keyspace contract (legacy rows, corrupt LSMtree payloads, prefix
  collisions). The scanner MUST tolerate any `key.len()` and surface a typed
  error on non-canonical lengths.

**Out-of-scope** (NOT touched by this bead):
- Fjall schema migration.
- LSMtree compaction tuning.
- On-disk repair of corrupt rows.

---

## Boundary 4 — Parser Boundary: Typed Keyspace Decoding

**Location**: `crates/vb_storage/src/keys.rs:346-433` (`decode_storage_key`).

**Responsibility**: Translate raw bytes into a `StorageKey` enum.

**Contract preservation**: The decoder's `KeyLengthMismatch` arm is the
**secondary** safety net at `latest_durable_snapshot_seq:43`. The contract
preserves the call site; the primary cap check at line 36 fires first.

**Reference pattern** (NOT modified): `headers.rs:67-72` uses
`JournalError::MalformedKeyspaceRow` for the run-header keyspace. The trim
path follows the **`TrimError::IncompleteTrim`** contract per the bead's
typed-error choice (see `error-taxonomy.md`).

---

## Boundary 5 — Test Boundary: temp_journal() Helper

**Location**: `crates/vb_storage/src/trimming/tests.rs:17-21`,
`crates/vb_storage/src/snapshot_tests.rs:16-20`.

**Responsibility**: Spin up a tempdir-backed Fjall journal for integration
testing.

**Contract preservation**: The helper is contractually frozen for this
bead. New tests use the same helper to plant raw keys with arbitrary
lengths under the appropriate prefix. The contract pins the planting
recipe (see `hazard-analysis.md` §6).

---

## Boundary 6 — Public API Boundary (NOT touched)

**Location**: `crates/vb_storage/src/trimming/mod.rs:30-55` (TrimError),
`crates/vb_storage/src/error/mod.rs:21-188` (JournalError).

**Responsibility**: Stable error types with stable diagnostic codes.

**Contract preservation**:

- `TrimError::IncompleteTrim { deleted_count: u64 }` shape: **unchanged**.
- `TrimError::INCOMPLETE_TRIM_CODE == 0x4102`: **unchanged**.
- `JournalError::Trim(Box<TrimError>)` delegation chain: **unchanged**.
- `JournalError::diagnostic_code()` returns `0x4102` for the trim chain:
  **unchanged**.

No public API surface changes. No downstream caller (vb_runtime, vb_cli,
vb_core) is affected.

---

## Boundary 7 — Cross-Crate Boundaries (NOT touched)

| Crate | Touches trim API? | Impact |
|-------|-------------------|--------|
| `vb_core` | No (no trim API) | None |
| `vb_runtime` | No (per codebase-map.md §6) | None |
| `vb_cli` | No (per codebase-map.md §6) | None |
| `vb_validate` | No | None |

The bead is **strictly local to `vb_storage`**. The contract binds the
implementation to make zero cross-crate changes.

---

## Boundary Cross-Reference: Existing Precedent (`headers.rs`)

For comparison, the `run_header` keyspace uses the
`MalformedKeyspaceRow` typed-error pattern (`headers.rs:67-72`):

```rust
if key_len != crate::constants::RUN_ONLY_KEY_BYTES {
    return Err(JournalError::MalformedKeyspaceRow {
        prefix: PREFIX_RUN_HEADER,
        expected_len: crate::constants::RUN_ONLY_KEY_BYTES,
        actual_len: key_len,
    });
}
```

The trim path COULD have followed this pattern but the bead's typed-error
choice (preserve `IncompleteTrim`) keeps the trim-specific counter
(`deleted_count`) available to callers. Future work in other keyspaces
should evaluate the trade-off:

| Aspect | `MalformedKeyspaceRow` (headers.rs precedent) | `IncompleteTrim` (trim path, this bead) |
|--------|-----------------------------------------------|------------------------------------------|
| Counter | No | Yes (`deleted_count: u64`) |
| Prefix | Yes | No (caller infers from stack) |
| Expected length | Yes | No |
| Actual length | Yes | No |
| Diagnostic code | `0x4030` | `0x4102` |
| Symbolic code | `MALFORMED_KEYSPACE_ROW` | `JOURNAL_INCOMPLETE_TRIM` |

The bead opts for `IncompleteTrim` because the trim counter (`deleted_count`)
is the primary diagnostic signal (how much progress did the abort cost?).
For other keyspaces without a progress counter, `MalformedKeyspaceRow` is
the right choice.

---

## Summary — Boundaries Touched vs. Untouched

| Boundary | Status |
|----------|--------|
| Pure core (constants) | **TOUCHED** (add two `pub(crate) const` aliases) |
| Imperative shell (trim scanners) | **TOUCHED** (replace three magic literals with named caps) |
| Storage boundary (Fjall) | untouched |
| Parser boundary (typed keyspace decoding) | untouched |
| Test boundary (temp_journal helper) | untouched |
| Public API boundary (error variants + codes) | untouched |
| Cross-crate boundaries | untouched |

The contract's blast radius is **two files**: `constants.rs` (declarations)
and `trimming/logic.rs` (three replacements). All other files are
read-only references for the proof planner and test writer.

END OF BOUNDARY MAP.