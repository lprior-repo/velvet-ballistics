# Hazard Analysis — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Overview

This hazard analysis covers the **build-graph** surface of the wire
operation (1 declaration, 3 lines, 0 production logic) and the
**test-runtime** surface of the 26 surfaced tests. The hazard model
focuses on:

- Whether the wire introduces new compile-time hazards (it does not).
- Whether the surfaced tests introduce new runtime hazards (mostly no;
  yes for the 4 concurrent tests).
- Whether the wire breaks any pre-existing invariants (it does not).

The contract is **bounded**: there is exactly one production-code edit
(3 lines added), and the only behavior change is that 26 dormant tests
become active in `cargo test -p vb_storage --lib`.

---

## Hazard Class Summary

| Class | Triggered? | Severity | Lane |
|-------|------------|----------|------|
| Build-graph / module-resolution | YES (root cause) | **P1** (root cause of bead) | `default-rust` |
| Concurrency (multi-thread test race) | YES (4 of 26 tests) | **P2** | `default-rust` (Loom optional) |
| Persistence (Fjall keyspace state) | YES (22 of 26 tests open tempdir) | **P2** | `default-rust` |
| Parser / codec (record boundary) | YES (5 of 26 tests) | **P2** | `default-rust` (existing Kani harnesses cover invariants) |
| Arithmetic (max-value constants) | YES (1 test: `very_large_run_header_values`) | **P3** | `default-rust` |
| Temporal / open-close ordering | YES (3 of 26 tests) | **P3** | `default-rust` |
| File-size (637-line file) | YES (carried over from existing exception) | **P3** | n/a (already on ledger) |
| Unsafe / provenance | NO | n/a | n/a (no unsafe code) |
| FFI | NO | n/a | n/a |
| Network | NO | n/a | n/a |
| Time / clock | NO | n/a | n/a |
| Hostile input / fuzz | NO | n/a | n/a (deterministic concrete-value tests) |
| Public API | NO | n/a | n/a (no `pub` change) |
| Cross-crate | NO | n/a | n/a (no `Cargo.toml` change) |
| Release / API | NO | n/a | n/a (test-only) |
| Performance | LOW | n/a | n/a (estimated +5-15s on workstation, +30-60s on slow CI) |

---

## H-WIRE-1 — Module-Resolution Hazard (root cause)

**Severity**: P1 (root cause of the bead).
**Trigger**: A `.rs` file under `crates/<crate>/src/` that is not
declared by any `mod` in the crate's `lib.rs` is invisible to cargo's
test discovery. The file is **never compiled**, **never run**, and
**never linted**. The 26 tests it contains count as zero toward
`cargo test --lib`'s tally.

**Why this matters for `edge_case_tests.rs`**: the file was added on
2026-05-23 (commit `a95354665`, subject `test: rounds 2-7 - exhaustive
behavior tests across 7 crates`) but the `mod` declaration was never
written. The file has been dormant for the entire wave-3 → wave-10
window. Until wired, the 26 tests provide zero coverage.

**Mitigation (contract)**:

1. Add the 3-line `#[cfg(test)] #[path = "edge_case_tests.rs"] mod
   edge_case_tests;` declaration at `lib.rs:182`.
2. Co-locate with the 16 sibling `#[path = "..."]` declarations
   (`lib.rs:118-181`).
3. Match the canonical pattern byte-for-byte (3 lines, no `pub`, no
   inline body).

**Verifier lane**: `cargo test -p vb_storage --lib edge_case` (default-
rust) MUST return `26 passed; 0 failed` post-wire.

**Failure detection**:

| Failure | Detection |
|---------|-----------|
| Typo in module name | `cargo check -p vb_storage --tests` reports E0432 |
| Typo in `#[path]` | `cargo check` reports file-not-found |
| Forgot `#[cfg(test)]` | `cargo build -p vb_storage` (non-test build) fails on test-only imports |
| Added `pub` | No compile error, but violates project hygiene |

---

## H-CONC-1 — Concurrent Test Schedule-Exploration Hazard

**Severity**: P2.
**Trigger**: The 4 concurrent tests
(`multiple_threads_append_to_different_runs`,
`concurrent_enqueue_to_writer_queue`,
`concurrent_batch_writes_from_multiple_threads`,
`concurrent_read_while_another_writes`) use `std::thread::spawn` with
`Arc<FjallJournal>` and `Arc<JournalWriterQueue>`. Without explicit
synchronization, the test relies on the production code's interior
mutability:

- `FjallJournal::append_*` takes `&self`; Fjall internally locks per
  partition.
- `JournalWriterQueue` wraps `Mutex<InnerState>` at `queue/writer.rs:33`.

**Why this matters**: a future change to the production locking model
(e.g., switching to `RwLock`, dropping a lock, or changing Fjall's
internal concurrency model) could expose a race that the default-Rust
multi-thread test misses. The test would intermittently FAIL on slow
disks or under contention.

**Mitigation (contract)**:

1. The 4 tests follow the existing pattern in `journal/tests.rs:2598+`
   and `recovery/tests.rs` (which also use `std::thread::spawn` +
   `Arc<...>` without Loom).
2. Each test uses a per-test `tempfile::tempdir()` to avoid cross-test
   Fjall state contamination.
3. Each test calls `handle.join().expect(...)` on every spawned thread,
   so a panic in any thread surfaces as a test FAIL rather than a
   silent dead-lock.

**Verifier lane**: default-Rust threading is the precedent and
acceptable. The planner MAY add a Loom permutation lane if it finds
new interleaving risk. The contract does not require Loom.

**Failure detection**:

| Failure | Detection |
|---------|-----------|
| Data race | Loom would catch; default-Rust might miss |
| Deadlock | `handle.join().expect("...")` would catch the panic |
| Lost write | `events_for_run(run).len()` would catch |
| Out-of-order seq | `events.iter().enumerate()` seq check would catch |

---

## H-PERSIST-1 — Fjall Keyspace Persistence Hazard

**Severity**: P2.
**Trigger**: 22 of the 26 tests call `temp_journal()` to spin up a
fresh Fjall keyspace in a `tempfile::tempdir()`. The Fjall keyspace
has a `Flush` step that calls `fsync` on the underlying file. On
slow CI disks (NFS, magnetic, or constrained container disks), this
can race or timeout.

**Why this matters**: tests that rely on `close()` + reopen() to verify
durability (`rapid_open_close_cycles_preserve_data`,
`rapid_open_close_without_writes`,
`open_append_close_reopen_verify`) could intermittently FAIL if the
filesystem does not flush the LSM batch in time.

**Mitigation (contract)**:

1. Each test owns its own `tempfile::TempDir` (RAII drop on test end).
2. The rapid-open/close tests do **not** interleave with concurrent
   I/O (they are sequential).
3. The contract preserves the existing production durability path
   (no change to `FjallJournal::close`).

**Verifier lane**: default-Rust is sufficient. The contract does not
require synthetic-fault injection (no `fault_injection` feature, no
chaos testing).

**Failure detection**:

| Failure | Detection |
|---------|-----------|
| LSM batch not flushed before reopen | `events_for_run(run).len() != expected` |
| Tempdir not cleaned up | `cargo test` reports no warning (TempDir is RAII) |
| Disk full during test | `tempfile::tempdir().expect("...")` panic surfaces as FAIL |

---

## H-CODEC-1 — Record-Kind Boundary Hazard

**Severity**: P2.
**Trigger**: The 5 record-boundary tests (`encode_rejects_unknown_magic`,
`encode_accepts_run_header_with_index_magic`,
`encode_accepts_index_update_with_index_magic`,
`decode_rejects_zero_max_payload_with_nonzero_payload`,
`encode_rejects_zero_length_payload_serialization`) exercise the
`encode_record` / `decode_record` round-trip against specific
magic/kind pairings.

**Why this matters**: the magic/kind family table is **load-bearing**:
a future change that allows an incorrect magic/kind pairing would
corrupt persisted data on disk. The 5 tests pin 5 specific cells of
this table.

**Mitigation (contract)**:

1. The 5 tests follow the same structural pattern as the existing
   `codec/tests.rs` and `record_tests.rs` modules (which also exercise
   the magic/kind table).
2. Existing Kani harnesses (`kani_record_magic.rs`,
   `kani_record_kind.rs`, `kani_record_payload_len.rs`,
   `kani_record_crc.rs`, `kani_record_schema.rs`,
   `kani_postcard_envelope_wire.rs`) cover the **complete** magic/kind
   table at the type level. The wire's 5 tests add concrete-value
   coverage.

**Verifier lane**: default-Rust is sufficient; existing Kani harnesses
provide exhaustive type-level coverage.

**Failure detection**:

| Failure | Detection |
|---------|-----------|
| Accepting wrong magic | `matches!(result, Err(JournalError::RecordKindFamilyMismatch { .. }))` fails |
| Rejecting valid magic | `assert!(result.is_ok())` fails |
| Overflow on payload length | `matches!(result, Err(JournalError::PayloadTooLarge { .. }))` fails |

---

## H-ARITH-1 — Max-Value Arithmetic Hazard

**Severity**: P3.
**Trigger**: `very_large_run_header_values` (file lines 313-328)
constructs a `RunHeaderRecord` with `RunId::new(u64::MAX)`,
`WorkflowId::new(u32::MAX)`, and `accepted_at_ms: u64::MAX`. This is
the only test in the file that exercises the **upper boundary** of
the newtype ranges.

**Why this matters**: a future change to `RunId::new` or
`WorkflowId::new` that introduces a `+ 1` overflow check could
silently reject `u64::MAX` and make the test FAIL.

**Mitigation (contract)**:

1. The test pins the newtype constructors' **upper-bound acceptance**
   behavior — `RunId::new(u64::MAX).get() == u64::MAX`.
2. The test pins `accepted_at_ms: u64::MAX` round-trip through
   `put_run_header` → `run_header(run)`.
3. The contract does not modify any newtype constructor; this is a
   behavior-preservation pin.

**Verifier lane**: default-Rust is sufficient. No Kani harness needed
because the newtype is bounded by `u64::MAX` at the language level.

**Failure detection**: `assert_eq!(loaded.run, run)` or
`assert_eq!(loaded.accepted_at_ms, u64::MAX)` fails.

---

## H-TEMP-1 — Open-Close Ordering Hazard

**Severity**: P3.
**Trigger**: The 3 rapid open/close tests
(`rapid_open_close_cycles_preserve_data`,
`rapid_open_close_without_writes`,
`open_append_close_reopen_verify`) close and reopen the same Fjall
keyspace on the same path, relying on the LSM tree to flush the
in-memory batch to disk before `close()` returns.

**Why this matters**: on a slow disk, the `close()` call could return
before the flush completes (or the Fjall library could change its
flush semantics). The reopen-then-verify pattern would then FAIL.

**Mitigation (contract)**:

1. Each test uses a unique path inside a `tempfile::tempdir()` (no
   cross-test contamination).
2. The pattern is identical to the existing
   `journal/tests.rs:2598+` precedent.
3. The contract does not modify `FjallJournal::close`; this is a
   behavior-preservation pin.

**Verifier lane**: default-Rust is sufficient.

**Failure detection**: `events_for_run(run).len() != expected` after
reopen.

---

## H-SIZE-1 — 637-Line File-Size Hazard

**Severity**: P3 (carried over from existing exception).
**Trigger**: `edge_case_tests.rs` is 637 lines, exceeding the
project's 300-line rule. It is already on
`.config/source-length-exceptions.txt:150` with active owner `lewis`
and removal plan `vb-jpq7.47` (split-or-retire-before-release).

**Why this matters**: a future `moon ci` source-length gate could
escalate this file to a hard failure if the exception is removed
without a corresponding split.

**Mitigation (contract)**:

1. The wire does NOT change the file's line count.
2. The split is tracked by a separate bead (`vb-jpq7.47`) and is
   **explicitly out of scope** for `vb-n5k6v`.
3. The source-length exception is preserved verbatim at
   `.config/source-length-exceptions.txt:150`.

**Verifier lane**: n/a (the wire does not touch the file's content).

---

## H-NETWORK-1 — (NO HAZARD)

There are zero network calls in the 26 tests. Fjall is local.

---

## H-UNSAFE-1 — (NO HAZARD)

There are zero `unsafe` blocks in `edge_case_tests.rs`. The
`#![forbid(unsafe_code)]` at `lib.rs:1` already prohibits unsafe
code at the crate level; the file is consistent.

---

## H-PUB-1 — (NO HAZARD)

The wire declaration does NOT add `pub`. The module remains private
to `vb_storage`. Downstream crates (`vb_runtime`, `vb_cli`,
`vb_validate`) cannot reference it.

---

## H-CARGO-1 — (NO HAZARD)

The wire does NOT modify `Cargo.toml`. All required dev-deps
(`tempfile`, `proptest`) are already present.

---

## Performance Hazard

**Estimated test-run delta**: +5 to +15 seconds on a workstation;
+30 to +60 seconds on slow CI disks. This is the cost of restoring
26 dormant tests.

**Mitigation if CI budget exceeded**: per-test `tempfile::tempdir()`
isolation is already in place; the planner may mark slow tests with
`#[ignore]` if CI budget pressure arises, but this is **out of scope**
for the wire-only fix.

---

## Risk-Profile Summary

| Aspect | Status |
|--------|--------|
| Production API change | **None** |
| Diagnostic code change | **None** |
| Cross-crate change | **None** |
| Existing test breakage | **None** (no test was running pre-wire; all are new) |
| New test surface | 26 default-Rust behavior tests |
| Verifier lanes required | 1 (`default-rust` cargo test) |
| Verifier lanes optional | 0 (Loom is optional; planner decides) |
| Verifier lanes not required | 6 (Kani, Verus, Flux, proptest, fuzz, TLA+) |
| Concurrency risk | **Medium** (4 tests); default-Rust acceptable per precedent |
| Persistence risk | **Low** (per-test tempdir); CI disk sensitivity low |
| Public-API risk | **None** |

---

END OF HAZARD ANALYSIS.