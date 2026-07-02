# Codebase Map — vb-n5k6v

**bead_id:** vb-n5k6v
**title:** Tests: wire orphaned edge_case_tests or delete stale file (P1 bug)
**scout:** explore (State 2)
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v
**captured_at:** 2026-07-01T15:22:00Z

---

## 1. Bead scope (literal)

> An orphaned edge_case_tests file is referenced by no Cargo.toml and has stale content. Either wire it into a target crate's [[test]] entries, or delete it if it's truly dead.

The bead description gives two allowed resolutions: (a) wire it into a target crate's `[[test]]` entries, or (b) delete the file if it is truly dead.

---

## 2. The orphan file (the single concrete target)

| Field | Value |
| --- | --- |
| Path | `crates/vb_storage/src/edge_case_tests.rs` |
| Size | 637 lines (`rtk wc -l crates/vb_storage/src/edge_case_tests.rs`) |
| Module declaration in file | `mod edge_case_tests { ... }` at line 12, wrapped in `#[cfg(test)]` at line 11 |
| Imports | `use crate::{BlobRecord, CompiledIrRecord, EventSeq, FjallJournal, JournalError, JournalEvent, JournalWriterQueue, RecordKind, RunHeaderRecord, RunSnapshot, StorageLimits, WorkflowSourceRecord, MAGIC_BLOB, MAGIC_JOURNAL_EVENT, MAGIC_INDEX_RECORD, MAX_JOURNAL_EVENT_PAYLOAD_BYTES, MAX_RUN_HEADER_BYTES, DIGEST_BYTES, decode_record, encode_record};` plus `use vb_core::{RunId, StepIdx, WorkflowDigest, WorkflowId};` |
| File-level allows | `#![allow(clippy::as_conversions, clippy::cast_possible_truncation, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::panic_in_result_fn, clippy::unwrap_used)]` at lines 1–9 |
| Test count | 26 `#[test]` functions (verified by `rtk rg -n '^    fn \|^fn '` → 27 matches incl. `temp_journal` helper = 26 tests) |
| Topic buckets (per file comments) | Disk full simulation (2), concurrent access (4), very large values (6), rapid open/close (3), record kind boundary (5), batch edge cases (3), queue edge cases (3) |
| Concurrency model | 4 tests use `std::thread::spawn` with `Arc<FjallJournal>` / `Arc<JournalWriterQueue>` (lines 84, 123, 163, 199) |
| Dev-deps required | `tempfile = { workspace = true }` (already in `[dev-dependencies]` of `vb_storage/Cargo.toml:21`) and `proptest = { workspace = true }` (line 20 — not actually used by this file) |

Test inventory (verified by `rtk rg -n '^    fn '`):

```
36  persist_strict_handles_simulated_failure
58  persist_strict_recovers_after_simulated_failure
84  multiple_threads_append_to_different_runs          [concurrent]
123 concurrent_enqueue_to_writer_queue                  [concurrent]
163 concurrent_batch_writes_from_multiple_threads       [concurrent]
199 concurrent_read_while_another_writes                [concurrent]
249 very_large_blob_payload
263 very_large_compiled_ir_payload
277 very_large_workflow_source_payload
291 very_large_snapshot_with_many_slots
313 very_large_run_header_values
331 many_events_per_run
358 rapid_open_close_cycles_preserve_data
385 rapid_open_close_without_writes
400 open_append_close_reopen_verify
443 encode_rejects_unknown_magic
462 encode_accepts_run_header_with_index_magic
481 encode_accepts_index_update_with_index_magic
500 decode_rejects_zero_max_payload_with_nonzero_payload
523 encode_rejects_zero_length_payload_serialization
537 batch_commit_then_second_batch_with_same_run_seq_rejected
560 batch_len_zero_after_digest_mismatch_abort
575 empty_batch_strict_commits_successfully
588 queue_capacity_one_single_enqueue_dequeue
601 queue_drain_all_with_large_batch_relative_to_capacity
616 queue_rejects_all_writes_after_shutdown
```

---

## 3. Why the file is orphaned (root cause)

`edge_case_tests.rs` lives under `crates/vb_storage/src/` (the **library crate**), not under `crates/vb_storage/tests/` (the **integration test directory**). Cargo only auto-discovers integration tests under `tests/`. For an in-source test module to be compiled, `crates/vb_storage/src/lib.rs` must declare it.

Verified by `rtk rg -n '^mod\s|^#\[cfg\(test\)\]\s*$|^pub\s+mod\s' crates/vb_storage/src/lib.rs` and `rtk rg -n 'path\s*=\s*"\w+_tests\.rs"' crates/vb_storage/src/lib.rs`:

- `crates/vb_storage/src/lib.rs` declares **16** `#[cfg(test)] #[path = "<name>_tests.rs"] mod <name>;` entries (lines 123–180): `error_tests`, `error_code_tests`, `type_tests`, `index_tests`, `index_maintenance_tests`, `artifact_tests`, `blob_tests`, `header_tests`, `hydrate_tests`, `process_lock_tests`, `record_tests`, `recover_tests`, `recovery_type_tests`, `replay_core_tests`, `snapshot_tests`.
- `edge_case_tests.rs` is **NOT** in that list — the closest sibling entries (`snapshot_tests.rs` at lib.rs:180 and `tests.rs` at lib.rs:184) bracket the expected position. No other `mod` / `#[path = ...]` reference exists anywhere in the workspace (`rtk rg 'edge_case_tests'` → 5 hits, all unrelated: source-length exception ledger, two `to-fix/` audit reports, the file itself's `mod edge_case_tests { ... }`, and `fuzz/research/pub-fn-raw.txt` line 393 which is a separate `f64_edge_case_strategy` symbol).

The historical context (verified by `rtk git log --diff-filter=A -- crates/vb_storage/src/edge_case_tests.rs` → commit `a95354665` "test: rounds 2-7 - exhaustive behavior tests across 7 crates", 2026-05-23): the file was added in the Round 3 sweep that bulk-inserted 26 edge-case tests for `vb_storage` but the `mod` declaration was never written. The bead description's "stale content" hypothesis is a false alarm here — see §6.

---

## 4. Sibling files that ARE wired (the canonical pattern to follow)

Verified `#[path = "..."]` declarations in `crates/vb_storage/src/lib.rs`:

| Line | Declaration |
| --- | --- |
| 121 | `#[cfg(test)] #[path = "proptests.rs"] mod proptest_integration;` |
| 123 | `#[cfg(test)] #[path = "error_tests.rs"] mod error_tests;` |
| 127 | `#[cfg(test)] #[path = "error_code_tests.rs"] mod error_code_tests;` |
| 131 | `#[cfg(test)] #[path = "type_tests.rs"] mod type_tests;` |
| 135 | `#[cfg(test)] #[path = "index_tests.rs"] mod index_tests;` |
| 140 | `#[cfg(test)] #[path = "index_maintenance_tests.rs"] mod index_maintenance_tests;` (commented "vb-3wn7x") |
| 144 | `#[cfg(test)] #[path = "artifact_tests.rs"] mod artifact_tests;` |
| 148 | `#[cfg(test)] #[path = "blob_tests.rs"] mod blob_tests;` |
| 152 | `#[cfg(test)] #[path = "header_tests.rs"] mod header_tests;` |
| 156 | `#[cfg(test)] #[path = "hydrate_tests.rs"] mod hydrate_tests;` |
| 160 | `#[cfg(test)] #[path = "process_lock_tests.rs"] mod process_lock_tests;` |
| 164 | `#[cfg(test)] #[path = "record_tests.rs"] mod record_tests;` |
| 168 | `#[cfg(test)] #[path = "recover_tests.rs"] mod recover_tests;` |
| 172 | `#[cfg(test)] #[path = "recovery_type_tests.rs"] mod recovery_type_tests;` |
| 176 | `#[cfg(test)] #[path = "replay_core_tests.rs"] mod replay_core_tests;` |
| 180 | `#[cfg(test)] #[path = "snapshot_tests.rs"] mod snapshot_tests;` |

The canonical pattern for `edge_case_tests.rs` is therefore a 3-line insertion between `lib.rs:180` and the next public module declaration (`pub mod queue;` at lib.rs:183, since the `pub mod tests;` and `pub mod vb_2bok_durability_gate_tests;` belong to test mode but are not under `#[path = ...]`):

```rust
#[cfg(test)]
#[path = "edge_case_tests.rs"]
mod edge_case_tests;
```

This mirrors the 16 sibling declarations exactly and is the single, minimal, lowest-risk resolution.

---

## 5. Cargo.toml verification (bead hypothesis check)

The bead hypothesis "referenced by no Cargo.toml" is technically correct as far as it goes — but the actual wiring path is **not** a `[[test]]` entry. Verified:

- `crates/vb_storage/Cargo.toml` has NO `[[test]]` table.
- No other `Cargo.toml` in the workspace references `edge_case_tests.rs` (`rtk rg 'edge_case_tests' Cargo.toml` → 0 hits).
- `Cargo.toml` is not the right place for this orphan: the file uses `use crate::*` (intra-crate imports), so it must be wired via `mod`/`#[path = "..."]` inside the crate, not as a standalone integration test.

`vb_storage/Cargo.toml` already has all required dev-deps:

```toml
[dev-dependencies]
proptest = { workspace = true }
tempfile.workspace = true
```

No `Cargo.toml` change is needed for the wire-recommendation path.

---

## 6. Stale-content audit (bead's "delete if truly dead" branch)

The bead explicitly allows deletion if the content is stale. We checked each symbol used by `edge_case_tests.rs` against the current production source:

| Symbol used | Production location | Status |
| --- | --- | --- |
| `FjallJournal::open(path, None)` | `crates/vb_storage/src/journal/core.rs:79` | OK |
| `FjallJournal::append_journaled(&event)` | `crates/vb_storage/src/journal/append.rs:7` | OK |
| `FjallJournal::append_strict(&event)` | `crates/vb_storage/src/journal/append.rs:35` | OK |
| `FjallJournal::persist_strict()` | `crates/vb_storage/src/journal/append.rs:81` | OK |
| `FjallJournal::fail_next_persist_for_test()` | `crates/vb_storage/src/journal/core.rs:227` (`pub(crate)`) | OK (visibility works because test file is in same crate) |
| `FjallJournal::events_for_run(run)` | `crates/vb_storage/src/journal/replay.rs:59`, `readonly.rs:70` | OK |
| `FjallJournal::close()` | `crates/vb_storage/src/journal/core.rs:222` | OK |
| `FjallJournal::put_blob` / `blob` | `crates/vb_storage/src/blobs.rs:20,35` | OK |
| `FjallJournal::put_compiled_ir` / `compiled_ir` | `crates/vb_storage/src/journal/source.rs:53,68` | OK |
| `FjallJournal::put_workflow_source` / `workflow_source` | `crates/vb_storage/src/journal/source.rs:20,35` | OK |
| `FjallJournal::put_snapshot` / `snapshot` | `crates/vb_storage/src/snapshots.rs:31` | OK |
| `FjallJournal::put_run_header` / `run_header` | `crates/vb_storage/src/headers.rs:18` | OK |
| `FjallJournal::batch()` | `crates/vb_storage/src/batch/mod.rs` | OK |
| `BatchBuilder::append_event` / `strict` / `commit` | `crates/vb_storage/src/batch/append_event.rs:42`, `commit.rs:7,20` | OK |
| `BatchBuilder::put_workflow_source` / `len` | `crates/vb_storage/src/batch/putters.rs:23`, `types.rs:48` | OK |
| `JournalWriterQueue::new(cap, batch, limits)` | `crates/vb_storage/src/queue/writer.rs:40` | OK (matches `(usize, usize, StorageLimits)` signature) |
| `JournalWriterQueue::enqueue_journaled` / `enqueue_strict` | `crates/vb_storage/src/queue/writer.rs:67,72` | OK |
| `JournalWriterQueue::drain_all` / `flush_batch` / `shutdown` | `crates/vb_storage/src/queue/writer.rs:237,152,266` | OK |
| `encode_record` / `decode_record` | `crates/vb_storage/src/codec/mod.rs:60,82` | OK |
| `RecordKind::{WorkflowSource, RunHeader, Blob, IndexUpdate, RunAccepted}` | `crates/vb_storage/src/records.rs:141,145,202,204,147` | OK |
| `MAGIC_BLOB` / `MAGIC_INDEX_RECORD` / `MAGIC_JOURNAL_EVENT` | `crates/vb_storage/src/constants.rs:66,72,62` | OK |
| `MAX_RUN_HEADER_BYTES` / `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` / `DIGEST_BYTES` | `crates/vb_storage/src/constants.rs:94,88,82` | OK |
| `StorageLimits::DEFAULT` | `crates/vb_storage/src/types.rs:17` | OK |
| `JournalError::StrictDurabilityFailed` | `crates/vb_storage/src/journal/append.rs:84` (return) | OK |
| `JournalError::PayloadTooLarge` | `crates/vb_storage/src/error/mod.rs` (variant present) | OK |
| `JournalError::RecordKindFamilyMismatch { magic, kind }` | `crates/vb_storage/src/error/mod.rs:80` | OK |
| `JournalError::DuplicateEvent` | `crates/vb_storage/src/batch/append_event.rs:63` (return) | OK |
| `JournalError::QueueShutdown` | `crates/vb_storage/src/error/mod.rs:45` (variant present), `queue/writer.rs:82` (return) | OK |
| `JournalEvent::RunAccepted { run, seq, workflow }` | `crates/vb_storage/src/events.rs` | OK |
| `JournalEvent::StepStarted { run, seq, step, attempt }` | `crates/vb_storage/src/events.rs` | OK |
| `vb_core::{RunId, StepIdx, WorkflowDigest, WorkflowId}` newtypes | `crates/vb_core/src/lib.rs` | OK |
| `blake3::hash(...)` | external dev-dep transitive via `vb_storage/Cargo.toml:9` | OK |

**Stale-content verdict:** NOT stale. All 26 tests reference live, public/`pub(crate)`-visible APIs in the current production source. No symbol is missing, renamed, or with a changed signature.

We also verified there is no test-name collision: `rtk rg -n 'fn (persist_strict_handles_simulated_failure|multiple_threads_append_to_different_runs|very_large_blob_payload|rapid_open_close_cycles_preserve_data|queue_rejects_all_writes_after_shutdown|encode_rejects_unknown_magic|empty_batch_strict_commits_successfully|decode_rejects_zero_max_payload_with_nonzero_payload|queue_drain_all_with_large_batch_relative_to_capacity|queue_capacity_one_single_enqueue_dequeue|batch_len_zero_after_digest_mismatch_abort|batch_commit_then_second_batch_with_same_run_seq_rejected|encode_accepts_index_update_with_index_magic|encode_accepts_run_header_with_index_magic|encode_rejects_zero_length_payload_serialization|many_events_per_run|very_large_run_header_values|very_large_workflow_source_payload|very_large_compiled_ir_payload|very_large_snapshot_with_many_slots|concurrent_read_while_another_writes|open_append_close_reopen_verify|rapid_open_close_without_writes|persist_strict_recovers_after_simulated_failure|concurrent_batch_writes_from_multiple_threads|concurrent_enqueue_to_writer_queue)\b'` returns 26 hits **all in `edge_case_tests.rs`** — no duplicates in `tests.rs`, `journal/tests.rs`, `recovery/tests.rs`, `codec/tests.rs`, or any other wired module.

The orphan status is purely a missing `mod` declaration; the file is otherwise owned, recognized, and active. **Delete is rejected on both stale-content grounds and on ownership/recognition grounds** (see §7).

---

## 7. Ownership and recognition (delete-vs-wire verdict)

Verified the file is owned and tracked:

- `.config/source-length-exceptions.txt:150` lists `crates/vb_storage/src/edge_case_tests.rs|lewis|vb-jpq7.47|split-or-retire-before-release|Pre-existing over-300-line Rust source baseline (637 lines); must be split by domain responsibility or retired before removing exception.`
  - This means the project has a tracked owner (`lewis`), a tracked split-or-retire bead (`vb-jpq7.47`), and an explicit removal plan (`split-or-retire-before-release`). The file is **recognized project state**, not orphaned dead code.
- `to-fix/wave3/agent-09-verus.md:19,45` and `to-fix/wave3/agent-07-test-reviewer.md:23` reference the file as part of the dormant-test-file wave-3 audit. The 9 dormant files listed in agent-09-verus.md include `edge_case_tests.rs`; the 16 wired files in lib.rs (`error_tests`, `error_code_tests`, `type_tests`, `index_tests`, `index_maintenance_tests`, `artifact_tests`, `blob_tests`, `header_tests`, `hydrate_tests`, `process_lock_tests`, `record_tests`, `recover_tests`, `recovery_type_tests`, `replay_core_tests`, `snapshot_tests`) match the wave-3 wiring subset; `edge_case_tests.rs` is the missing wire entry.

**Resolution recommendation:** **WIRE**, not delete. Concretely: add the 3-line `#[cfg(test)] #[path = "edge_case_tests.rs"] mod edge_case_tests;` declaration to `crates/vb_storage/src/lib.rs`, immediately after the `snapshot_tests` declaration at line 180 (or any other consistent position among the 16 `#[path = "..."]` siblings). Delete is rejected because:

1. Content is not stale (verified §6).
2. Test names are unique (no duplicate coverage lost).
3. File is on the source-length ledger with an active owner and removal plan (cannot be silently deleted without breaking the ledger's promise that the file either gets wired or split before release).
4. 26 behavior tests covering disk-full, concurrency, large values, open/close cycles, record boundary, batch edge, and queue edge cases are currently compiled to nothing — wiring them restores lost CI coverage for free.

---

## 8. Risk tags

| Risk | Where | Notes for downstream agents |
| --- | --- | --- |
| `concurrency` | edge_case_tests.rs:84, 123, 163, 199 | 4 tests use `std::thread::spawn` + `Arc<FjallJournal>` / `Arc<JournalWriterQueue>`. Shared mutable state is the Fjall keyspace; FjallJournal's append path takes `&self` and uses internal locking (verified `JournalWriterQueue::state: Mutex<...>` at `queue/writer.rs:33`). Stacked Borrows Tree should be safe but no atomic ordering proof. Recommend optional Loom lane for the 4 multi-thread tests if proof-planner wants formal coverage; default Rust lane is acceptable per existing pattern in `journal/tests.rs` and `recovery/tests.rs`. |
| `temporal` | edge_case_tests.rs:358, 385, 400 | Rapid open/close cycles depend on filesystem ordering; on slow CI disks `tempfile::tempdir()` can race. Already uses unique `tempfile::tempdir()` per test, no global state. |
| `persistence` | edge_case_tests.rs:36, 58, 249–351 | Disk-full simulation uses `fail_next_persist_for_test` (pub(crate) hook); very-large tests allocate 128 KiB–1 MiB payloads via `vec![…; N]`. Fjall tempdir is per-test. |
| `arithmetic` | edge_case_tests.rs:313, 315, 318, 321 | Uses `u64::MAX`, `u32::MAX`, `RunId::new(u64::MAX)`. All bounded by existing newtype contracts in `vb_core`. |
| `parser/codec` | edge_case_tests.rs:443–530 | 5 record-boundary tests exercise `encode_record`/`decode_record` rejecting unknown magic, accepting mixed-family magics, zero/max payload. Pure codec tests, no I/O. |
| `file-size` | edge_case_tests.rs whole file | 637 lines exceeds 300-line rule but is on `.config/source-length-exceptions.txt:150` (split-or-retire-before-release plan, owner `lewis`, bead `vb-jpq7.47`). Wiring it does not change the line count; splitting is a separate downstream concern tracked by `vb-jpq7.47`. |
| `dependency` | `Cargo.toml` already has `tempfile`, `proptest`, `blake3` (transitive), `fjall` | No `Cargo.toml` change needed. |
| `user-visible-behavior` | none | All tests are internal; no public-API churn. Wiring only affects `cargo test -p vb_storage --lib` result count. |
| `migration` | none | No schema/version change. |

---

## 9. Existing tests / proofs / evidence adjacent to this scope

Verified location of related artifacts:

- `crates/vb_storage/src/tests.rs` (7700 lines) — main wired lib test, exposes Fjall-journal API surface contracts.
- `crates/vb_storage/src/journal/tests.rs` (2426 lines) — journal-specific tests including `fail_next_persist_for_test` usage at line 2630.
- `crates/vb_storage/src/queue/tests.rs` (1071 lines) — queue-specific tests.
- `crates/vb_storage/src/recovery/tests.rs` (2626 lines) — recovery tests.
- `crates/vb_storage/src/codec/tests.rs` (2557 lines) — codec tests (some overlap with edge_case record-boundary tests but edge_case focuses on magic/kind rejection while codec tests focus on serialization round-trip).
- `crates/vb_storage/src/queue/writer.rs` lines 40–279 — production `JournalWriterQueue` impl.
- `crates/vb_storage/src/journal/append.rs` lines 7–84 — production append paths.
- `crates/vb_storage/src/journal/core.rs` lines 79–227 — production `FjallJournal::open`/`close`/`fail_next_persist_for_test`.
- `crates/vb_storage/src/types.rs` lines 10–32 — `StorageLimits::DEFAULT`.

Existing evidence (pre-this-bead):

- `.beads/vb-2bok/qa-report.md:5` — `cargo test -p vb_storage --lib` baseline (922 passed pre-this-bead).
- `.beads/vb-2yb8/qa-report.md:23,98` — `cargo test -p vb_storage --lib` baseline (922 passed).
- `.beads/vb-core-atomic-admission/STATE.md:1349` — `cargo test -p vb_storage --lib` (924 passed; 0 failed).
- `to-fix/wave3/agent-09-verus.md:19,45` — wave-3 dormant-test audit (P0 evidence: `cargo test -p vb_storage --lib 'put_blob_is_idempotent'` returns 0 tests because of missing `mod`).

Wiring `edge_case_tests.rs` is expected to add 26 more `cargo test -p vb_storage --lib` tests, which will move the count from ~924 to ~950 (matches historical wave-3 contract claim of "+84 surfaced" — but that was for all 12 dormant files, not just `edge_case_tests.rs`; this single-file wire adds 26).

---

## 10. Open questions for downstream agents

- **Q1 (for rust-contract / proof-planner):** Do any of the 4 concurrent tests (`multiple_threads_append_to_different_runs`, `concurrent_enqueue_to_writer_queue`, `concurrent_batch_writes_from_multiple_threads`, `concurrent_read_while_another_writes`) need a Loom permutation lane, or is default-Rust multi-threading sufficient? Recommendation: default-Rust is sufficient because `FjallJournal::append_*` takes `&self` and `JournalWriterQueue` wraps a `Mutex`; this matches the existing pattern in `journal/tests.rs:2598+` and `recovery/tests.rs`. **No new Loom lane required.**
- **Q2 (for test-planner):** Should the 5 record-boundary tests (lines 443–530) be Kani-ified for full magic/kind coverage? Out of scope for the wire-only fix — leave as default-Rust behavior tests; existing `kani_record_magic.rs` / `kani_record_kind.rs` harnesses already cover the codec invariants.
- **Q3 (for holzman-rust):** The file-level `#![allow(...)]` block at lines 1–9 (clippy::as_conversions, cast_possible_truncation, expect_used, indexing_slicing, panic, panic_in_result_fn, unwrap_used) matches the existing pattern in `tests.rs:1-13`. Same applies to all 16 sibling `_tests.rs` files. Verify against `.config/lint-baseline.md` / `docs/rust-governance.md` if those exist; pattern is consistent with `to-fix/wave3/agent-07-test-reviewer.md:23` which confirms `--lib` clippy is clean but `--lib --tests` flags E0453 on these allow-blocks (pre-existing, unrelated to wire fix).
- **Q4 (for black-hat / evidence-packaging):** Wiring changes `cargo test -p vb_storage --lib` from N to N+26 tests. Capture pre/post `cargo test -p vb_storage --lib --no-run` outputs (or `--lib` runs) as evidence. If the wire succeeds, also capture one `cargo test -p vb_storage --lib edge_case` or similar filter that returns 26 hits, proving the wiring is live.

---

## 11. Recommended downstream owners

- **rust-contract:** No new contract needed. The wiring is a pure module-declaration insertion.
- **proof-planner:** No new proof obligations. All 26 tests are default-Rust behavior tests; concurrent tests follow the existing `journal/tests.rs` precedent and don't need a separate Loom lane.
- **test-planner:** No new test-plan needed. The 26 tests are pre-existing behavior tests being un-orphaned.
- **holzman-rust:** Single 3-line edit to `crates/vb_storage/src/lib.rs` between lines 180 and 183. No production logic change. Follows the existing 16-sibling pattern exactly.
- **formal-verifier:** No new Verus/Kani/Flux obligation. (Kani harnesses live under `kani_*` modules; this file is unit-test territory.)
- **evidence-packaging:** Capture `cargo test -p vb_storage --lib` pre/post run counts, plus a `cargo test -p vb_storage --lib edge_case` filtered run returning 26 tests.

---

## 12. UNKNOWN / MISSING

- UNKNOWN: whether `edge_case_tests.rs` was ever compiled at any point in the project's history (no `git log` evidence of `lib.rs` ever declaring it; only the 2026-05-23 add-commit `a95354665` exists).
- MISSING: there is no companion `[[test]]` entry anywhere — confirmed by `rtk rg 'edge_case_tests' Cargo.toml` returning 0 hits. The orphan is purely a missing `mod` declaration, not a Cargo wiring issue.
- MISSING: there is no `.beads/vb-n5k6v/test-plan.md` or `.beads/vb-n5k6v/contract.md` (this is State 2 explore, not State 3/4 — those files are not yet required).

