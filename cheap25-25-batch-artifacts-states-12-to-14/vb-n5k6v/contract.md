# Contract — Wire Orphaned `edge_case_tests` Module (vb-n5k6v)

## Bead ID

`vb-n5k6v` — Tests: wire orphaned `edge_case_tests` or delete stale file (P1 bug)

## Bead Scope (verbatim)

> An orphaned `edge_case_tests` file is referenced by no `Cargo.toml`
> and has stale content. Either wire it into a target crate's
> `[[test]]` entries, or delete it if it's truly dead.

## Resolution Branch

**WIRE** (not DELETE). Justifications:

1. **Content is fresh**: all 32 symbols used by the 26 tests resolve
   to live production source (verified `codebase-map.md` §6).
2. **Tests are unique**: 26 names have zero collisions across the
   workspace (verified `codebase-map.md` §6).
3. **File is owned**: `.config/source-length-exceptions.txt:150`
   lists `crates/vb_storage/src/edge_case_tests.rs` with owner
   `lewis` and removal plan `vb-jpq7.47`
   (split-or-retire-before-release).
4. **Wave-3 dormant sweep**: `to-fix/wave3/agent-09-verus.md:19,45`
   lists this file as the lone remaining dormant `vb_storage` test
   file (the other 8 are already wired at `lib.rs:123-180`).

## Contract Lane

**Rust-local implementation** — build-graph/test-orchestration
repair. **No formal verifier required.** The change is exclusively
a 3-line insertion into `crates/vb_storage/src/lib.rs:182` that
mirrors the canonical pattern of the 16 sibling `#[path = "..."]`
declarations already at `lib.rs:118-181`.

---

## Pre-conditions (must hold at contract acceptance)

1. `crates/vb_storage/src/edge_case_tests.rs` exists at 637 lines
   (`rtk wc -l` confirms).
2. `crates/vb_storage/src/lib.rs` is 246 lines
   (`rtk wc -l` confirms).
3. The file has a top-level `#![allow(...)]` block at lines 1-9
   (8 clippy allows; same shape as sibling `_tests.rs` files).
4. The file has an inner `mod edge_case_tests { ... }` wrapper at
   lines 11-12, containing 26 `#[test]` fns.
5. The file uses `use crate::{...}` imports (intra-crate) at lines
   13-23, which is why it MUST be wired via `#[path = "..."] mod`
   inside the crate, NOT via `Cargo.toml` `[[test]]`.
6. `crates/vb_storage/Cargo.toml` has `tempfile` and `proptest` in
   `[dev-dependencies]` (lines 19-21).
7. `blake3` and `fjall` are available as transitive deps
   (`Cargo.toml:9, 12`).
8. The 16 sibling `#[path = "..."]` declarations at
   `lib.rs:118-181` are intact.
9. The 32 symbols used by the 26 tests resolve to live production
   source (verified `codebase-map.md` §6 and `delivery-scope.jsonl`
   rows 4-46).
10. The 26 test names are unique across the workspace (verified
    `codebase-map.md` §6).
11. The file is on `.config/source-length-exceptions.txt:150` with
    owner `lewis` and removal plan `vb-jpq7.47`.

---

## Contract Clauses

Each clause has an ID, a domain claim, an implementation site, an
invariant binding, and a test-pinning reference.

### CC-WIRE-001 — Module Declaration Insertion

**Claim**: A 3-line declaration is inserted into
`crates/vb_storage/src/lib.rs` immediately after line 181 (the
`mod snapshot_tests;` declaration) and immediately before line 183
(the `pub mod queue;` declaration):

```rust
#[cfg(test)]
#[path = "edge_case_tests.rs"]
mod edge_case_tests;
```

**Invariant**: The declaration matches the 16 sibling
`#[path = "..."]` declarations byte-for-byte (modulo the path and
module name).

**Forbidden**:

- Adding `pub` to the declaration.
- Adding `#[cfg(not(test))]` or any non-`cfg(test)` attribute.
- Renaming the module to anything other than `edge_case_tests`.
- Changing the `#[path]` value.
- Adding any other text on the same 3 lines (e.g., doc comments).

**Test pinning**: `cargo check -p vb_storage --tests` returns no
errors; the new module appears in the lib-test binary.

---

### CC-WIRE-002 — Zero Production-Logic Change

**Claim**: The implementation makes zero changes outside the 3-line
declaration insertion.

**Invariant**: `git diff` against the pre-bead `main` shows
**exactly** 3 lines added in `crates/vb_storage/src/lib.rs:182`
and **zero** lines removed or modified anywhere else in the
workspace.

**Forbidden**:

- Modifying any other line in `crates/vb_storage/src/lib.rs`.
- Modifying any other file in `crates/vb_storage/src/`.
- Modifying any file in `crates/vb_storage/tests/`.
- Modifying `crates/vb_storage/Cargo.toml`.
- Modifying `Cargo.lock`.
- Modifying `.config/source-length-exceptions.txt`.
- Modifying any file in `to-fix/wave3/`.

**Test pinning**: `git diff --stat` shows 1 file changed, 3
insertions, 0 deletions.

---

### CC-WIRE-003 — Zero Cross-Crate Change

**Claim**: The implementation makes zero changes outside `vb_storage`.

**Invariant**: `crates/vb_core`, `crates/vb_runtime`, `crates/vb_cli`,
`crates/vb_validate`, and all other crates are unchanged.

**Test pinning**: `cargo check --workspace` continues to pass with
no new warnings.

---

### CC-WIRE-004 — All 26 Surfaced Tests Pass

**Claim**: After wire, `cargo test -p vb_storage --lib edge_case`
runs all 26 tests and reports them under `edge_case_tests::`
module path, with all 26 passing.

**Invariant**:

| Test fn | Bucket | Pass? |
|---------|--------|-------|
| `persist_strict_handles_simulated_failure` | Disk full | YES |
| `persist_strict_recovers_after_simulated_failure` | Disk full | YES |
| `multiple_threads_append_to_different_runs` | Concurrent | YES |
| `concurrent_enqueue_to_writer_queue` | Concurrent | YES |
| `concurrent_batch_writes_from_multiple_threads` | Concurrent | YES |
| `concurrent_read_while_another_writes` | Concurrent | YES |
| `very_large_blob_payload` | Very large | YES |
| `very_large_compiled_ir_payload` | Very large | YES |
| `very_large_workflow_source_payload` | Very large | YES |
| `very_large_snapshot_with_many_slots` | Very large | YES |
| `very_large_run_header_values` | Very large | YES |
| `many_events_per_run` | Very large | YES |
| `rapid_open_close_cycles_preserve_data` | Open/close | YES |
| `rapid_open_close_without_writes` | Open/close | YES |
| `open_append_close_reopen_verify` | Open/close | YES |
| `encode_rejects_unknown_magic` | Record boundary | YES |
| `encode_accepts_run_header_with_index_magic` | Record boundary | YES |
| `encode_accepts_index_update_with_index_magic` | Record boundary | YES |
| `decode_rejects_zero_max_payload_with_nonzero_payload` | Record boundary | YES |
| `encode_rejects_zero_length_payload_serialization` | Record boundary | YES |
| `batch_commit_then_second_batch_with_same_run_seq_rejected` | Batch | YES |
| `batch_len_zero_after_digest_mismatch_abort` | Batch | YES |
| `empty_batch_strict_commits_successfully` | Batch | YES |
| `queue_capacity_one_single_enqueue_dequeue` | Queue | YES |
| `queue_drain_all_with_large_batch_relative_to_capacity` | Queue | YES |
| `queue_rejects_all_writes_after_shutdown` | Queue | YES |

**Test pinning**: `cargo test -p vb_storage --lib edge_case 2>&1 |
tail -30` reports `26 passed; 0 failed; 0 ignored`.

---

### CC-WIRE-005 — Test Count Delta = +26

**Claim**: `cargo test -p vb_storage --lib` test count increases by
exactly 26 relative to the pre-wire baseline.

**Invariant**:

- Pre-wire baseline (verified `2026-07-01` by `PROPTEST_CASES=1
  cargo test -p vb_storage --lib 2>&1 | tail -3` from the isolated
  workdir): 1530 tests. The historical May 2026 captures at
  `.beads/vb-2bok/qa-report.md:5` and
  `.beads/vb-core-atomic-admission/STATE.md:1349` (both 924) are
  the `historic_2026_05_baseline` and are NOT the current pre-wire
  value.
- Post-wire expected: 1530 + 26 = 1556 tests.
- Delta: 26.

**Test pinning**: `cargo test -p vb_storage --lib 2>&1 | tail -5`
reports `test result: ok. 1556 passed; 0 failed; 0 ignored; 0 measured;
...`.

---

### CC-WIRE-006 — File Line Count Unchanged

**Claim**: `crates/vb_storage/src/edge_case_tests.rs` remains at
637 lines after the wire.

**Invariant**: `rtk wc -l crates/vb_storage/src/edge_case_tests.rs`
returns `637` post-wire.

**Forbidden**:

- Adding new test functions to the file.
- Removing existing test functions from the file.
- Refactoring any test function body.

**Test pinning**: `rtk wc -l` returns `637` post-wire.

---

### CC-WIRE-007 — Source-Length Exception Preserved

**Claim**: `.config/source-length-exceptions.txt:150` continues to
list `edge_case_tests.rs` with the same owner (`lewis`), removal
plan (`vb-jpq7.47`), and removal action
(`split-or-retire-before-release`).

**Invariant**: the exception ledger entry is byte-identical
pre- and post-wire.

**Test pinning**: `rtk rg -n 'edge_case_tests'
.config/source-length-exceptions.txt` returns the same single
hit at line 150 pre- and post-wire.

---

### CC-WIRE-008 — Test Names Unique Across Workspace

**Claim**: All 26 test fn names remain unique across the workspace
(no collisions with the 16 sibling modules or any other test file).

**Invariant**: `rtk rg -n 'fn (persist_strict_...|multiple_threads_...|
very_large_...|rapid_open_close_...|open_append_close_...|
encode_rejects_...|encode_accepts_...|decode_rejects_...|
batch_commit_...|batch_len_zero_...|empty_batch_...|
queue_capacity_...|queue_drain_all_...|queue_rejects_...)\b'`
returns exactly 26 hits, all in `edge_case_tests.rs`.

**Test pinning**: post-wire `rtk rg` returns 26 hits, all in
`edge_case_tests.rs`.

---

### CC-WIRE-009 — Cargo.toml Unchanged

**Claim**: `crates/vb_storage/Cargo.toml` is byte-identical
pre- and post-wire.

**Invariant**: `git diff crates/vb_storage/Cargo.toml` returns
empty output.

**Test pinning**: `git diff --stat` shows 0 changes to
`Cargo.toml`.

---

### CC-WIRE-010 — Module-Resolution Path Lints Clean

**Claim**: The new `#[path = "..."] mod ...;` declaration does not
trigger any clippy lint.

**Invariant**: `cargo clippy -p vb_storage --tests -- -D warnings`
returns no errors and no new warnings related to the new
declaration.

**Forbidden**: any clippy lint that would normally fire on a
`#[path = "..."] mod ...;` declaration (the project's
`docs/rust-governance.md` does not list this pattern as an
allow-needed category).

**Test pinning**: `cargo clippy -p vb_storage --tests 2>&1 |
grep -i 'edge_case_tests'` returns empty output.

---

## Clause-to-Code Mapping

| Clause | Production site | Test pinning site |
|--------|-----------------|--------------------|
| CC-WIRE-001 | `crates/vb_storage/src/lib.rs:182` (3 lines added) | `cargo check -p vb_storage --tests` |
| CC-WIRE-002 | (constraint only) | `git diff --stat` |
| CC-WIRE-003 | (constraint only) | `cargo check --workspace` |
| CC-WIRE-004 | (test-surface verification) | `cargo test -p vb_storage --lib edge_case` |
| CC-WIRE-005 | (test-count delta) | `cargo test -p vb_storage --lib 2>&1 \| tail -5` |
| CC-WIRE-006 | (file-shape preservation) | `rtk wc -l crates/vb_storage/src/edge_case_tests.rs` |
| CC-WIRE-007 | `.config/source-length-exceptions.txt:150` (unchanged) | `rtk rg -n 'edge_case_tests' .config/source-length-exceptions.txt` |
| CC-WIRE-008 | (workspace-wide uniqueness) | `rtk rg` over the 26 names |
| CC-WIRE-009 | (constraint only) | `git diff crates/vb_storage/Cargo.toml` |
| CC-WIRE-010 | (lint hygiene) | `cargo clippy -p vb_storage --tests` |

---

## Verifier Lane Profile (for the proof planner)

Per `delivery-scope.jsonl` row 57:

| Lane | Status | Rationale |
|------|--------|-----------|
| `default-rust` (cargo test) | **REQUIRED** | CC-WIRE-004 + CC-WIRE-005: 26 surfaced tests, count delta |
| `kani` | **NOT_REQUIRED** | Existing harnesses (`kani_record_magic`, `kani_record_kind`, etc.) already cover the codec invariants; the 5 record-boundary tests add concrete-value coverage on top |
| `verus` | **NOT_REQUIRED** | No new `exec fn` with non-trivial bound; the wire is a `mod` declaration |
| `flux` | **NOT_REQUIRED** | No new refinement type |
| `loom` | **OPTIONAL** (planner decides) | 4 concurrent tests use `std::thread::spawn` + `Arc`; default-Rust threading is the precedent in `journal/tests.rs:2598+` and `recovery/tests.rs`. Loom is not required. |
| `fuzz` | **NOT_REQUIRED** | All 26 tests are deterministic concrete-value behavior tests |
| `proptest` | **NOT_REQUIRED** | All 26 tests use specific concrete values (e.g., `u64::MAX`, `100`, `1024 * 1024`); proptest would add redundant coverage |
| `tla+` | **NOT_REQUIRED** | No temporal state machine to model |

The proof planner's `proof-obligations.planned.jsonl` should plan:

1. One `cargo test -p vb_storage --lib edge_case` run (CC-WIRE-004).
2. One `cargo test -p vb_storage --lib` run capturing the post-wire
   count (CC-WIRE-005).
3. One `git diff --stat` capture (CC-WIRE-002).
4. One `cargo check --workspace` run (CC-WIRE-003).
5. Six `not_applicable` rows for Kani/Verus/Flux/Fuzz/Proptest/TLA+
   with the rationale documented above.

---

## Open Domain Questions Resolved by This Contract

1. **Wire as integration test (move to `tests/`) or in-source
   module?** → **In-source module (CC-WIRE-001)**. The file uses
   `use crate::{...}` (intra-crate imports) and accesses
   `pub(crate)` methods (`fail_next_persist_for_test`). It is
   structurally an in-source `#[cfg(test)]` module.
2. **Single wire or split into 7 topic buckets?** → **Single wire
   (CC-WIRE-001)**. The 7 buckets are comment-delimited sections,
   not separate files. Splitting would create 7 sibling declarations
   instead of 1 and break the wave-3 audit trail.
3. **Add `pub` to the declaration?** → **No (CC-WIRE-001
   forbidden)**. The module is test-private; `pub` would expose
   internals.
4. **Delete the file?** → **No (rejected in `domain-model.md`)**.
   The file is not stale, is owned, and is on the source-length
   ledger.

## Open Questions Deferred to the Proof Planner

1. **Loom lane for the 4 concurrent tests?** The contract
   recommends default-Rust threading; the planner may upgrade.
2. **Proptest lanes for the 26 tests?** The contract recommends
   concrete-value tests only; the planner may add proptest if it
   finds value.
3. **Performance budget for `+5-15s` test-run delta?** The contract
   acknowledges the delta; the planner may mark slow tests with
   `#[ignore]` if CI budget pressure arises.

---

## Risk-Profile Summary

| Aspect | Status |
|--------|--------|
| Production API change | **None** |
| Diagnostic code change | **None** |
| Cross-crate change | **None** |
| Existing test breakage | **None** (no test was running pre-wire) |
| New test surface | 26 default-Rust behavior tests |
| Verifier lanes | 1 required + 6 not-required (with rationale) |
| Concurrency risk | Medium (4 tests); default-Rust acceptable |
| Persistence risk | Low (per-test tempdir) |
| File-size risk | Low (already on exception ledger) |
| Public-API risk | None |

The bead is a **lowest-blast-radius** internal build-graph fix with a
**bounded verification surface**. The contract commits to:

- Inserting the 3-line `mod` declaration (CC-WIRE-001).
- Touching zero other code (CC-WIRE-002).
- Touching zero other crates (CC-WIRE-003).
- All 26 surfaced tests pass (CC-WIRE-004).
- Test count increases by exactly 26 (CC-WIRE-005).
- File line count unchanged (CC-WIRE-006).
- Source-length exception preserved (CC-WIRE-007).
- Test name uniqueness preserved (CC-WIRE-008).
- Cargo.toml unchanged (CC-WIRE-009).
- Clippy clean (CC-WIRE-010).

END OF CONTRACT.