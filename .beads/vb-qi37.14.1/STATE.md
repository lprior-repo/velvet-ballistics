# Bead State - vb-qi37.14.1

## Bead Info
- **Bead ID**: vb-qi37.14.1
- **Title**: cli: Add single-step run command
- **Priority**: P1
- **Type**: feature
- **Owner**: Lewis
- **Assignee**: Lewis

## Workspace Isolation
- **Source Checkout**: /home/lewis/src/velvet-ballistics
- **Isolated Workspace**: /home/lewis/src/vb-qi37-14-1
- **JJ Workspace Name**: go-skill-vb-qi37-14-1

## State Machine
- **Current State**: 10 (IMPLEMENT)
- **State 1-9 Completed**: 2026-05-18 (test-plan + test-suite: STATUS APPROVED)

## Implementation Fixes Required
1. `code` → `error` in JSON error output
2. Write JSON to stdout (not stderr) when `--json` flag
3. Remap exit codes: 1=RuntimeFailed, 2=ValidationFailed
4. Enhance cmd_run_step() for structured output: pc/slot/taint/state deltas
5. Add JSON/JSONL variant of print_step_result()

## Pipeline Tracking
| State | Description | Status | Notes |
|-------|-------------|--------|-------|
| 1-9 | | COMPLETE | |
| 10 | IMPLEMENT | COMPLETE | Implementation compiles, clippy clean, 10,962 tests pass |
| 11 | EXECUTE | COMPLETE | 10962 tests pass, 55 Verus verified, Kani BLOCKED_TOOLING waived |
| 12 | ATTACK | PASS | black-hat-review.md: STATUS PASS |
| 13 | EVIDENCE/LAND | APPROVED | final-evidence-decision.md: STATUS APPROVED |
| 14 | LANDING | IN_PROGRESS | landing-skill |
| 15 | CLEANUP | PENDING | Orchestrator verification |

## Notes
- Bead depends on: vb-qi37.13.4 (cli: Structured output contract tests) - COMPLETE
- Blocks: vb-qi37.14 (cli: Prove explain, diff, graph, and run-step contracts)
