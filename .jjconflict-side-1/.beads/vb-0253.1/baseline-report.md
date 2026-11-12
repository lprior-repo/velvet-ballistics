# Baseline Report - vb-0253.1

## Workspace
- source_checkout: `/home/lewis/src/velvet-ballistics`
- isolated_workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-0253-1`
- source_head: `39df7f43ad59e15898c2aa773d34be781d6754e1`
- isolation_check: `pwd -P` matched isolated workspace and was outside source checkout

## State 1-4 Artifact Check
- `STATE.md`: present
- `codebase-map.md`: present
- `delivery-scope.jsonl`: present and `jq -c` valid
- `contract.md`: present
- `proof-obligations.jsonl`: present and `jq -c` valid
- `traceability-matrix.jsonl`: present and `jq -c` valid
- `verification-layers.md`: present
- `proof-strategy.md`: present
- `proof-plan-review-input.md`: present
- `proof-obligations.planned.jsonl`: present

## Baseline Status
- Baseline reconstructed in the exact requested workspace because the prior sanitized workspace `.beads/vb-0253.1/STATE.md` marked `baseline-report.md` as pending.
- Initial worktree status before State 5: only `.beads/vb-0253.1/` untracked after restoring prior artifacts.
