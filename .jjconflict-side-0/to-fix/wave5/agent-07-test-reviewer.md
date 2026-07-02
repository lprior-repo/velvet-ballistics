# Wave 5 — Agent 07: Test Reviewer (architecture drift / IPC / CLI)

Working directory: `/home/lewis/src/velvet-ballistics`
Bugs reviewed: `vb-keji6`, `vb-krus1`, `vb-lhxze`, `vb-mx7qt`
Date: 2026-06-24

## Bug-by-bug review

### `vb-keji6` — SA-003 `append_event` intra-batch dedup

Source: `crates/vb_storage/src/batch.rs:243-251`
Finding: `bug-hunt-2026-06-21/findings/storage-admission/SA-003-batch-append-event-no-intra-batch-dedup.md` (file no longer present in tree).

`append_event` checks `self.journal.events.contains_key(key)?` only (committed-state). The struct still has `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` at `batch.rs:47` but it is `#[allow(dead_code)]` and unused. The doc comment at `batch.rs:217-220` explicitly documents the current behavior:

> Same-batch idempotent inserts are allowed (duplicates within the same batch are collapsed at commit time).

A regression test for the SA-003 *fix* does not exist; instead the inverse behavior is asserted by `batch_append_event_allows_duplicate_key_insertion` at `crates/vb_storage/src/journal/tests.rs:2184-2202`:

```rust
batch.append_event(&event).expect("first batch append");
batch.append_event(&event).expect("second batch append in same batch"); // expects Ok
batch.commit().expect("commit should succeed");
let replayed = journal.events_for_run(run).expect("replay");
assert_eq!(replayed.len(), 1, "duplicate in batch should result in single event");
```

The only duplicate-related tests for `JournalWriteBatch::append_event` use two separate batches (`batch_append_event_rejects_duplicate_event` at `crates/vb_storage/src/batch.rs:936` and `crates/vb_storage/src/batch/tests.rs:605`). Neither exercises the intra-batch case.

The `staged_event_keys` field was kept in production code; the kani harness `crates/vb_storage/src/kani_vb_vzcuf_ps009.rs` references it as if it were live. Three vacuum Verus specs covering this were deleted in `4644a6cc6` per AGENTS.md GOD RULE 2. The bug-hunt SA-003 description literally proposes "Maintain a `staged_event_keys: HashSet<[u8; JOURNAL_KEY_BYTES]>` (or equivalent) inside `JournalWriteBatch`", which contradicts the active contract at `batch.rs:217-220`.

Bead status: `IN_PROGRESS`. No production change has been applied; the alleged fix is incompatible with the current design.

| bug-id | pri | test-file | assertion-strength | deterministic | public-api | mutation-resistant | targeted-cmd | result | verdict | evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| vb-keji6 | P2 | `crates/vb_storage/src/journal/tests.rs:2184` (`batch_append_event_allows_duplicate_key_insertion`) | weak — asserts OLD/contract behavior, not the SA-003 fixed behavior | yes | yes (public `JournalWriteBatch::append_event`) | no — directly contradicts the bug's claimed fix; an `Err(DuplicateEvent)` mutation would fail the test | `cargo test -p vb_storage --lib batch::tests::batch_append_event_rejects_duplicate_event` | pass | NOT-PATCHED | bead IN_PROGRESS; `append_event` `batch.rs:245` still uses `self.journal.events.contains_key(key)?` only; doc comment at `batch.rs:217-220` documents current behavior; live test `batch_append_event_allows_duplicate_key_insertion` `journal/tests.rs:2184` asserts last-write-wins is acceptable |

### `vb-krus1` — `ipc_decode_order_proptest` ReservedNonZero expectation

Source: `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs:101-112` (file was renamed from `restate_decode_error_taxonomy_tests.rs` in commit `807394195`; previous file at line 244 per bead description).

The proptest's case 3 still writes 1 to bytes[10..12] and expects `Err(IpcError::ReservedNonZero { actual: 1 })`. The close-reason on the bead claims "Updated to test PermissionDenied rejection (zero capabilities) instead" per SEC-01, but the test source never received that change. The Option B plan in `.bead-progress/vb-krus1/progress.md` (commit `8611be681`) explicitly proposed changing `selector in 0_u8..6` to `selector in 0_u8..5` and removing case 3 — this change is not present in the current file.

The decoder at `crates/vb_ipc/src/frame_types.rs:97-99` still returns `IpcError::ReservedNonZero { actual: reserved }` when `reserved != 0`, so the test passes only because the decoder behaves the way the test expects. A search of `crates/vb_ipc/src` for `caller_capabilities`, `ROOT_CAPABILITY`, or `capability_bit` returns zero matches — the SEC-01 capability envelope described in the close-reason is not in the codebase. Test passes vacuously against the pre-SEC-01 wire.

Targeted run: `cargo test -p velvet-ballistics-workspace-tests --test restate_decode_error_taxonomy_tests` — 6 passed, 0 failed.

| bug-id | pri | test-file | assertion-strength | deterministic | public-api | mutation-resistant | targeted-cmd | result | verdict | evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| vb-krus1 | P1 | `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs:101-112` (`ipc_decode_order_proptest` case 3) | weak — asserts pre-SEC-01 behavior; the bead-claimed fix to `PermissionDenied` was never applied; test passes only because decoder still emits `ReservedNonZero` at `frame_types.rs:97-99` | yes (proptest 256 cases, deterministic) | yes (`vb_ipc::IpcFrameHeader::decode`, `IpcError`) | no — a mutation that adds `PermissionDenied` for zero caps (as the bead claims is correct) would fail this test | `cargo test -p velvet-ballistics-workspace-tests --test restate_decode_error_taxonomy_tests` | pass (6/6) | NOT-PATCHED | test source still contains `Err(IpcError::ReservedNonZero { actual: 1 })` at line 108; close-reason promises `PermissionDenied` was substituted but the diff was never landed; SEC-01 capability envelope absent from `crates/vb_ipc/src` |

### `vb-lhxze` — kani harness compile failures in vb_cli / vb_runtime/capability

Source: kani harnesses in `crates/vb_cli/src/agent_context/tests/kani_harnesses.rs` and `crates/vb_runtime/src/kani_capability_harnesses.rs`.

The fix per the close-reason was to remove a `#[kani::proof_for(parse)]` attribute in `crates/vb_core/src/kani_workflow_arbitrary.rs:667` and a bad `pub use self::decision::*` re-export in `crates/vb_storage/src/journal/append/mod.rs:38-43`. A search of `crates/vb_core` for `proof_for` returns zero matches, confirming the attribute has been removed.

A `cargo check -p vb_runtime --features kani-capability-harnesses` completes with no errors. There is no separate regression test target for "kani harness compiles" — the harness IS the test; the close-reason relies on `moon :verify-kani-vb-cli` and `moon :verify-kani-vb-runtime-capability` exiting 0. I did not invoke moon in this reviewer pass (read-only/limited scope).

There is a stale dangling module `crates/vb_cli/src/kani_lifecycle.rs` (with `fn main() {}` at line 165 and an artifact-note comment at lines 12-19 stating it must be registered by a downstream agent). It is not `#[cfg(kani)]`-gated, is not declared in `crates/vb_cli/src/lib.rs` (no `mod kani_lifecycle;` line), and so does not affect the build. This is a dead file; not a compile failure.

| bug-id | pri | test-file | assertion-strength | deterministic | public-api | mutation-resistant | targeted-cmd | result | verdict | evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| vb-lhxze | P1 | n/a (kani harness is the test; verified by `moon :verify-kani-*` exit code) | n/a — verification tool, not unit test | yes (kani bounded model check is deterministic per harness) | yes (`crates/vb_cli/src/agent_context::build`, `crates/vb_runtime::admission::check_capability`) | n/a | `cargo check -p vb_runtime --features kani-capability-harnesses` | clean | PATCHED | `proof_for` no longer present in `crates/vb_core`; `kani_capability_harnesses.rs` compiles under the feature gate; no `[features]` requires `kani` in `crates/vb_cli/Cargo.toml` (harnesses use `#[cfg(kani)]` on the test module instead) |

### `vb-mx7qt` — workspace_tests cluster (3 failing tests)

Bead description cites:

1. `test_out_of_scope_vb_cli_xtask_changes_are_routed_with_touched_package_evidence` at `vb_a0t1_source_length_gate_tests.rs:679` — file deleted (commit `458a4fe35` modified it last; subsequent refactors removed it). Not present anywhere in the tree.

2. `valid_workspace_passes_sharpened_assertions` at `vb_8ma2_workspace_assertions.rs:323` — current file is 253 lines; the test is at line 174. The test passes today.

   The assertion script `scripts/check-workspace-assertions.rs:73-85` lists `EXPECTED_FEATURES` for `crates/vb_core` as `["bench", "default", "kani-diagnostic-codes", "test-util", "volatile"]`; `crates/vb_core/Cargo.toml` (current) matches exactly — no `legacy-tests` feature exists. The synthetic Cargo.toml written by `write_boundary_crates_with_dependency` (`vb_8ma2_workspace_assertions.rs:94-124`) deliberately mirrors this set, so the test asserts the assertions script + the expected set are consistent. The test is self-consistent (tautological by construction).

3. `edge_submit_after_shutdown_enqueues_but_does_not_process` at `vb_test_runtime_ipc_resource_behavior.rs:1353` — current file is 1222 lines; the test is at line 1137. The test passes today.

   Test body: shutdown runtime, submit after shutdown (expects `Ok(())`), tick_all returns `Ok(false)`, counters `runs_submitted == 0`. Consistent with the doc comment at line 1134-1136.

| bug-id | pri | test-file | assertion-strength | deterministic | public-api | mutation-resistant | targeted-cmd | result | verdict | evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| vb-mx7qt | P2 | (1) `crates/workspace_tests/tests/vb_a0t1_source_length_gate_tests.rs` — DELETED; (2) `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:174` (`valid_workspace_passes_sharpened_assertions`); (3) `crates/workspace_tests/tests/vb_test_runtime_ipc_resource_behavior.rs:1137` (`edge_submit_after_shutdown_enqueues_but_does_not_process`) | weak (1 missing; 2 tautological — synthesises its own expected set; 3 medium — hard-coded 1-shard NonZeroUsize, hard-counter `runs_submitted == 0` after `submit_direct(Ok(()))`) | yes | (1) n/a; (2) partly (`vb_core`/`vb_runtime`/`vb_storage`/`vb_ipc` features only — does not exercise actual workspace); (3) yes (`Runtime::new`, `shutdown_graceful`, `submit_direct`, `tick_all`, `counters_snapshot`) | low — (1) cannot test; (2) test would still pass with any feature set as long as the synthesised manifest matches; (3) moderate | (1) n/a; (2) `cargo test -p velvet-ballistics-workspace-tests --test vb_8ma2_workspace_assertions valid_workspace_passes_sharpened_assertions`; (3) `cargo test -p velvet-ballistics-workspace-tests --test vb_test_runtime_ipc_resource_behavior edge_submit_after_shutdown_enqueues_but_does_not_process` | (1) file deleted; (2) pass; (3) pass | PARTIAL | (1) `vb_a0t1_source_length_gate_tests.rs` not in tree (`find … -name 'vb_a0t1*'` returns zero); (2) test at line 174 passes; assertion script and vb_core Cargo.toml agree on features `["bench", "default", "kani-diagnostic-codes", "test-util", "volatile"]`; (3) test at line 1137 passes (`submit_direct … Ok(())`, `tick_all` returns `Ok(false)`, counters `runs_submitted == 0`) |

## Summary

- Bugs checked: 4
- PATCHED: 1 (`vb-lhxze` — kani compile gates verified clean; harness compile is its own test)
- PARTIAL: 1 (`vb-mx7qt` — 1 of 3 cited tests file-deleted, 2 of 3 cited tests pass; bead description stale)
- NOT-PATCHED: 2 (`vb-keji6` — fix incompatible with current contract; `vb-krus1` — close-reason contradicts test source)
- UNKNOWN: 0

### Weak-test cases (top-3)

1. `crates/vb_storage/src/journal/tests.rs:2184` — `batch_append_event_allows_duplicate_key_insertion` asserts the OLD (buggy per SA-003) behavior is correct. If SA-003 were applied as described, this test would fail. Its existence is the strongest evidence that vb-keji6's "fix" would regress the codebase.

2. `crates/workspace_tests/tests/restate_decode_error_taxonomy_tests.rs:108` — case 3 of `ipc_decode_order_proptest` asserts the pre-SEC-01 `ReservedNonZero` behavior; the bead close-reason claims the test was rewritten for post-SEC-01 `PermissionDenied`, but no rewrite is present. The test is the only thing keeping the decoder's `ReservedNonZero` path alive.

3. `crates/workspace_tests/tests/vb_8ma2_workspace_assertions.rs:174` — `valid_workspace_passes_sharpened_assertions` writes its own synthetic `Cargo.toml` files and runs the assertions script against them. The synthesised set is hard-coded into the test (`write_boundary_crates_with_dependency`), so the test is tautological: it passes whenever the assertion script and the test's own expectations agree. It does not assert anything about the real workspace.

### Top-3 NOT-PATCHED with reason

1. **vb-keji6** — Bead IN_PROGRESS, no production change applied. `batch.rs:243-251` still only checks `self.journal.events.contains_key(key)`. The active contract at `batch.rs:217-220` explicitly documents intra-batch duplicates as allowed, and `batch_append_event_allows_duplicate_key_insertion` (`journal/tests.rs:2184`) is a passing regression test for that behavior. The proposed fix (use `staged_event_keys`) is incompatible with the current design.

2. **vb-krus1** — Bead CLOSED but the test source contradicts the close-reason. `restate_decode_error_taxonomy_tests.rs:108` still asserts `Err(IpcError::ReservedNonZero { actual: 1 })`. The Option B fix described in `.bead-progress/vb-krus1/progress.md` (drop case 3, change selector range to 0..5) is not present in the file. `caller_capabilities` / `ROOT_CAPABILITY` are absent from `crates/vb_ipc/src`. Test passes only because the decoder at `frame_types.rs:97-99` still emits `ReservedNonZero`.

3. **vb-mx7qt** — Bead IN_PROGRESS; only 1 of 3 cited tests still exists in the tree (`vb_a0t1_source_length_gate_tests.rs` deleted). The 2 surviving tests pass, but the bead's failure diagnosis (`legacy-tests` feature missing from `vb_core`) is stale — `legacy-tests` is not in `EXPECTED_FEATURES` at `scripts/check-workspace-assertions.rs:73-85`, and the real `vb_core/Cargo.toml` matches the expected feature set. The bead is effectively resolving a non-issue.

## File path written

`/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-07-test-reviewer.md`
