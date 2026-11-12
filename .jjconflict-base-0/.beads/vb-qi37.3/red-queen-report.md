# Red Queen Report: vb-qi37.3 State 11 rerun

STATUS: PASS

## Doctrine read / precedence

- Read `/home/lewis/.claude/skills/red-queen/SKILL.md`: deterministic over AI; generated challengers are executed by shell; exit code is ground truth; survivors are deterministic failures (`lines 29-50`, `79-89`, `531-540`).
- Read `/home/lewis/.agents/skills/red-queen/SKILL.md`: same content observed; no conflict. If conflict existed, `/home/lewis/.agents/skills/red-queen/SKILL.md` would win.

## Context read

- Read bead artifacts requested for rerun: `STATE.md`, `test-suite-review.md`, `qa-report.md`, `qa-review.md`, `black-hat-review.md`, `defects.md`, `test-repair-blackhat.md`, `implementation.md`, `moon-report.md`, and `regression-diff.md`.
- State 9 QA and State 10 suite review are approved after black-hat repair.
- Known global FORMAT/CLIPPY/`vb_ui_model` debt remains `DEFERRED_GLOBAL` under `vb-bkgo`; no bead-local causality found in this Red Queen rerun.

## Adversarial commands and observed summaries

### 1. Semantic collect lineage: duplicate/stale/out-of-order, including duplicate with intervening allocations

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state) | test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_next_stale_page_returns_order_violation_stale_and_preserves_state) | test(collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state)'
```

Observed summary / exit code:

```text
Exit status: 0
Nextest run ID c0793f95-34ac-4910-bbdf-9a7caad42a50
Summary [   0.028s] 4 tests run: 4 passed, 1355 skipped
```

Survivor: none.

### 2. Capacity zero/one/full collect evidence fail-closed behavior

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_slot_extra_capacity_zero_returns_capacity_error_before_success) | test(collect_slot_extra_capacity_one_preserves_required_slot_written_extra) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop)'
```

Observed summary / exit code:

```text
Exit status: 0
Nextest run ID 7ada64bd-4323-4f23-8440-a8affaaf3a89
Summary [   0.032s] 3 tests run: 3 passed, 1356 skipped
```

Survivor: none. Note: the expression selected three concrete tests; no unmatched-selection failure occurred.

### 3. Collect hydration failure modes: corrupt collect-bearing slot value and current-page mismatch

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state) | test(collect_hydration_current_page_mismatch_returns_page_mismatch_and_no_state) | test(collect_hydration_corrupt_extra_returns_decode_failed_and_no_state) | test(recovered_collect_state_rejects_run_mismatch_and_inserts_no_state) | test(recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state)'
```

Observed summary / exit code:

```text
Exit status: 0
Nextest run ID 11dee37b-7b7a-460f-a29f-054b413601d7
Summary [   0.032s] 5 tests run: 5 passed, 1354 skipped
```

Survivor: none.

### 4. Broad `vb_runtime collect_` suite

Command:

```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_
```

Observed summary / exit code:

```text
Exit status: 0
Nextest run ID 3b3b5e9c-8b5e-4a40-bfb9-2843c9ae553c
Summary [   0.147s] 102 tests run: 102 passed, 1257 skipped
```

Survivor: none.

## Survivors / failures

- Critical: none.
- Major: none.
- Minor: none.
- Reproduction commands for survivors: none, because every adversarial challenger exited `0`.

## Decision

State 11 Red Queen can pass.

The repaired implementation defeated the required adversarial challengers for semantic page lineage, capacity fail-closed behavior, collect hydration failure modes, and the broad `vb_runtime collect_` regression suite. Crown defended for bead-local Red Queen scope.
