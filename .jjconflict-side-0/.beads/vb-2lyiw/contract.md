# Rust Contract: vb-2lyiw Six-File Decomposition Repair

**Bead:** vb-2lyiw
**Type:** Bounded source-decomposition repair (faithful + compilable)
**Date:** 2026-10-21
**Authoritative source:** `5766dff91` (frozen good). Parent checkpoint `29f36f3ad` is
local5090's incomplete, non-compiling decomposition; not accepted source.

## Goal

Complete the six-file >300-line split against the frozen good source, preserving
exact production behavior. Every moved body matches `5766` except necessary
module qualification/visibility. No invented business logic, no dropped functions,
no line-compaction tricks, no `include!` for the new split, no lint allows, no
exception-ledger rows for these six files.

## Scope (six production files, all now <=300 lines)

1. `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs` (500)
   - Split into `part_05_digest_primitive.rs` + `part_05_digest_validate.rs`.
   - Fix: child `mod` decls need `#[path]`; child `super::` qualification.
2. `crates/vb_runtime/src/engine/execute/dispatch.rs` (301)
   - Genuine split of compound-family dispatchers into `dispatch/compound.rs`.
3. `crates/vb_runtime/src/recovery/full.rs` (363)
   - Extract `pending_ask_schedule_for_step` + AskScheduleMutation helpers into
     `recovery/full/full_ask_schedule.rs`, byte-for-byte original logic.
4. `crates/vb_runtime/src/run_checkpoint.rs` (388)
   - Move `#[cfg(test)]` module to `run_checkpoint_tests.rs` (test_in_src).
5. `crates/vb_runtime/src/shard/lifecycle/chunk_005.rs` (352)
   - Extract `schedule_action_abi_digest`/`awaiting_schedule_event` into
     `chunk_005_prepare.rs`; extract `drive_state`/`apply_drive_result`/
     `apply_drive_continue` into `chunk_005_drive_apply.rs`. Remove the nested
     `include!` (line-compaction). All lifecycle methods present exactly once.
6. `crates/vb_runtime/src/shard/transitions.rs` (343)
   - Extract terminal/cleanup + retry-checkpoint methods into
     `transitions_cleanup.rs`. Fix now-unused imports in both files.

## Extra: `durable_retry_run_attempt_authority.rs`

Repair the three ignored tick `Result`s:
- setup tick and current-completion tick: propagate success.
- queued stale-failure tick: assert exact `InvalidActionCompletion` rejection
  (from `require_pending_action_ownership`), not a blind `.ok()` or blind
  `?` propagation.
- Preserve the `assert_ne!(old.idempotency_key, new.idempotency_key)`.

## Out of scope

- vb-ga13x Kani fixture fields (local5090), vb-bbf56 mirror drift,
  local3090 journal-byte / mutation probes — do not touch or self-approve.
- No `moon ci` (root owns final canonical execution on frozen candidate).

## Gates (immutable `python3 /tmp/vb-run-gate.py SECONDS .beads/vb-2lyiw/evidence/NAME COMMAND ARGS`)

Focused lib check (vb_compile + vb_runtime), source-length, strict production
clippy, strict all-test compile, focused regressions (authority4/replay3/BCD10/
independent9/retry2/CLI3/Fjall-pending20) plus compiler primitive/digest tests.
Check actual JSON exit + raw logs. Bead stays OPEN.
