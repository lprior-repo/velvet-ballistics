# Domain Model — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Bead Scope (verbatim)

> An orphaned `edge_case_tests` file is referenced by no `Cargo.toml` and
> has stale content. Either wire it into a target crate's `[[test]]`
> entries, or delete it if it's truly dead.

This bead is a **test-only build-system repair**. No production code
changes. No cross-crate API changes. The contract pins the wiring pattern
that the 16 sibling `#[path = "..."]` modules already follow and commits
to the **WIRE** branch (rejected **DELETE**) with three concrete
justifications (see §7 of `codebase-map.md`).

The domain modeled here is therefore **not** the Fjall journal API surface
(which is fully covered by the existing 16 wired modules) — it is the
**build-graph / test-orchestration domain**: a Rust test module's
lifecycle from `add` → `orphan` → `wire` → `compile` → `run` → `PASS`.

---

## Ubiquitous Language

| Term | Definition |
|------|------------|
| **Test Module** | A Rust file under `crates/<crate>/src/` declared as a `mod` from the crate's `lib.rs` via `#[cfg(test)] #[path = "<name>_tests.rs"] mod <name>;`. The `#[cfg(test)]` makes the module invisible to production builds; `#[path = "..."]` overrides Rust's default module-name-to-filename inference. |
| **Sibling Pattern** | The 16 existing `#[cfg(test)] #[path = "<name>_tests.rs"] mod <name>;` declarations at `crates/vb_storage/src/lib.rs:118-181`: `proptest_integration`, `error_tests`, `error_code_tests`, `type_tests`, `index_tests`, `index_maintenance_tests`, `artifact_tests`, `blob_tests`, `header_tests`, `hydrate_tests`, `process_lock_tests`, `record_tests`, `recover_tests`, `recovery_type_tests`, `replay_core_tests`, `snapshot_tests`. |
| **Orphan Test File** | A `.rs` source file that lives under `crates/<crate>/src/` AND is not declared by any `mod` in the crate's `lib.rs` AND is not referenced by any `Cargo.toml` `[[test]]` entry. Cargo's test-discovery rules treat such a file as dead code: it is never compiled, never run, never linted. |
| **Dormant Test** | A specific category of orphan test file: one that (a) contains valid test functions targeting current production APIs, (b) has been added to the source tree intentionally (i.e., has a tracked owner in `.config/source-length-exceptions.txt` or similar ledger), but (c) has been left without the corresponding `mod` declaration. The wave-3 audit (`to-fix/wave3/agent-09-verus.md:19,45`) calls this state *dormant*. |
| **Dormant Wake-up** | The transition from **Dormant** to **Active** accomplished by adding a single `#[cfg(test)] #[path = "..."] mod ...;` declaration that points at the dormant file. The file's existing `mod <name> { ... }` inner wrapper (lines 12, 11 of `edge_case_tests.rs`) becomes the module body. |
| **Active Test Module** | The post-wire state: the file's `#[test]` functions are registered with `cargo test`, run by the `moon ci` test lane, and counted by `cargo test -p vb_storage --lib`. |
| **Topic Bucket** | A semantic grouping inside `edge_case_tests.rs` delimited by `// ====` comment banners. The file has **7** buckets: `Disk full simulation`, `Concurrent access patterns`, `Very large values`, `Rapid open/close cycles`, `Record kind boundary tests`, `Batch edge cases`, `Queue edge cases`. |
| **Wire-Recommendation** | The chosen resolution for `vb-n5k6v`: insert a 3-line `mod` declaration into `crates/vb_storage/src/lib.rs:182` so the dormant file becomes an active test module. |
| **Delete-Recommendation** | The rejected alternative: remove `crates/vb_storage/src/edge_case_tests.rs` outright. Rejected because the file is not stale, is on the source-length ledger with an active owner and removal plan, and the 26 test names are unique across the workspace. |
| **Test Lane** | The `default-rust` verifier lane (cargo test) — the only verifier lane affected by this bead. Kani/Verus/Flux/Loom/proptest/fuzz/TLA+ lanes are explicitly out of scope (see `contract.md` §"Verifier Lane Profile"). |

---

## Value Objects

### Test-Function Inventory (the 26 behaviors being un-orphaned)

Each test function is a value object: it has a unique name (no collisions
in the workspace — verified `codebase-map.md` §6), a topic bucket, a
risk profile, and a behavior-affecting flag. **No name, signature, or
assertion is changed by this bead; only the compile-graph entry is added.**

| # | Test fn | Topic bucket | Concurrent? | Behavior-affecting? |
|---|---------|--------------|-------------|---------------------|
| 1 | `persist_strict_handles_simulated_failure` | Disk full | No | YES (closes P1 wave-3 dormant test) |
| 2 | `persist_strict_recovers_after_simulated_failure` | Disk full | No | YES |
| 3 | `multiple_threads_append_to_different_runs` | Concurrent | YES (8 threads) | YES |
| 4 | `concurrent_enqueue_to_writer_queue` | Concurrent | YES (4 threads) | YES |
| 5 | `concurrent_batch_writes_from_multiple_threads` | Concurrent | YES (4 threads) | YES |
| 6 | `concurrent_read_while_another_writes` | Concurrent | YES (2 threads) | YES |
| 7 | `very_large_blob_payload` | Very large | No | YES (1 MiB blob) |
| 8 | `very_large_compiled_ir_payload` | Very large | No | YES (512 KiB IR) |
| 9 | `very_large_workflow_source_payload` | Very large | No | YES (128 KiB source) |
| 10 | `very_large_snapshot_with_many_slots` | Very large | No | YES (10K slots) |
| 11 | `very_large_run_header_values` | Very large | No | YES (u64::MAX / u32::MAX) |
| 12 | `many_events_per_run` | Very large | No | YES (200 events) |
| 13 | `rapid_open_close_cycles_preserve_data` | Open/close | No | YES (10 cycles) |
| 14 | `rapid_open_close_without_writes` | Open/close | No | YES (20 cycles) |
| 15 | `open_append_close_reopen_verify` | Open/close | No | YES (2-cycle scenario) |
| 16 | `encode_rejects_unknown_magic` | Record boundary | No | YES (codec invariant) |
| 17 | `encode_accepts_run_header_with_index_magic` | Record boundary | No | YES |
| 18 | `encode_accepts_index_update_with_index_magic` | Record boundary | No | YES |
| 19 | `decode_rejects_zero_max_payload_with_nonzero_payload` | Record boundary | No | YES |
| 20 | `encode_rejects_zero_length_payload_serialization` | Record boundary | No | YES (empty payload OK) |
| 21 | `batch_commit_then_second_batch_with_same_run_seq_rejected` | Batch | No | YES (DuplicateEvent) |
| 22 | `batch_len_zero_after_digest_mismatch_abort` | Batch | No | YES |
| 23 | `empty_batch_strict_commits_successfully` | Batch | No | YES |
| 24 | `queue_capacity_one_single_enqueue_dequeue` | Queue | No | YES |
| 25 | `queue_drain_all_with_large_batch_relative_to_capacity` | Queue | No | YES |
| 26 | `queue_rejects_all_writes_after_shutdown` | Queue | No | YES (QueueShutdown) |

Total: **26 tests** — 4 concurrent, 22 sequential. All 26 are
behavior-affecting (each one asserts a non-trivial post-condition).

### Topic-Bucket Enumeration (7 buckets, atomic in this contract)

```rust
// edge_case_tests.rs:31-33   — Disk full simulation (2 tests)
// edge_case_tests.rs:79-81   — Concurrent access patterns (4 tests)
// edge_case_tests.rs:244-246 — Very large values (6 tests)
// edge_case_tests.rs:353-355 — Rapid open/close cycles (3 tests)
// edge_case_tests.rs:438-440 — Record kind boundary tests (5 tests)
// edge_case_tests.rs:532-534 — Batch edge cases (3 tests)
// edge_case_tests.rs:583-585 — Queue edge cases (3 tests)
```

Each bucket corresponds to one **module-internal invariant family**
disjoint from the other 16 wired modules. The split was chosen by the
2026-05-23 round-2-7 sweep that introduced the file (commit `a95354665`,
subject `test: rounds 2-7 - exhaustive behavior tests across 7 crates`).

### File-Level `#![allow(...)]` Block (preserved verbatim)

```rust
// edge_case_tests.rs:1-9
#![allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
```

This block is **identical in shape** to the file-level allows in all 16
sibling `_tests.rs` files (verified against `tests.rs:1-13`,
`error_tests.rs:1-9`, etc.). It is a project-wide test-file convention
acknowledging that behavior tests legitimately use `unwrap`/`expect`/
indexing on freshly-allocated in-memory data structures. No change is
proposed; the allow block is part of the wire target.

### Source-Length Exception Entry (preserved verbatim)

```
// .config/source-length-exceptions.txt:150
crates/vb_storage/src/edge_case_tests.rs|lewis|vb-jpq7.47|split-or-retire-before-release|Pre-existing over-300-line Rust source baseline (637 lines); must be split by domain responsibility or retired before removing exception.
```

The wire **does not change the line count**. The 637-line file remains
637 lines. Splitting is tracked by a separate bead (`vb-jpq7.47`) and
explicitly out of scope for `vb-n5k6v`.

---

## Entities

### The 3-Line Wire Declaration (the only thing this bead produces)

```rust
// crates/vb_storage/src/lib.rs — INSERTED after line 181 (after the
// `mod snapshot_tests;` declaration), BEFORE line 183 (`pub mod queue;`).
#[cfg(test)]
#[path = "edge_case_tests.rs"]
mod edge_case_tests;
```

- **Visibility**: `cfg(test)` only — never compiled into production
  builds, never linked into `vb_storage`'s public API, never visible to
  downstream crates (`vb_runtime`, `vb_cli`, `vb_validate`).
- **Path attribute**: `#[path = "edge_case_tests.rs"]` overrides Rust's
  default inference (which would expect `edge_case_tests/...rs`). The
  attribute matches the literal filename exactly.
- **Module name**: `edge_case_tests` (matches the file's internal
  `mod edge_case_tests { ... }` wrapper at line 12).
- **Blast radius**: zero production logic touched; the only observable
  effect is `cargo test -p vb_storage --lib` test count increasing from
  ~924 to ~950 (delta of 26).

### Anchor Marker (the existing 16 declarations this mirrors)

```
// crates/vb_storage/src/lib.rs:179-181 (the immediate predecessor)
#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod snapshot_tests;
```

```
// crates/vb_storage/src/lib.rs:183 (the immediate successor)
pub mod queue;
```

The new declaration slots between these two lines, restoring the
canonical pattern.

---

## Commands, Events, Policies

There are **no runtime commands, events, or policies** introduced by this
bead. The contract is exclusively about the compile-graph; nothing runs
at production runtime as a result.

The test functions themselves are **passive observers**: each test
constructs a `temp_journal()`, invokes public/`pub(crate)` API methods,
and asserts a post-condition. None of them register listeners, mutate
shared global state, or trigger cross-test side effects.

---

## Invariants

| ID | Invariant | Holder |
|----|-----------|--------|
| INV-WIRE-1 | After wire, `cargo test -p vb_storage --lib` discovers all 26 `edge_case_tests::*` test functions. | cargo test infrastructure |
| INV-WIRE-2 | The wire declaration is the **only** edit to `crates/vb_storage/src/lib.rs`. | git diff |
| INV-WIRE-3 | The wire declaration compiles cleanly under `#![forbid(unsafe_code)]` (test mode). | rustc + clippy |
| INV-WIRE-4 | The wire declaration does not alter `vb_storage`'s public API surface (`pub use ...`, `pub mod ...`). | `cargo public-api` / `cargo doc` |
| INV-WIRE-5 | The wire declaration does not alter any sibling `#[path = "..."]` declaration. | git diff |
| INV-WIRE-6 | The wire declaration does not change the line count of `edge_case_tests.rs` (still 637 lines). | `rtk wc -l` |
| INV-WIRE-7 | The wire declaration preserves the source-length exception at `.config/source-length-exceptions.txt:150`. | ledger audit |
| INV-WIRE-8 | All 26 test names are unique across the workspace (zero collisions). | `rtk rg` |
| INV-WIRE-9 | The wire declaration does not require any `Cargo.toml` change. | git diff |
| INV-WIRE-10 | The wire declaration does not introduce any new external dependency. | `Cargo.lock` diff |

---

## Forbidden States (what this contract explicitly rejects)

| Forbidden | Why rejected |
|-----------|--------------|
| Wiring the file as a standalone integration test (i.e., moving it to `crates/vb_storage/tests/edge_case_tests.rs` and adding a `[[test]]` entry). | The file uses `use crate::{...}` (intra-crate imports) and accesses `pub(crate)` methods (`fail_next_persist_for_test`). It is structurally an in-source `#[cfg(test)]` module, not a top-level integration test. |
| Wiring the file with a different module name (e.g., `mod edge_cases` instead of `mod edge_case_tests`). | The internal `mod edge_case_tests { ... }` wrapper at line 12 binds the module name; a different outer name would be a mismatch. |
| Splitting the 7 topic buckets across multiple modules (e.g., `mod edge_case_disk_full_tests;`). | Would create 7 sibling declarations instead of 1 and obscure the wave-3 wiring pattern. Out of scope per bead description (single-file fix). |
| Deleting the file outright. | §6 of `codebase-map.md` shows the content is not stale; §7 shows it is on the source-length ledger with active owner `lewis` and removal plan `vb-jpq7.47`. Silent deletion would break the ledger's promise and lose 26 unique tests. |
| Adding a `Cargo.toml` change. | All required dev-deps (`tempfile`, `proptest`) are already present at `vb_storage/Cargo.toml:19-21`. |
| Adding any verifier lane beyond `default-rust`. | The change is a build-graph entry; no production logic is touched. Kani/Verus/Flux/Loom/fuzz/proptest coverage of these 26 tests would duplicate work already done by the sibling modules and the existing Kani harnesses (`kani_record_magic`, `kani_record_kind`, etc.). |

---

## Open Domain Questions Deferred to the Proof Planner

1. **Should the 4 concurrent tests receive a Loom permutation lane?**
   The contract does not commit. The 4 concurrent tests use
   `Arc<FjallJournal>` and `Arc<JournalWriterQueue>` with
   `std::thread::spawn`. `FjallJournal::append_*` takes `&self` and uses
   Fjall's internal locking; `JournalWriterQueue` wraps
   `Mutex<InnerState>`. The precedent in `journal/tests.rs:2598+` and
   `recovery/tests.rs` is default-Rust multi-threading without Loom.
   The contract's recommendation is **no Loom lane**; the planner may
   override if it finds new interleaving risk.

2. **Should the 5 record-boundary tests be Kani-ified?** The contract
   does not commit. Existing Kani harnesses
   (`kani_record_magic.rs`, `kani_record_kind.rs`,
   `kani_record_payload_len.rs`, `kani_record_crc.rs`,
   `kani_record_schema.rs`, `kani_postcard_envelope_wire.rs`) already
   cover the codec invariants. Default-Rust is sufficient for the
   wire-only fix.

3. **Should the 26 tests receive proptest coverage?** The contract
   does not commit. The tests are deterministic concrete-value behavior
   tests; proptest would add coverage of edge cases beyond the chosen
   concrete values (e.g., arbitrary payload sizes, arbitrary run IDs).
   The planner may add proptest lanes if it finds value.

---

END OF DOMAIN MODEL.