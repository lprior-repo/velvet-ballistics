# Test Suite Review Rerun: vb-qi37.3

STATUS: APPROVED

## Doctrine / context read

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: Mode 2 Suite Inquisition requires static scans, exact assertions, deterministic execution, error-variant completeness, and real command evidence (`lines 113-180`, `190-220`, `329-337`).
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same doctrine; agents copy wins if conflict appears.
- Read `/home/lewis/.claude/skills/test-reviewer/references/holzmann-test-rules.md`: loops/tables/helpers/local mutability are allowed when they keep exact assertions; reject skipped assertions, nondeterminism, swallowed errors, or weak evidence (`lines 1-6`, `13-49`, `114-155`, `195-210`).
- Read required bead artifacts: `STATE.md`, `contract.md`, `test-plan.md`, `test-plan-review.md`, `implementation.md`, `black-hat-review.md`, `defects.md`, `test-repair-blackhat.md`, `qa-report.md`, `qa-review.md`, `moon-report.md`, and `regression-diff.md`.

## Commands run and observed summaries

1. Focused black-hat repair tests:
   ```bash
   rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
   ```
   Observed: `Nextest run ID 6381b252-001a-4c94-ba13-d2dc05b9bd44`; `3 tests run: 3 passed, 1356 skipped`.

2. Broad collect regression suite:
   ```bash
   rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
   ```
   Observed: `Nextest run ID 82244ab8-a18c-4747-b81d-003f8ab57cb0`; `102 tests run: 102 passed, 1257 skipped`.

3. Focus-file static scan for weak assertions, ignored/sleeping tests, shared mutable globals, and mocks:
   ```bash
   rtk grep -n 'assert!\(result\.is_ok\(\)\)|assert!\(result\.is_err\(\)\)|#\[ignore\]|thread::sleep|tokio::time::sleep|sleep|static mut|lazy_static!|once_cell.*Mutex|once_cell.*RwLock|mockall|Mock.*::new\(\)|\.expect_' 'crates/vb_runtime/src/collect_tests.rs' 'crates/vb_runtime/src/engine/types.rs' 'crates/vb_runtime/src/primitives/collect.rs' 'crates/vb_runtime/src/engine/drive.rs'
   ```
   Observed: `0 matches`.

4. Required repair-test existence scan:
   ```bash
   rtk grep -n 'fn (collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state|collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence|collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state|collect_next_stale_page_returns_order_violation_stale_and_preserves_state|collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state|collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state)' 'crates/vb_runtime/src/collect_tests.rs' 'crates/vb_runtime/src/engine/types.rs'
   ```
   Observed: six required tests found at `collect_tests.rs:3246`, `3334`, `3380`, `3534`, `3570`, and `engine/types.rs:1134`.

5. Production split sanity scan:
   ```bash
   python3 - <<'PY'
   ...function span scan for collect/drive repair functions...
   PY
   ```
   Observed helper split exists around `collect_start`, `collect_next`, and `drive_deterministic_full` with extracted helpers (`build_collect_start_plan`, `finish_collect_start_page`, `build_collect_next_plan`, `begin_drive_step`, `finish_drive_step`, `emit_slot_evidence`). Suite approval does not adjudicate black-hat style limits beyond confirming the repair surface exists and tests are green.

## Coverage of black-hat defects

- DEFECT-001 semantic duplicate/stale/out-of-order lineage: covered by exact page-order tests in `crates/vb_runtime/src/collect_tests.rs:3195`, `3246`, `3334`, and `3380`; focused repair test passed.
- DEFECT-002 capacity-one fail-closed evidence preservation: covered by `crates/vb_runtime/src/engine/types.rs:1134`; focused repair test passed and asserts exact `CollectEvidenceCapacityExceeded` plus preserved existing evidence.
- DEFECT-003 corrupt collect-bearing slot values fail closed: covered by `crates/vb_runtime/src/collect_tests.rs:3570`; focused repair test passed and asserts exact `CollectExtraHydrationFailed { kind: DecodeFailed, ... }` plus no state inserted.
- DEFECT-004 production cohesion split: implementation now has extracted helpers in `crates/vb_runtime/src/primitives/collect.rs` and `crates/vb_runtime/src/engine/drive.rs`; State 10 suite gate confirms no test regression. Black-hat will re-adjudicate style in State 11.

## Findings

- LETHAL: none.
- MAJOR: none.
- MINOR: none.
- Rejection findings with file:line: none.

## Deferred global debt decision

The known `jj diff --name-only | moon ci --stdin` FORMAT/CLIPPY/`vb_ui_model` failures remain `DEFERRED_GLOBAL` under follow-up bead `vb-bkgo`. Reviewed `moon-report.md` and `regression-diff.md`; the explicit failing files/crates are outside this bead's changed source/test files and reproduce on clean main, so they are not State 10 rejection grounds.

## Decision

State 10 can exit: YES.

The repaired suite proves the black-hat regressions with exact assertions and real green command evidence. Advance to State 11 red-queen/black-hat rerun.
