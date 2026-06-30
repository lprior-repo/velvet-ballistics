# Wave 5 — Agent 10 Hands-On QA Report

**Date:** 2026-06-24
**Working dir:** /home/lewis/src/velvet-ballistics
**Beads checked:** 4 (vb-rvgjy, vb-sqcig, vb-t5zlm, vb-tqz3v)
**Mode:** Read-only, no beads created, source untouched

## Summary

| bug-id  | pri | affected-crate | targeted-cmd | exit-code | result | verdict     | log-path |
|---------|-----|----------------|--------------|-----------|--------|-------------|----------|
| vb-rvgjy | P0  | velvet-ballistics-workspace-tests | `cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_1_1_red_recovery_contract_test` | 0 | 19/19 pass; `ask_answer_records_exact_clean_taint_when_answer_writes_output`, `event_only_recovery_returns_derived_bool_when_durable_taint_is_derived`, `no_output_step_*` (3), `proptest_no_output_success_never_creates_slot_zero` all `ok` | PATCHED | /tmp/qa-vb-rvgjy-1.log |
| vb-rvgjy | P0  | velvet-ballistics-workspace-tests | `cargo test -p velvet-ballistics-workspace-tests --test recovery_watermark_tests` | 0 | 21/21 pass; `proptest_snapshot_seq_lt_tail_first_seq` `ok` | PATCHED | /tmp/qa-vb-rvgjy-2.log |
| vb-rvgjy | P0  | velvet-ballistics-workspace-tests | `cargo test -p velvet-ballistics-workspace-tests --test slot_written_ordering_integration_tests` | 0 | 16/16 pass; `tail_seq_equal_to_snapshot_seq_fails` `ok` | PATCHED | /tmp/qa-vb-rvgjy-3.log |
| vb-sqcig | P1  | xtask (no test target) | grep verification — `xtask crate is currently outside the active Cargo workspace` no longer present in `contracts.rs` / `benchmark_policy.rs` / `forbidden_scan.rs` (0 matches each) | n/a | Stale comments removed; only references that remain are legitimate `cargo xtask <cmd>` strings | PATCHED | n/a (grep-only) |
| vb-t5zlm | P0  | vb_ipc | `cargo test -p vb_ipc --lib decode_frame_roundtrip_preserves_cancel_run_command` | 0 | 1/1 pass — but the targeted fix is **not in source** | NOT-PATCHED | /tmp/qa-vb-t5zlm-2.log |
| vb-t5zlm | P0  | vb_runtime | `cargo test -p vb_runtime --lib runtime_cancel_run_routes_to_correct_shard` | 0 | 1/1 pass — but `Runtime::cancel_run_with_reason` does not exist | NOT-PATCHED | /tmp/qa-vb-t5zlm-1.log |
| vb-tqz3v | P1  | vb_storage | `cargo test -p vb_storage --lib batch::tests::batch_put_run_header_commits_and_is_readable batch::tests::batch_put_snapshot_commits_and_is_readable` (broad: `cargo test -p vb_storage --lib batch::` -> 72/72) | 0 | 30 batch::tests + 42 byte_accounting pass — but `put_run_header` / `put_snapshot` still propagate encode error via `?` without setting `self.aborted = true` | NOT-PATCHED | /tmp/qa-vb-tqz3v-1.log, /tmp/qa-vb-tqz3v-broad.log |

## Bugs-checked: 4
- PASS / PATCHED: **2** (vb-rvgjy, vb-sqcig)
- NOT-PATCHED: **2** (vb-t5zlm, vb-tqz3v)
- PARTIAL: 0
- UNKNOWN: 0

## Test Regressions Detected
**None.** All 56 target tests in `velvet-ballistics-workspace-tests` (19+21+16) and all 72 `vb_storage::batch::*` tests pass. The pre-existing cancel_run IPC tests (`decode_frame_roundtrip_preserves_cancel_run_command`, `runtime_cancel_run_routes_to_correct_shard`) still pass — but only because they exercise the **unchanged** behavior. No fix was applied for the new contract.

## Top NOT-PATCHED with exit-code + last-error-line

### 1. vb-t5zlm — IPC `cancel_run` reason routing
- **exit-code:** 0 (test passes — but for the wrong reason; no regression test for the new contract exists)
- **evidence (last-error-line-equivalent — grep result):**
  - `crates/vb_ipc/src/payloads.rs:29-32` — `CancelRun { run_id: RunId }` has **no** `reason: Option<String>` field
  - `crates/vb_ipc/src/server/handlers.rs:117-123` — signature still `fn handle_cancel_run(payload: &[u8], runtime: &mut Runtime)`; destructures only `{ run_id }`; calls `runtime.cancel_run(run_id)` (NOT `cancel_run_with_reason`)
  - `rg cancel_run_with_reason` across `/home/lewis/src/velvet-ballistics/crates` returns **zero matches** — the runtime-side method does not exist
- **verdict:** NOT-PATCHED

### 2. vb-tqz3v — `put_run_header` / `put_snapshot` batch-abort on encode failure
- **exit-code:** 0 (all 30 happy-path batch::tests + 42 byte_accounting tests pass — but no negative-path test was added)
- **evidence (grep result):**
  - `crates/vb_storage/src/batch.rs:123-134` — `put_run_header` uses `let key = run_header_key(...)?;` and `let value = encode_record(...)?;` — both `?`-propagate without `self.aborted = true`
  - `crates/vb_storage/src/batch.rs:137-148` — `put_snapshot` identical pattern; no abort on encode failure
  - Compare sibling `put_blob` (lines 153-174) which **does** `self.aborted = true` on each error arm — the fix was never mirrored
  - No test named `*encode_failure*` or `*abort_on_encode*` exists in `crates/vb_storage/src/batch/tests.rs`
- **verdict:** NOT-PATCHED

## Additional Observations

- **vb-rvgjy** matches its bead-close notes exactly: 19/19 + 21/21 + 16/16 = 56/56 — the three-bug fix (legacy_slot_taint value-typing, no_output slot fabrication skip, snapshot+tail contiguity relaxation) is fully landed and exercised.
- **vb-sqcig** is a comment-only fix; the workspace `Cargo.toml` line 21 still has `xtask` commented out, so the bead's premise "xtask IS in workspace" is technically wrong, but the desired outcome (removing stale prose) was achieved via updating the comments in `xtask/src/{contracts.rs,benchmark_policy.rs,forbidden_scan.rs}` rather than re-admitting the member.
- **vb-t5zlm** and **vb-tqz3v** are both marked CLOSED in beads but show zero source-level evidence of the described fix. These appear to be documentation-only closures; the production-path defects they target are still reproducible in the current tree.

## Files Written
- `/home/lewis/src/velvet-ballistics/to-fix/wave5/agent-10-hands-on-qa.md` (this report)
- `/tmp/qa-vb-rvgjy-1.log`, `/tmp/qa-vb-rvgjy-2.log`, `/tmp/qa-vb-rvgjy-3.log`, `/tmp/qa-vb-rvgjy-broad.log`
- `/tmp/qa-vb-t5zlm-1.log`, `/tmp/qa-vb-t5zlm-2.log`
- `/tmp/qa-vb-tqz3v-1.log`, `/tmp/qa-tqz3v-broad.log`
