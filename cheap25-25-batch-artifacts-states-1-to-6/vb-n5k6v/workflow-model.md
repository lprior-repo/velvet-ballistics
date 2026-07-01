# Workflow Model — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Overview

The bead's workflow is a **build-graph / test-orchestration workflow**,
not a runtime workflow. There is no runtime state machine, no async
pipeline, no recovery path. The workflow is the linear sequence of
operations that:

1. Detects the orphan.
2. Verifies the file is not stale.
3. Verifies the file is owned and tracked.
4. Inserts the 3-line `mod` declaration.
5. Compiles the test binary.
6. Runs the 26 surfaced tests.
7. Asserts all 26 pass.

The contract pins each of these as an explicit state with guards,
transitions, and terminal outcomes.

---

## Workflow State Machine — Bead Lifecycle

```
                    +-------------------+
                    |  Initialized      |  (bead created, STATE.md: status=initialized)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Scoped           |  (codebase-map.md + delivery-scope.jsonl complete)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Contract-Accepted|  (this artifact + 8 others)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Proof-Planned    |  (proof-planner emits planned.jsonl)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Implemented      |  (holzman-rust inserts the 3 lines)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Verified         |  (cargo test -p vb_storage --lib edge_case
                    +-------------------+    returns 26 passed, 0 failed)
                              |
                              v
                    +-------------------+
                    |  Evidenced        |  (evidence-packaging captures pre/post counts)
                    +-------------------+
                              |
                              v
                    +-------------------+
                    |  Landed           |  (jj describe + jj git push + bd close)
                    +-------------------+
                              |
                              v
                          (terminal)
```

### States

| State | Description | Terminal? |
|-------|-------------|-----------|
| `Initialized` | Bead created in Dolt; STATE.md committed; `controller: femdation`. | No |
| `Scoped` | `codebase-map.md` (State 2 explore artifact) and `delivery-scope.jsonl` committed; orphan status confirmed, all 32 symbols resolved. | No |
| `Contract-Accepted` | All 9 contract artifacts written and reviewed (this is the current target state — vb-n5k6v is at State 3). | No |
| `Proof-Planned` | `proof-planner` agent emits `proof-obligations.planned.jsonl`. | No |
| `Implemented` | `holzman-rust` agent inserts the 3-line `mod` declaration into `crates/vb_storage/src/lib.rs:182`. | No |
| `Verified` | `formal-verifier` agent runs `cargo test -p vb_storage --lib edge_case` and confirms 26 passed, 0 failed. | No |
| `Evidenced` | `evidence-packaging` agent captures pre/post test counts and the 26-test filtered run. | No |
| `Landed` | `landing-skill` agent runs `jj describe`, `jj git push`, `bd close vb-n5k6v`. | **Yes** |

### Guards

| Guard | Predicate | On failure |
|-------|-----------|------------|
| `G-ScopeComplete` | `codebase-map.md` and `delivery-scope.jsonl` exist and are non-empty | Cannot proceed to Contract-Accepted; re-run explore. |
| `G-ContentFresh` | All 32 symbols used by the 26 tests resolve to live production source | Cannot proceed to Contract-Accepted; re-run stale-content audit or reject the bead as truly dead. |
| `G-NoCollisions` | All 26 test names are unique across `vb_storage/src/` | Cannot proceed; rename or reject. |
| `G-DevDepsPresent` | `tempfile`, `proptest`, `blake3` are available in `vb_storage/Cargo.toml` | Cannot proceed; add dev-deps. |
| `G-LedgerEntry` | `edge_case_tests.rs` is on `.config/source-length-exceptions.txt:150` (or equivalent ledger) | Cannot proceed via WIRE; switch to DELETE branch. |
| `G-WireShape` | The 3-line insertion matches the 16 sibling declarations exactly | Cannot proceed; correct shape. |
| `G-TestCompile` | `cargo test -p vb_storage --lib --no-run` compiles without errors | Cannot proceed to Verified; debug. |
| `G-TestPass` | `cargo test -p vb_storage --lib edge_case` returns 26 passed, 0 failed | Cannot proceed to Evidenced; debug failures. |

### Terminal Outcome

**Landed**. There is no alternate outcome — the bead has a single
resolution branch (WIRE; DELETE was rejected in `domain-model.md`'s
forbidden states).

---

## Workflow State Machine — Per-Test Lifecycle (cargo test runtime)

Each of the 26 tests follows a strict lifecycle when `cargo test` runs:

```
       +--------------------+
       |  TestDiscovered    |  (cargo test parses the file; registers #[test] fns)
       +--------------------+
                  |
                  v
       +--------------------+
       |  TestCompiled      |  (rustc compiles edge_case_tests::test_fn as
       +--------------------+    part of the lib-test binary)
                  |
                  v
       +--------------------+
       |  TestSelected      |  (cargo test filter "edge_case" selects this fn)
       +--------------------+
                  |
                  v
       +--------------------+
       |  SetupPhase        |  (call to temp_journal() at file line 25
       +--------------------+    OR manual tempdir construction)
                  |
                  v
       +--------------------+
       |  InvokePhase       |  (call public/pub(crate) API methods)
       +--------------------+
                  |
                  v
       +--------------------+
       |  AssertPhase       |  (assert! / assert_eq! / matches! on result)
       +--------------------+
                  |
                  v
       +--------------------+
       |  TeardownPhase     |  (TempDir drops, journal closes if not already)
       +--------------------+
                  |
        +---------+---------+
        |                   |
        v                   v
   +---------+       +-------------+
   |  PASS   |       |  FAIL       |
   +---------+       +-------------+
        |                   |
        v                   v
   (terminal OK)      (terminal FAIL — debug, fix, re-run)
```

### Per-Test States

| State | Description | Terminal? |
|-------|-------------|-----------|
| `TestDiscovered` | cargo test parses the wired `mod edge_case_tests;` declaration and registers all 26 `#[test] fn`s. | No |
| `TestCompiled` | rustc compiles each `#[test] fn` body into the `vb_storage` lib-test binary. | No |
| `TestSelected` | The cargo test filter (`edge_case` substring) matches the test fn path. | No |
| `SetupPhase` | Test-local fixture created: `temp_journal()` returns `(TempDir, FjallJournal)` (18 tests) or manual `tempfile::tempdir()` + `FjallJournal::open(path)` (8 tests). | No |
| `InvokePhase` | Test invokes the API: `append_journaled`, `persist_strict`, `enqueue_journaled`, `batch()`, `commit()`, `encode_record`, etc. The 4 concurrent tests additionally `thread::spawn` and `.join()`. | No |
| `AssertPhase` | Test asserts a post-condition: `assert!(matches!(result, Err(...)))`, `assert_eq!(events.len(), N)`, `assert!(result.is_ok())`, etc. | No |
| `TeardownPhase` | `TempDir` RAII drop removes the Fjall keyspace directory. `FjallJournal::close()` is called explicitly in the open/close-cycle tests (lines 372, 391, 414, 428). | No |
| `PASS` | All assertions hold; cargo test records the test as `ok`. | **Yes** |
| `FAIL` | Any assertion fails; cargo test records the test as `FAILED` with a diff. | **Yes** (failure) |

### Per-Test Guards

| Guard | Predicate | On failure |
|-------|-----------|------------|
| `G-SetupOk` | `temp_journal()` returns `Ok` (or manual `open` returns `Ok`) | Test panics on the `expect`; counts as a FAIL. |
| `G-InvokeOk` | API calls return `Ok` where expected, or the right `Err` variant | Test asserts; counts as a FAIL if mismatched. |
| `G-ReplayLen` | `journal.events_for_run(run).len() == expected` for replay tests | Test FAIL. |
| `G-MatchVariant` | `matches!(result, Err(JournalError::X))` for error-variant tests | Test FAIL. |
| `G-ThreadJoin` | All `thread::spawn` handles `.join()` successfully | Test FAIL (panics propagate through `expect`). |

### Concurrency Sub-Workflow (4 of 26 tests)

The 4 concurrent tests
(`multiple_threads_append_to_different_runs`,
`concurrent_enqueue_to_writer_queue`,
`concurrent_batch_writes_from_multiple_threads`,
`concurrent_read_while_another_writes`) have an extended workflow:

```
       SetupPhase (Arc::new(journal) and/or Arc::new(queue))
                  |
                  v
       +--------------------+
       |  SpawnPhase        |  (2, 4, or 8 thread::spawn calls, each
       +--------------------+    captures an Arc clone)
                  |
                  v
       +--------------------+
       |  ConcurrentInvoke  |  (each thread calls append_*, enqueue_*,
       +--------------------+    or batch() concurrently — no explicit
                  |             synchronization; Fjall/queue internals
                  v             serialize via Mutex or internal locking)
       +--------------------+
       |  JoinPhase         |  (handle.join().expect("...") for each handle)
       +--------------------+
                  |
                  v
       AssertPhase (verify events_for_run returns expected counts)
```

**Concurrency hazard**: The contract does NOT commit to a Loom lane
(see `domain-model.md` §"Open Domain Questions"). The default-Rust
threading is sufficient because `FjallJournal::append_*` is `&self`
(interior mutability via Fjall's internal locks) and `JournalWriterQueue`
wraps `Mutex<InnerState>` at `queue/writer.rs:33`.

---

## Workflow State Machine — Compile-Graph Transition

The **single** state machine that this bead modifies is the cargo
compile graph for the `vb_storage` lib-test binary:

```
       Pre-Wire State:
       ================
       cargo test -p vb_storage --lib discovers:
         - proptest_integration (mod at lib.rs:120)
         - error_tests (lib.rs:124)
         - error_code_tests (lib.rs:128)
         - type_tests (lib.rs:132)
         - index_tests (lib.rs:136)
         - index_maintenance_tests (lib.rs:141)
         - artifact_tests (lib.rs:145)
         - blob_tests (lib.rs:149)
         - header_tests (lib.rs:153)
         - hydrate_tests (lib.rs:157)
         - process_lock_tests (lib.rs:161)
         - record_tests (lib.rs:165)
         - recover_tests (lib.rs:169)
         - recovery_type_tests (lib.rs:173)
         - replay_core_tests (lib.rs:177)
         - snapshot_tests (lib.rs:181)
         - tests (mod tests, not via #[path])
         - vb_2bok_durability_gate_tests (mod, not via #[path])
       Total: 16 #[path = "..."] modules + 2 other mod declarations = 18 modules

       Post-Wire State:
       =================
       cargo test -p vb_storage --lib discovers:
         - (all 18 above)
         - edge_case_tests (mod at lib.rs:182, NEW)
       Total: 17 #[path = "..."] modules + 2 other mod declarations = 19 modules
```

### Transition Guard

| Guard | Predicate |
|-------|-----------|
| `G-InsertionShape` | The new 3-line declaration at `lib.rs:182` matches the existing 16 declarations byte-for-byte (modulo the path and module name) |
| `G-NoForeignChange` | No other line in `lib.rs` is altered by this bead (git diff restricted to 3 added lines, 0 removed, 0 modified) |

---

## Idempotence

The wire operation is **idempotent** in the strongest sense: applying
it twice produces no further change to the compile graph. The first
wire moves the file from `Dormant` to `Active`; a second wire is a
no-op (Rust forbids duplicate `mod` declarations for the same name).

The wire is also **reversible**: removing the 3-line declaration moves
the file back to `Dormant`. No production code is lost; no persistent
state is changed.

---

## Cancellation Paths

There are **no cancellation paths** in the test-only workflow because
there is no runtime workflow to cancel. The build-graph workflow can be
aborted at any state by reverting the JJ change (`jj abandon`), which
restores the pre-bead state without side effects.

---

## Retry Semantics

The wire operation has no retry semantics: it either succeeds (3-line
insertion accepted by `cargo check`) or fails (cargo check reports an
error). There is no transient failure mode that would benefit from
retry.

The 26 surfaced tests have cargo-test retry semantics built-in (each
test runs once per `cargo test` invocation; cargo test does not retry
on FAIL by default).

---

## Hazard Notes

See `hazard-analysis.md` for the full enumeration. The most relevant
workflow-level hazard is:

- **H-WF-1 (Concurrent Test Schedule Exploration)**: The 4 concurrent
  tests use `std::thread::spawn` without explicit synchronization. The
  contract commits to default-Rust threading; the planner may upgrade
  to Loom if it finds interleaving risk. No temporal deadline applies
  because tests have no real-time requirement.

---

END OF WORKFLOW MODEL.