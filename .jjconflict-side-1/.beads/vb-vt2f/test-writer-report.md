# Test Writer Report — vb-vt2f State 8 trace-eviction stale ask repair

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-writer/SKILL.md`: lines 21-30 require public behavior tests with exact assertions; lines 158-163 reject weak assertions; lines 313-360 define verification gates.
- Read `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content; no conflict observed, and the agents copy wins if conflict exists.

## Scope

- Bead: `vb-vt2f` only.
- State/sublane: `State 8 test-repair-stale-ask-trace-eviction-after-black-hat`, attempt 3.
- Contract target: `.beads/vb-vt2f/contract.md` ERR-004 lines 64-65 requires stale/mismatched ask tickets to return exact typed error and avoid unrelated-run mutation.
- Black-hat route: `.beads/vb-vt2f/black-hat-review.md` lines 13-20 rejects trace-retention-dependent stale detection; lines 37-39 require an executable BDD where terminal trace evidence is unavailable/dropped/evicted.

## Files Touched

- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`
- `.beads/vb-vt2f/test-writer-report.md`

## BDD Regression Added

- Added `trace_starved_config()` with `trace_capacity: 1` to force bounded trace evidence loss while preserving public direct-runtime APIs.
- Added `test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted`.
- Given: a finished stale run whose retained trace is only `RunSubmitted`, proving terminal `RunFinished` evidence is unavailable, plus one unrelated suspended run.
- When: `Runtime::answer_ask(...)` is called with the stale ask ticket.
- Then: exact expected result is `Err(RuntimeError::RunNotFound)` immediately, and unrelated snapshot, trace events, active-list, and counters remain unchanged.
- No production implementation changed.

## Command Evidence

Workdir: `/home/lewis/src/bd-vb-vt2f-bdd`.

```bash
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
```

Status: PASS, no output.

```bash
TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted
```

Status: RED, expected failing-first evidence captured.

```text
Nextest run ID 1356b2f3-1901-4ea9-9629-0ea161a143e4
Summary [   0.003s] 1 test run: 0 passed, 1 failed, 13 skipped
FAIL velvet-ballistics-workspace-tests::vb_vt2f_direct_runtime_api_acceptance test_direct_api_answer_ask_rejects_stale_ticket_when_terminal_trace_was_evicted
assertion `left == right` failed
  left: Ok(())
 right: Err(RunNotFound)
```

## Classification

- `RED_FAILING_FIRST_EVIDENCE_CAPTURED`.
- The new public direct-API BDD exposes the State 12 black-hat defect: concrete `Runtime::answer_ask(...)` returns `Ok(())` for a stale terminal run when terminal trace evidence has been dropped/evicted from the bounded trace ring.

## Next Route

- Route to implementation repair for `ERR-004`: make stale terminal/non-active ask rejection independent of lossy trace retention, then rerun this direct API target plus affected formal/ledger gates required by the controller.
