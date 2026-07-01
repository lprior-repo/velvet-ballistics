# Boundary Map — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Overview

The bead is **strictly a build-graph repair inside `vb_storage`** with
zero production logic touched. The boundary map shows the existing
test-orchestration layers in `vb_storage/src/` and pinpoints the
**single boundary** the wire modifies: the `lib.rs` ↔
`<name>_tests.rs` compile-graph entry.

There is **no** change to:

- Pure core (constants, encoders, decoders)
- Imperative shell (Fjall I/O, batch writers, queue workers)
- Async shell (no async code in `edge_case_tests.rs`)
- Storage boundary (the Fjall keyspace is exercised but not modified)
- Network / time / FFI / unsafe boundaries (none are touched)

```
                       (production runtime — unchanged)
                       ============================

                       +-----------------------------+
                       |  vb_runtime (upstream)      |
                       |  does NOT touch test wiring |
                       +-----------------------------+
                                       |
                                       |  (no call edge to edge_case_tests)
                                       v
+------------------------------------------------------------------+
|                       vb_storage crate                            |
|                                                                  |
|  +-------------------+        +---------------------------+       |
|  |  Pure core        |        |  Imperative shell         |       |
|  |  - constants.rs   |        |  - journal/core.rs        |       |
|  |  - records.rs     |        |  - journal/append.rs      |       |
|  |  - codec/mod.rs   |        |  - batch/                 |       |
|  |  - error/codes.rs |        |  - queue/writer.rs        |       |
|  +-------------------+        |  - headers.rs, blobs.rs   |       |
|              ^                |  - snapshots.rs           |       |
|              |                |  - source.rs              |       |
|              |                +---------------------------+       |
|              |                            |                       |
|              +--------------+-------------+
|                             |
|                             v
|                   +---------------------------+
|                   |  Storage boundary          |       <-- Fjall keyspace
|                   |  Fjall (run_event,        |       (exercised by tests,
|                   |  run_snapshot, blob, etc) |        unchanged by bead)
|                   +---------------------------+
+------------------------------------------------------------------+
                                |
                                v
                       +---------------------------+
                       |  Disk (LSM tree)          |
                       +---------------------------+


                       (test orchestration — THIS BEAD)
                       ===================================

+------------------------------------------------------------------+
|                  crates/vb_storage/src/lib.rs                    |
|                                                                  |
|  Lines 107-181: 16 existing #[cfg(test)] #[path = "..."] mod ...; |
|  Line 182 (NEW): #[cfg(test)] #[path = "edge_case_tests.rs"]      |
|                  mod edge_case_tests;                            |
|  Line 183: pub mod queue; (unchanged)                            |
|                                                                  |
|              +--------------+--------------+
|                             |
|                             v (#[path = "edge_case_tests.rs"])
|                                                                  |
|  +-------------------------------------------------------------+ |
|  |  crates/vb_storage/src/edge_case_tests.rs (637 lines)        | |
|  |                                                             | |
|  |  #![allow(clippy::as_conversions, ...)] (file-level, lines 1-9) | |
|  |  #[cfg(test)]                                              | |
|  |  mod edge_case_tests {                                     | |
|  |      use crate::{ BlobRecord, ... };                       | |
|  |      fn temp_journal() -> (TempDir, FjallJournal) { ... } | |
|  |      #[test] fn persist_strict_handles_simulated_failure(){| |
|  |          // 26 test functions across 7 topic buckets       | |
|  |      }                                                     | |
|  |  }                                                         | |
|  +-------------------------------------------------------------+ |
+------------------------------------------------------------------+
```

---

## Boundary 1 — Compile-Graph Entry (THE bead's only boundary)

**Type**: In-source `#[cfg(test)]` module declaration
**Site**: `crates/vb_storage/src/lib.rs:182` (3 inserted lines)
**Direction**: Outbound from `lib.rs` to `edge_case_tests.rs`
**Effect**: `cargo test -p vb_storage --lib` discovers 26 additional
test functions; without the declaration, the file is compiled to
nothing.

### Inbound Contracts (what the wire GUARANTEES to the file)

| Guarantee | Holder |
|-----------|--------|
| `tempfile`, `proptest` are resolvable from the crate root | `Cargo.toml:19-21` (dev-deps) |
| `blake3` is resolvable as a transitive dep | `Cargo.toml:9` |
| All 32 symbols used by the 26 tests resolve to live production source | `crate::*` import at file lines 13-23 |
| The `mod edge_case_tests { ... }` inner wrapper matches the outer declaration name | file lines 11-12 |

### Outbound Contracts (what the file GUARANTEES to lib.rs)

| Guarantee | Holder |
|-----------|--------|
| All `#[test]` fns are inside the inner wrapper | file lines 11-637 |
| No top-level `#[test]` fns escape the inner wrapper | grep verifies |
| All `use` statements are inside the inner wrapper | file lines 13-23 |
| The file-level `#![allow(...)]` block is the same shape as the 16 sibling files | file lines 1-9 |
| No `unsafe`, `panic!`, `unwrap()` is at file scope (all inside `#[test]` or `fn temp_journal`) | file structure |

### Failure Modes

| Failure | Detection |
|---------|-----------|
| `mod` name mismatch (e.g., `mod edge_case_test;`) | `cargo check -p vb_storage --tests` reports E0432 (cannot find module) |
| `#[path]` typo (e.g., `#[path = "edge_case_test.rs"]`) | `cargo check` reports E0432 or file-not-found |
| Missing `#[cfg(test)]` | `cargo build -p vb_storage` (non-test build) fails: test code references test-only imports |
| Adding `pub` to the declaration | No compile error, but violates project hygiene (`pub` test modules expose internals) |

---

## Boundary 2 — Pure Core (UNCHANGED)

The 26 tests use these pure-core types:

- `RunId`, `StepIdx`, `WorkflowDigest`, `WorkflowId` (from `vb_core`)
- `EventSeq`, `RecordKind`, `StorageLimits`, `BlobRecord`,
  `CompiledIrRecord`, `RunHeaderRecord`, `RunSnapshot`,
  `WorkflowSourceRecord` (from `vb_storage`)
- `MAGIC_BLOB`, `MAGIC_INDEX_RECORD`, `MAGIC_JOURNAL_EVENT`,
  `MAX_RUN_HEADER_BYTES`, `MAX_JOURNAL_EVENT_PAYLOAD_BYTES`,
  `DIGEST_BYTES` (from `vb_storage::constants`)
- `JournalError` (from `vb_storage::error`)
- `encode_record`, `decode_record` (from `vb_storage::codec`)

**None of these types are modified by the bead.** The wire only
increases the *consumer* count for these types (from 16 modules to
17 modules); the producer side is untouched.

---

## Boundary 3 — Imperative Shell (EXERCISED, NOT MODIFIED)

The 26 tests exercise these imperative-shell methods:

- `FjallJournal::open`, `close`, `append_journaled`, `append_strict`,
  `persist_strict`, `events_for_run`, `put_blob`, `blob`,
  `put_workflow_source`, `workflow_source`, `put_compiled_ir`,
  `compiled_ir`, `put_snapshot`, `snapshot`, `put_run_header`,
  `run_header`, `batch` (16 methods)
- `BatchBuilder::append_event`, `put_workflow_source`, `strict`,
  `commit`, `len` (5 methods)
- `JournalWriterQueue::new`, `enqueue_journaled`, `enqueue_strict`,
  `flush_batch`, `drain_all`, `shutdown` (6 methods)

**None of these methods are modified by the bead.** The wire only
increases the *caller* count for these methods; the methods themselves
are unchanged.

---

## Boundary 4 — Storage Boundary (Fjall keyspace, EXERCISED)

The 22 of 26 tests that use `temp_journal()` open a fresh Fjall
keyspace in a `tempfile::tempdir()`. The Fjall keyspace is the
storage boundary:

```
       test code (in-process)
              |
              v
       FjallJournal::open(path, None)            <-- storage boundary
              |
              v
       Fjall LSM tree (file-backed, ephemeral)
              |
              v
       TempDir RAII drop on test end
```

**Effect of wire**: 22 tests now create and tear down Fjall keyspaces
per `cargo test` invocation, instead of 0. This is a one-time increase
in test-run wall-clock time (estimated: +5 to +15 seconds on a
workstation; +30 to +60 seconds on slow CI disks). The contract
acknowledges this performance delta as a known cost of restoring
the wave-3 dormant coverage.

**Mitigation if CI budget exceeded**: per-test `tempfile::tempdir()`
isolation already prevents cross-test contamination. The planner may
mark slow tests with `#[ignore]` if CI budget pressure arises, but
this is **out of scope** for the wire-only fix.

---

## Boundary 5 — Time Boundary (NOT TOUCHED)

The 26 tests do **not** read the system clock, do not call
`std::time::Instant::now()`, and do not use tokio timers. The only
"time" they encounter is the implicit wall-clock time of
`tempfile::tempdir()` creation/destruction and the Fjall LSM tree
flush — both are filesystem operations, not test-controlled.

---

## Boundary 6 — Concurrency Boundary (EXERCISED, 4 of 26 tests)

The 4 concurrent tests use `std::thread::spawn` with
`Arc<FjallJournal>` and `Arc<JournalWriterQueue>`. The concurrency
boundary is:

```
       test main thread
              |
              +-- Arc::clone(&journal) ---> thread 1: append_journaled
              +-- Arc::clone(&journal) ---> thread 2: append_journaled
              +-- Arc::clone(&journal) ---> thread 3: append_journaled
              +-- Arc::clone(&journal) ---> thread 4: append_journaled
              |                          ...
              +-- Arc::clone(&queue)   ---> thread N: enqueue_journaled
              |
              v
       handle.join().expect(...)  for each thread
```

**Synchronization primitives**: none at the test layer. The
production code's interior mutability (Fjall's internal locks,
`JournalWriterQueue::state: Mutex<InnerState>`) serializes the
concurrent calls.

**Concurrency hazard**: see `hazard-analysis.md` H-CONC-1. The
contract does NOT commit to a Loom lane; the planner may upgrade.

---

## Boundary 7 — FFI / Unsafe Boundaries (NONE)

The 26 tests use zero `unsafe` code, zero FFI calls, and zero C
bindings. The `vb_storage` crate has `#![forbid(unsafe_code)]` at
`lib.rs:1`; the wire is consistent with this prohibition.

---

## Boundary 8 — Network Boundary (NONE)

The 26 tests make zero network calls. Fjall is a local LSM-tree
embedded engine.

---

## Cross-Crate Boundary (NONE)

The wire does not introduce any new cross-crate dep edge:

- `vb_core` is already a dependency of `vb_storage` (used by the 16
  wired modules for `RunId`, `StepIdx`, `WorkflowDigest`, `WorkflowId`).
- No new feature flag is added to `vb_storage/Cargo.toml`.
- No new `pub use ...` re-export is added.
- No `pub mod ...` is changed.

The contract preserves the cross-crate boundary exactly.

---

## Summary Table

| Boundary | Modified? | Risk | Mitigation |
|----------|-----------|------|------------|
| Compile-graph entry (lib.rs:182) | **YES** (3 lines added) | None (matches 16 sibling pattern) | INV-WIRE-1..10 |
| Pure core | NO | None | n/a |
| Imperative shell | NO | None | n/a |
| Storage (Fjall) | NO (exercised) | Low (slow CI disks) | Per-test tempdir isolation |
| Time | NO | None | n/a |
| Concurrency | NO (exercised) | Medium (4 tests) | Default-Rust threading; Loom optional |
| FFI / Unsafe | NO | None | `#![forbid(unsafe_code)]` already enforced |
| Network | NO | None | n/a |
| Cross-crate | NO | None | No `Cargo.toml` change |

---

END OF BOUNDARY MAP.