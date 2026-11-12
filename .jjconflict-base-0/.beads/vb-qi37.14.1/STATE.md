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
- **Current State**: 14 (LANDING)
- **State 1-13 Completed**: 2026-05-18

## Evidence Status
- final-evidence-decision.md: **STATUS: APPROVED**
- truth-serum-report.md: **CLEAN** (no evidence laundering)
- assurance-bundle.md: maps all requirements to evidence

## Pipeline Tracking
| State | Description | Status | Notes |
|-------|-------------|--------|-------|
| 1-13 | | COMPLETE | |
| 14 | LANDING | IN_PROGRESS | Subagent: landing-skill |
| 15 | CLEANUP | PENDING | Orchestrator verification |

## Notes
- Bead depends on: vb-qi37.13.4 (cli: Structured output contract tests) - COMPLETE
- Blocks: vb-qi37.14 (cli: Prove explain, diff, graph, and run-step contracts)
- DEFERRED_GLOBAL: 10 vb-qi37.13 tests (pre-existing exit code mismatch)
