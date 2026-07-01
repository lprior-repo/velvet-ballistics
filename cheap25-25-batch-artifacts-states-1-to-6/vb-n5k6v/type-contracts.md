# Type Contracts — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Overview

This bead produces **exactly one new token** at the Rust type level: a
module declaration. The type contract is therefore exceptionally small
and focused on (a) the declaration's syntactic shape, (b) its visibility
attributes, (c) the file-internal module body it points at, and (d) the
types referenced by the 26 tests it surfaces.

No new types are introduced. No existing types are modified. The
contract is purely a **build-graph** contract, not a runtime contract.

---

## Module Declaration Type Contract

### `mod edge_case_tests;` (the only new declaration)

```rust
// crates/vb_storage/src/lib.rs — INSERTED between line 181 (end of
// `mod snapshot_tests;`) and line 183 (`pub mod queue;`)
#[cfg(test)]
#[path = "edge_case_tests.rs"]
mod edge_case_tests;
```

#### Type-Level Field-by-Field Contract

| Field | Value | Type | Visibility | Stability |
|-------|-------|------|------------|-----------|
| `#[cfg(test)]` | attribute | cfg-attribute | n/a | locked — must remain `cfg(test)` |
| `#[path = "edge_case_tests.rs"]` | attribute | path-attribute | n/a | locked — must match filename exactly |
| `mod edge_case_tests;` | item | module-declaration | private (no `pub`) | locked — must match inner wrapper at file line 12 |

#### Pre-conditions

1. `crates/vb_storage/src/edge_case_tests.rs` exists at the path
   resolved relative to `lib.rs` (i.e., same directory). Verified: file
   is present at `crates/vb_storage/src/edge_case_tests.rs` (637 lines).
2. The file contains a top-level `mod edge_case_tests { ... }` wrapper
   (file lines 11-12) so the inner module name matches the outer
   declaration name. Verified.
3. `crates/vb_storage/Cargo.toml` declares `tempfile` and `proptest` as
   `dev-dependencies` (already present at lines 19-21). Verified.
4. All public/`pub(crate)` symbols used by the 26 tests are resolvable
   from `crates/vb_storage/src/lib.rs` (verified §6 of
   `codebase-map.md`: 32 distinct symbols, all resolved to production
   source).
5. The 16 sibling `#[path = "..."]` declarations at lines 118-181 of
   `lib.rs` are untouched. Verified.

#### Post-conditions

1. `cargo check -p vb_storage --tests` succeeds with zero new warnings.
2. `cargo test -p vb_storage --lib --no-run` compiles the
   `edge_case_tests` module and links it into the test binary.
3. `cargo test -p vb_storage --lib edge_case` runs all 26 tests and
   reports them under the `edge_case_tests::` module path.
4. `cargo test -p vb_storage --lib` test count increases by exactly 26
   relative to pre-wire baseline (924 → 950).

#### Forbidden Patterns

- ❌ Adding `pub` to the declaration: would expose the test module to
  downstream crates and violate the project's `#![forbid(unsafe_code)]`
  policy boundary. (Not a safety issue, but a hygiene issue: tests are
  not part of the public API.)
- ❌ Adding `#[cfg(not(test))]`: would compile test code into
  production binaries, violating the bead's test-only scope.
- ❌ Renaming `edge_case_tests` to a different module name: would
  mismatch the inner wrapper at file line 12 and break compilation.
- ❌ Using a different filename: would require moving the file, which
  would change git history of the existing 637-line source-length
  exception.
- ❌ Combining the declaration with any other change to `lib.rs`: the
  blast radius must be exactly 3 lines added (3 lines removed = 0).

#### Equivalence Class

The declaration is **structurally equivalent** to the 16 existing
sibling declarations at `lib.rs:118-181`. Specifically:

| Sibling | File | Declaration site |
|---------|------|------------------|
| `error_tests` | `crates/vb_storage/src/error_tests.rs` | `lib.rs:123` |
| `error_code_tests` | `crates/vb_storage/src/error_code_tests.rs` | `lib.rs:127` |
| `type_tests` | `crates/vb_storage/src/type_tests.rs` | `lib.rs:131` |
| `index_tests` | `crates/vb_storage/src/index_tests.rs` | `lib.rs:135` |
| `index_maintenance_tests` | `crates/vb_storage/src/index_maintenance_tests.rs` | `lib.rs:140` |
| `artifact_tests` | `crates/vb_storage/src/artifact_tests.rs` | `lib.rs:144` |
| `blob_tests` | `crates/vb_storage/src/blob_tests.rs` | `lib.rs:148` |
| `header_tests` | `crates/vb_storage/src/header_tests.rs` | `lib.rs:152` |
| `hydrate_tests` | `crates/vb_storage/src/hydrate_tests.rs` | `lib.rs:156` |
| `process_lock_tests` | `crates/vb_storage/src/process_lock_tests.rs` | `lib.rs:160` |
| `record_tests` | `crates/vb_storage/src/record_tests.rs` | `lib.rs:164` |
| `recover_tests` | `crates/vb_storage/src/recover_tests.rs` | `lib.rs:168` |
| `recovery_type_tests` | `crates/vb_storage/src/recovery_type_tests.rs` | `lib.rs:172` |
| `replay_core_tests` | `crates/vb_storage/src/replay_core_tests.rs` | `lib.rs:176` |
| `snapshot_tests` | `crates/vb_storage/src/snapshot_tests.rs` | `lib.rs:180` |
| `edge_case_tests` (NEW) | `crates/vb_storage/src/edge_case_tests.rs` | `lib.rs:182` (between lines 181 and 183) |

The 17th sibling slot is the entire scope of this bead.

---

## Inner Module Wrapper (file-owned, preserved verbatim)

```rust
// crates/vb_storage/src/edge_case_tests.rs:11-12 (UNCHANGED)
#[cfg(test)]
mod edge_case_tests {
    // ... 26 test functions ...
}
```

- The outer `#[cfg(test)]` on the file-level wrapper is **redundant**
  with the `#[cfg(test)]` on the outer declaration in `lib.rs:182` (Rust
  requires test-mode to even see the file). The redundancy is
  preserved verbatim — it is a defensive pattern matching all 16 sibling
  files (`tests.rs:11-12`, `error_tests.rs:11-12`, etc.).
- The inner `mod edge_case_tests { ... }` body is the **canonical Rust
  2018+ in-file module pattern** used to scope `use` statements and
  helpers (e.g., `temp_journal` at file lines 25-29) without polluting
  the crate namespace.
- The inner module name MUST be `edge_case_tests` — it is the symbol
  path under which cargo test will report each test
  (`edge_case_tests::persist_strict_handles_simulated_failure`,
  etc.).

---

## Symbol Resolution Contract (32 symbols referenced by 26 tests)

The contract guarantees that **all** symbols used by the 26 tests
resolve to live, public or `pub(crate)`, in-crate APIs at the time of
wire. Verified `codebase-map.md` §6 and `delivery-scope.jsonl` rows
4-46:

### FjallJournal methods (12 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `FjallJournal::open(path, None)` | `pub` | `journal/core.rs:79` |
| `FjallJournal::close()` | `pub` | `journal/core.rs:222` |
| `FjallJournal::fail_next_persist_for_test()` | `pub(crate)` | `journal/core.rs:227` |
| `FjallJournal::append_journaled(&event)` | `pub` | `journal/append.rs:7` |
| `FjallJournal::append_strict(&event)` | `pub` | `journal/append.rs:35` |
| `FjallJournal::persist_strict()` | `pub` | `journal/append.rs:81` |
| `FjallJournal::events_for_run(run)` | `pub` | `journal/replay.rs:59`, `readonly.rs:70` |
| `FjallJournal::put_blob(&record)` | `pub` | `blobs.rs:20` |
| `FjallJournal::blob(digest)` | `pub` | `blobs.rs:35` |
| `FjallJournal::put_workflow_source(&record)` | `pub` | `journal/source.rs:20` |
| `FjallJournal::put_compiled_ir(&record)` | `pub` | `journal/source.rs:53` |
| `FjallJournal::put_snapshot(&snapshot)` | `pub` | `snapshots.rs:31` |
| `FjallJournal::put_run_header(&header)` | `pub` | `headers.rs:18` |
| `FjallJournal::batch()` | `pub` | `batch/mod.rs` |
| `FjallJournal::snapshot(run, seq)` | `pub` | `snapshots.rs` |
| `FjallJournal::run_header(run)` | `pub` | `headers.rs` |
| `FjallJournal::workflow_source(digest)` | `pub` | `journal/source.rs:35` |
| `FjallJournal::compiled_ir(digest)` | `pub` | `journal/source.rs:68` |

### BatchBuilder methods (5 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `BatchBuilder::append_event(&event)` | `pub` | `batch/append_event.rs:42` |
| `BatchBuilder::put_workflow_source(&record)` | `pub` | `batch/putters.rs:23` |
| `BatchBuilder::strict()` | `pub` | `batch/commit.rs:7` |
| `BatchBuilder::commit()` | `pub` | `batch/commit.rs:20` |
| `BatchBuilder::len()` | `pub` | `batch/types.rs:48` |

### JournalWriterQueue methods (6 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `JournalWriterQueue::new(cap, batch, limits)` | `pub` | `queue/writer.rs:40` |
| `JournalWriterQueue::enqueue_journaled(event)` | `pub` | `queue/writer.rs:67` |
| `JournalWriterQueue::enqueue_strict(event)` | `pub` | `queue/writer.rs:72` |
| `JournalWriterQueue::flush_batch(&journal)` | `pub` | `queue/writer.rs:152` |
| `JournalWriterQueue::drain_all(&journal)` | `pub` | `queue/writer.rs:237` |
| `JournalWriterQueue::shutdown(&journal)` | `pub` | `queue/writer.rs:266` |

### Codec functions (2 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `encode_record(magic, kind, len, record, max)` | `pub` | `codec/mod.rs:60` |
| `decode_record::<T>(bytes, magic, max_payload_len)` | `pub` | `codec/mod.rs:82` |

### Constants (6 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `MAGIC_BLOB` | `pub` | `constants.rs:66` |
| `MAGIC_INDEX_RECORD` | `pub` | `constants.rs:72` |
| `MAGIC_JOURNAL_EVENT` | `pub` | `constants.rs:62` |
| `MAX_RUN_HEADER_BYTES` | `pub` | `constants.rs:94` |
| `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | `pub` | `constants.rs:88` |
| `DIGEST_BYTES` | `pub` | `constants.rs:82` |

### RecordKind variants (5 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `RecordKind::WorkflowSource` | `pub` | `records.rs:141` |
| `RecordKind::RunHeader` | `pub` | `records.rs:145` |
| `RecordKind::RunAccepted` | `pub` | `records.rs:147` |
| `RecordKind::IndexUpdate` | `pub` | `records.rs:204` |
| `RecordKind::Blob` | `pub` | `records.rs:202` |

### StorageLimits and Records (3 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `StorageLimits::DEFAULT` | `pub` | `types.rs:17` |
| `BlobRecord` | `pub` | `records.rs` |
| `CompiledIrRecord` | `pub` | `records.rs` |
| `EventSeq` | `pub` | `events.rs` |
| `JournalEvent` | `pub` | `events.rs` |
| `JournalWriterQueue` | `pub` | `queue/writer.rs` |
| `RunHeaderRecord` | `pub` | `records.rs` |
| `RunSnapshot` | `pub` | `records.rs` |
| `WorkflowSourceRecord` | `pub` | `records.rs` |
| `FjallJournal` | `pub` | `journal/core.rs` |

### vb_core newtypes (4 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `vb_core::RunId` | `pub` | `vb_core/src/lib.rs` |
| `vb_core::StepIdx` | `pub` | `vb_core/src/lib.rs` |
| `vb_core::WorkflowDigest` | `pub` | `vb_core/src/lib.rs` |
| `vb_core::WorkflowId` | `pub` | `vb_core/src/lib.rs` |

### Error variants (5 symbols)

| Symbol | Visibility | Production site |
|--------|------------|-----------------|
| `JournalError::StrictDurabilityFailed` | `pub` | `error/mod.rs` (returned at `journal/append.rs:84`) |
| `JournalError::PayloadTooLarge` | `pub` | `error/mod.rs` |
| `JournalError::RecordKindFamilyMismatch { magic, kind }` | `pub` | `error/mod.rs:80` |
| `JournalError::DuplicateEvent` | `pub` | `error/mod.rs` (returned at `batch/append_event.rs:63`) |
| `JournalError::QueueShutdown` | `pub` | `error/mod.rs:45` (returned at `queue/writer.rs:82`) |

### External dev-deps (3 crates)

| Crate | Version source | Purpose |
|-------|----------------|---------|
| `tempfile` | workspace | `tempfile::tempdir()` helper at file lines 26, 359, 386, 401 |
| `proptest` | workspace | imported but not actively used in this file (vestigial dev-dep) |
| `blake3` | workspace (transitive via `vb_storage/Cargo.toml:9`) | `blake3::hash(&large)` at file lines 252, 266, 280 |
| `fjall` | workspace (transitive) | the entire Fjall keyspace |

### Resolution Guarantee

If **any** of the 32 symbols above disappears from production source
between contract acceptance and proof execution, the wire will FAIL TO
COMPILE. This is the safety property of the build-graph contract: drift
in production source is caught at compile time, never at runtime.

---

## Internal Helper Contract (preserved verbatim)

```rust
// crates/vb_storage/src/edge_case_tests.rs:25-29
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open");
    (temp, journal)
}
```

- 18 of the 26 tests call `temp_journal()` to spin up an isolated Fjall
  keyspace in a `tempfile::tempdir()` (the 8 open/close-cycle tests use
  a shared path instead, manually constructed at file lines 359-359,
  386-388, 401-403).
- Each test owns its own `tempfile::TempDir` via the RAII return — the
  directory is removed on test drop. No cross-test contamination.
- Visibility: `fn` (private to the `mod edge_case_tests` wrapper).
  Cannot leak to other test modules or to production code.

---

## Co-location Invariant

The wire declaration at `lib.rs:182` is positioned **immediately after**
the `snapshot_tests` declaration at `lib.rs:180` and **immediately
before** the `pub mod queue;` declaration at `lib.rs:183`. This
preserves the canonical pattern of grouping all `#[cfg(test)] #[path =
"..."] mod ...;` declarations together at the bottom of `lib.rs` (lines
107-181 currently) before the production `pub mod` declarations resume
at line 183.

**Forbidden**: positioning the wire declaration between any other two
`#[cfg(test)]` blocks or after the `pub mod` section. Such positioning
would not change behavior but would break the canonical ordering and
make future wave-3 audits harder.

---

END OF TYPE CONTRACTS.