# Code Review — vb-t59ev six-file decomposition and Kani fixture repairs

Reviewer: LOCAL5090 independent code reviewer
Workspace: `/home/lewis/src/isoloated/velvet-ballistics-vb-t59ev-local5090-review`
Git root == JJ root == verified.

**STATUS: APPROVED**

## Scope reviewed

Source split candidate `bfb8bac3f676909425f6df3656ed62394e951fda` against authoritative good `5766dff917a95f0d56a6199871b93d17de5fd7be`, plus Kani field repairs in `d14ae7333`.

## Six decompositions — faithful

All six production files split below 300 lines with byte-identical function bodies, only necessary `super::` module qualification and visibility changes. No invented logic, no dropped functions, no new `include!`, no lint allows.

1. **part_05_digest (500→294):** `digest_step_primitive` moved to `part_05_digest_primitive.rs` (122), `digest_author_value`+`validate_branch_counts` moved to `part_05_digest_validate.rs` (96). `#[path = "..."]` sibling imports correct.
2. **dispatch (301→186):** `dispatch_collect`/`dispatch_reduce`/`dispatch_repeat` moved to `dispatch/compound.rs` (138). `super::family_invariant` imported correctly.
3. **recovery/full (363→285):** `pending_ask_schedule_for_step` and supporting `AskScheduleMutation` enum/handlers moved to `recovery/full/full_ask_schedule.rs` (82). Fold logic byte-identical.
4. **run_checkpoint (388→241):** Test module extracted to `run_checkpoint_tests.rs` (140). Both test functions identical.
5. **chunk_005 (352→250):** `drive_state`/`apply_drive_result`/`apply_drive_continue` moved to `chunk_005_drive_apply.rs` (77); `schedule_action_abi_digest`/`awaiting_schedule_event` moved to `chunk_005_prepare.rs` (26). All lifecycle methods present exactly once.
6. **transitions (343→213):** Five cleanup/retry methods moved to `transitions_cleanup.rs` (129). Bodies byte-identical.

## durable_retry_run_attempt_authority — properly typed tick outcomes

Three previously ignored `tick_all().ok()` calls now properly handled:
- Setup tick: `map_err(...)?` propagates failure
- Current-completion tick: `map_err(...)?` propagates failure
- Queued stale-failure tick: asserts exact `InvalidActionCompletion` rejection (not blind `.ok()` or blind `?`)

`assert_ne!(old.idempotency_key, new.idempotency_key)` preserved.

## Kani field repairs (d14ae7333) — correct two-line delta

Two fixture field repairs make Kani inputs match current production struct definitions:
- `kani_idempotency_gates.rs`: `attempt: kani::any()` added to `symbolic_ticket()`; `run_attempt` was already symbolic (not changed by d14)
- `replay/kani_harnesses.rs`: `input_slots: Box::new([])` added to `make_minimal_plan` `WorkflowParts`; `semantic_source_identity` was already present (not changed by d14)

## Bounded verification (reproduced)

- `cargo check -p vb_compile -p vb_runtime --lib`: exit 0
- `cargo test -p vb_compile --features test-util --lib -- digest`: 123 passed
- `cargo test -p vb_runtime --lib -- run_checkpoint`: 2 passed
- `cargo test -p vb_runtime --lib -- recovery`: 37 passed
- `cargo test -p vb_runtime --lib -- shard`: 683 passed
- `cargo test -p vb_runtime --test durable_retry_run_attempt_authority`: 4 passed

## Limits

- Full `moon ci` (all-features clippy/fmt) not run; root's responsibility
- Kani replay proof timed out (840s) as documented in evidence; not rerun
- Orphan `chunk_002_drive_apply.rs` pre-existing dead code untouched