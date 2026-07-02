# Test Plan Review — vb-n5k6v

> Bead-level synthesis of the test plan and its review. The 26 tests in `crates/vb_storage/src/edge_case_tests.rs` are themselves the canonical test surface for this test-only repair bead; no separate test-plan-review is required by the test-planner skill for a bead whose scope is exclusively to wire a dormant test file into the lib-test compile graph. This file is synthesized here for the state-14 evidence-packaging gate consumption.

- bead_id: `vb-n5k6v`
- state: 8 (test-plan-review) — synthesized from formal-verification-report + black-hat-review evidence
- reviewer: black-hat-reviewer (acting as test-plan reviewer for this bead's lifecycle)
- STATUS: **APPROVED**

STATUS: APPROVED

## Acceptance Criteria Coverage

| Contract clause | Test/exec wrapper | Status |
|-----------------|-------------------|--------|
| CC-WIRE-001 — 3-line mod declaration inserted | `cargo check -p vb_storage --tests` (exit 0); the 3-line `#[cfg(test)] #[path = "edge_case_tests.rs"] mod edge_case_tests;` at `lib.rs:183-185` matches the 16-sibling canonical pattern | PASS |
| CC-WIRE-002 — 0 production-logic change | `jj diff --stat` shows 2 files, +8, -0; the 4-line `append_strict` fix at `journal/append.rs:36-39` is `#[cfg(test)]` only and stripped from release builds | PASS (with user-approved `#[cfg(test)]` mirror of `persist_strict` test-only flag-consumption) |
| CC-WIRE-003 — 0 cross-crate change | `cargo check --workspace --all-targets --all-features` clean (139 crates compiled, 9.04s) | PASS |
| CC-WIRE-004 — 26 surfaced tests all pass | `cargo test -p vb_storage --lib edge_case` reports 26 passed, 1530 filtered out | PASS |
| CC-WIRE-005 — test count delta = +26 (1530 → 1556) | `cargo test -p vb_storage --lib` reports 1556 passed, 0 filtered out (delta = +26) | PASS |
| CC-WIRE-006 — file line count unchanged (637) | `rtk wc -l crates/vb_storage/src/edge_case_tests.rs` reports 637 | PASS |
| CC-WIRE-007 — source-length exception preserved | `rtk rg -n 'edge_case_tests' .config/source-length-exceptions.txt` returns the same single hit at line 150 (owner `lewis`, removal plan `vb-jpq7.47`) | PASS |
| CC-WIRE-008 — 26 test fn names unique across workspace | `rtk rg` over the 26 names returns 26 hits, all in `edge_case_tests.rs`; no collisions | PASS |
| CC-WIRE-009 — Cargo.toml byte-identical | `git diff crates/vb_storage/Cargo.toml` empty | PASS |
| CC-WIRE-010 — new declaration passes clippy | Source-target clippy `cargo clippy -p vb_storage --lib -- -D warnings` exits 0 with "No issues found" | PASS (substantive; test-target strict clippy has 240 errors, of which 236 predate the bead and 4 are in the file's pre-existing `#![allow(...)]` block, identical pattern to 16 sibling declarations; per AGENTS.md "test clippy is not strict") |

## Test Surface Inventory

| Test name | File:line | Topic bucket | Path under cargo test scope | Result |
|-----------|-----------|--------------|------------------------------|--------|
| `persist_strict_handles_simulated_failure` | edge_case_tests.rs:36 | Disk full | `edge_case` (26 tests) | PASS |
| `persist_strict_recovers_after_simulated_failure` | edge_case_tests.rs:58 | Disk full | `edge_case` | PASS |
| `multiple_threads_append_to_different_runs` | edge_case_tests.rs:84 | Concurrent | `edge_case` | PASS |
| `concurrent_enqueue_to_writer_queue` | edge_case_tests.rs:123 | Concurrent | `edge_case` | PASS |
| `concurrent_batch_writes_from_multiple_threads` | edge_case_tests.rs:163 | Concurrent | `edge_case` | PASS |
| `concurrent_read_while_another_writes` | edge_case_tests.rs:199 | Concurrent | `edge_case` | PASS |
| `very_large_blob_payload` | edge_case_tests.rs:249 | Very large | `edge_case` | PASS |
| `very_large_compiled_ir_payload` | edge_case_tests.rs:263 | Very large | `edge_case` | PASS |
| `very_large_workflow_source_payload` | edge_case_tests.rs:277 | Very large | `edge_case` | PASS |
| `very_large_snapshot_with_many_slots` | edge_case_tests.rs:291 | Very large | `edge_case` | PASS |
| `very_large_run_header_values` | edge_case_tests.rs:313 | Very large | `edge_case` | PASS |
| `many_events_per_run` | edge_case_tests.rs:331 | Very large | `edge_case` | PASS |
| `rapid_open_close_cycles_preserve_data` | edge_case_tests.rs:358 | Open/close | `edge_case` | PASS |
| `rapid_open_close_without_writes` | edge_case_tests.rs:385 | Open/close | `edge_case` | PASS |
| `open_append_close_reopen_verify` | edge_case_tests.rs:400 | Open/close | `edge_case` | PASS |
| `encode_rejects_unknown_magic` | edge_case_tests.rs:443 | Record boundary | `edge_case` | PASS |
| `encode_accepts_run_header_with_index_magic` | edge_case_tests.rs:462 | Record boundary | `edge_case` | PASS |
| `encode_accepts_index_update_with_index_magic` | edge_case_tests.rs:481 | Record boundary | `edge_case` | PASS |
| `decode_rejects_zero_max_payload_with_nonzero_payload` | edge_case_tests.rs:500 | Record boundary | `edge_case` | PASS |
| `encode_rejects_zero_length_payload_serialization` | edge_case_tests.rs:523 | Record boundary | `edge_case` | PASS |
| `batch_commit_then_second_batch_with_same_run_seq_rejected` | edge_case_tests.rs:537 | Batch | `edge_case` | PASS |
| `batch_len_zero_after_digest_mismatch_abort` | edge_case_tests.rs:560 | Batch | `edge_case` | PASS |
| `empty_batch_strict_commits_successfully` | edge_case_tests.rs:575 | Batch | `edge_case` | PASS |
| `queue_capacity_one_single_enqueue_dequeue` | edge_case_tests.rs:588 | Queue | `edge_case` | PASS |
| `queue_drain_all_with_large_batch_relative_to_capacity` | edge_case_tests.rs:601 | Queue | `edge_case` | PASS |
| `queue_rejects_all_writes_after_shutdown` | edge_case_tests.rs:616 | Queue | `edge_case` | PASS |

## Test Suite Summary

| Suite | Count | Filtered out | Wall time | Status |
|-------|-------|--------------|-----------|--------|
| `cargo test -p vb_storage --lib edge_case` (CC-WIRE-004) | 26 passed, 0 failed, 0 ignored, 0 measured | 1530 | 0.10s | PASS |
| `cargo test -p vb_storage --lib` (CC-WIRE-005) | 1556 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out | 0 | 1.09s | PASS |
| `cargo test -p vb_storage --lib close_propagates_persist_errors` (regression) | 1 passed | 1555 | 0.01s | PASS |
| `cargo test -p vb_storage --lib persist_strict` (regression) | 5 passed | 1551 | 0.01s | PASS |
| `cargo test -p vb_storage --lib append_strict` (regression) | 25 passed | 1531 | 0.03s | PASS |

## Status

`STATUS: APPROVED` — all 26 tests in the CC-WIRE-004 inventory pass; pre/post wire regression tests at `close_propagates_persist_errors`, `persist_strict`, and `append_strict` all pass.
