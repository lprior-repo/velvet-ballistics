# Test Suite Review — vb-vt2f State 9 trace-eviction stale ask BDD

STATUS: APPROVED

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 113-221 define suite inquisition; lines 127-173 require banned-pattern, determinism, mock, private-import, and exact error scans; lines 265-278 define rejection thresholds.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content observed; agents copy wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-20 require traceable exact behavior; lines 32-48 allow bounded helpers/loops when exact; lines 114-133 reject swallowed errors; lines 195-210 require compile/execute evidence.

## VERDICT: APPROVED

Scope: bead `vb-vt2f` only, State 9 sublane `test-review-trace-eviction-stale-ask-after-black-hat`. This is a RED-regression review after State 8; failing execution is expected and routes to implementation repair, not test rejection.

## Tier 0 — Static / Evidence Review

- [PASS] Banned weak assertion / silent discard / ignored / sleep / nondeterministic global / mock / private-surface scan over `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`: no hits.
- [PASS] Public direct API evidence is adequate: imports at lines 17-21 use public `vb_runtime` surfaces; stale regression uses `Runtime::new`, `snapshot_run`, `list_events`, and `answer_ask` at lines 724, 734-743, and 755-762.
- [PASS] Exact typed assertion: line 762 asserts `assert_eq!(answer_result, Err(RuntimeError::RunNotFound));`.
- [PASS] Terminal trace eviction precondition is executable, not comment-only: lines 741-744 assert retained stale trace is exactly `vec![TraceEvent::RunSubmitted { run: stale }]`.
- [PASS] Unrelated-run non-mutation is covered at lines 745-773 by snapshot, per-run trace, active-list, and counters equality.

## Tier 1 — Scoped RED Execution

Command, workdir `/home/lewis/src/bd-vb-vt2f-bdd`:

```bash
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballastics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted
```

Observed evidence:

```text
Nextest run ID 18ef6efb-66ee-4451-a9d1-a0db10e8e2d8
Summary [   0.003s] 1 test run: 0 passed, 1 failed, 13 skipped
FAIL test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted
crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:762:5
assertion `left == right` failed
  left: Ok(())
 right: Err(RunNotFound)
```

Interpretation: the new BDD is correctly RED against the known defective implementation and precisely exposes `LETHAL-001`.

## LETHAL FINDINGS

- None in the test design/suite evidence for this sublane.

## MAJOR FINDINGS (0)

- None.

## MINOR FINDINGS (0/5 threshold)

- None.

## MANDATE

- Route to State 10 implementation repair. Preserve the exact immediate `Err(RuntimeError::RunNotFound)` oracle and unrelated-run non-mutation assertions. Do not weaken this BDD to match the current `Ok(())` behavior.

STATUS: APPROVED
