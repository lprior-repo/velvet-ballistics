# Handoff — vb-2lyiw six-file decomposition repair

Author: DeepSeek v4Pro (OMP)
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-2lyiw-deepseek-repair`
Git root == JJ root == that workspace (verified).
Root owns ALL VCS/JJ mutations and final canonical `moon ci`. **Bead left OPEN; nothing committed,
nothing pushed.**

Authoritative source: `5766dff91`. Parent checkpoint `29f36f3ad` was local5090's non-compiling
partial; superseded here.

## What was repaired

All six >300-line production files are now split below 300 lines, faithful to `5766` (moved bodies
byte-identical except necessary module qualification/visibility), no invented logic, no dropped
functions, no new `include!`, no lint allows, no exception-ledger rows.

| File | Before | After | Split into |
|------|-------:|------:|------------|
| `vb_compile/.../part_05_digest.rs` | 500 | 294 | `part_05_digest_primitive.rs` (122), `part_05_digest_validate.rs` (96) |
| `vb_runtime/.../dispatch.rs` | 301 | 186 | `dispatch/compound.rs` (138) |
| `vb_runtime/.../recovery/full.rs` | 363 | 285 | `recovery/full/full_ask_schedule.rs` (82) |
| `vb_runtime/.../run_checkpoint.rs` | 388 | 241 | `run_checkpoint_tests.rs` (140) |
| `vb_runtime/.../chunk_005.rs` | 352 | 250 | `chunk_005_prepare.rs` (26), `chunk_005_drive_apply.rs` (77) |
| `vb_runtime/.../transitions.rs` | 343 | 213 | `transitions_cleanup.rs` (129) |

Specific fixes:
1. **part_05_digest**: child `mod` decls needed `#[path = "..."]` (sibling files, not a
   `part_05_digest/` subdir). Reverted local5090's unnecessary `pub(crate)` on parent helpers back
   to private (private parent items are visible to child modules; only the three re-exported
   functions need `pub(crate)`). Child references fixed `crate::mod_compile_lowering::part_05_digest::`
   → `super::`.
2. **dispatch**: genuine split — moved `dispatch_collect`/`dispatch_reduce`/`dispatch_repeat` into
   `dispatch/compound.rs` (reusing the `super::family_invariant` pattern from `action_signal.rs`);
   restored the removed blank line; `action_signal.rs` restored byte-identical to `5766`.
3. **full**: `full_ask_schedule.rs` was an invented 30-line bogus body in the checkpoint; rewrote it
   to the exact original `AskScheduleMutation` fold logic. Simplified the `use` to
   `use full_ask_schedule::pending_ask_schedule_for_step;`.
4. **run_checkpoint**: test module extraction was already faithful; only rustfmt cleanup.
5. **chunk_005**: removed the nested `include!("chunk_005_prepare.rs")` line-compaction trick;
   recreated the vanished `chunk_005_drive_apply.rs` (`drive_state`/`apply_drive_result`/
   `apply_drive_continue`); all lifecycle methods present exactly once; lifecycle.rs includes the two
   new files before `chunk_005.rs`.
6. **transitions**: `transitions_cleanup.rs` bodies were already faithful; removed now-unused imports
   (`StepIdx` from transitions.rs, `PendingTimer`/`PendingTimerKind` from transitions_cleanup.rs).

## durable_retry_run_attempt_authority.rs

The three ignored `tick_all().ok()` Results are now typed:
- **setup tick** and **current-completion tick**: propagate success (`map_err(...)?`).
- **queued stale-failure tick** (after `fail_action(old_ticket)`): asserts the exact
  `InvalidActionCompletion` rejection (from `require_pending_action_ownership`; the run-attempt
  mismatch falls through the per-action stale arm). No longer a blind `.ok()` or blind `?`.
- `assert_ne!(old.idempotency_key, new.idempotency_key)` preserved.

## Gates (all through `/tmp/vb-run-gate.py`, true JSON exit codes + raw logs in `.beads/vb-2lyiw/evidence/`)

| Gate | Command | Result |
|------|---------|--------|
| libcheck2 | `cargo check -p vb_compile -p vb_runtime --lib` | **0** |
| clippy-strict2 | canonical `-D` flag list (below), `RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings`, `--lib` | **0** |
| srclen2 | `bash scripts/check-source-length.sh` | **0**, over_limit=0 all categories |
| testcompile2 | `cargo test -p vb_compile --features test-util --no-run` | **0** |
| testcompile-runtime2 | `cargo test -p vb_runtime --no-run` | **0** |
| digest-unit2 | `cargo test -p vb_compile --features test-util --lib -- digest` | **0** — 123 passed |
| digest-integration | 5 digest `--test` targets | **0** — 42 passed |
| run-checkpoint | `cargo test -p vb_runtime --lib -- run_checkpoint` | **0** — 2 passed |
| recovery-unit2 | `cargo test -p vb_runtime --lib -- recovery` | **0** — 37 passed |
| shard-tests2 | `cargo test -p vb_runtime --lib -- shard` | **0** — 683 passed |
| authority4-rerun | `cargo test -p vb_runtime --test durable_retry_run_attempt_authority` | **0** — 4 passed |
| replay3 | `cargo test -p vb_runtime --test vb_r4fm9_event_generation` | **0** — 3 passed |
| bcd10 | `cargo test -p vb_runtime --test durable_retry_boundary` | **0** — 10 passed |
| independent9 | `cargo test -p vb_runtime --test vb_4qog3_independent_review` | **0** — 9 passed |
| retry2 | `cargo test -p vb_runtime --test durable_retry_execution` | **0** — 2 passed |
| cli3 | `cargo test -p velvet-ballistics --test durable_retry_cli` | **0** — 3 passed |
| pending20 | `cargo test -p velvet-ballistics-workspace-tests --test runtime_fjall_pending_action_recovery` | **0** — 20 passed |

Canonical strict-clippy flag list used (`.moon/tasks/all.yml`): `-D warnings -W clippy::all
-D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic
-D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro
-D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap
-D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use
-D clippy::await_holding_lock -D clippy::print_stdout -D clippy::print_stderr`.
Note: scoped to `-p vb_compile -p vb_runtime --lib` (not full `--workspace --all-features`, which is
root's final canonical execution and would pull the `#[cfg(kani)]` verification lanes).

## Pre-existing findings (not this task; do not waive)

- `cargo test -p vb_compile -p vb_runtime --no-run` (no features) fails `E0432 unresolved import
  vb_compile::WorkflowSourceParts` — pre-existing: `lib.rs` gates it behind
  `#[cfg(any(test, feature = "test-util"))]`, but integration tests import it ungated. Passing the
  gate requires `--features test-util` (vb_compile's only feature) or `--all-features`. No production
  export changed.
- Orphan tracked-but-unreferenced `chunk_002_drive_apply.rs` (251 lines, duplicate method bodies) is
  pre-existing dead code in `5766`; left untouched.

## Out of scope (per task, not touched)

- vb-ga13x Kani fixture fields (local5090); vb-bbf56 mirror drift; local3090 journal-byte /
  mutation probes.

## Next (root)

1. Review the working-tree diff (uncommitted).
2. Freeze a reviewed candidate and run canonical `moon ci` (full `--all-features` clippy + fmt).
3. Merge + close bead after canonical gates pass.
