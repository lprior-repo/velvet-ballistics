# Baseline Report - vb-8mdp.2

## Workspace
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260525/vb-8mdp-2`
- source_head: `249e4911c8335a8a6424a51c7d87e7a40e531298`
- isolation_check: `pwd -P` matched isolated workspace and was outside source checkout

## State 1 Artifact Check
- `STATE.md`: present
- `baseline-report.md`: present (this file)
- `delivery-scope.jsonl`: pending (explore will produce)

## Baseline Status
- Fresh bead initialized in isolated workspace
- State 2 (explore) dispatched to map codebase
- Source checkout: `/home/lewis/src/velvet-ballistics`
- Crate target: to be determined by explore

## Source Scope
- Integration-level Fjall journal/snapshot read-path tests only
- After shared envelope codec contract in vb-3t44 exists, prove every storage read reserves/checks payload length before decode/allocation
- Returns exact typed budget errors
- Do not duplicate envelope fixed-wire tests from vb-3t44
