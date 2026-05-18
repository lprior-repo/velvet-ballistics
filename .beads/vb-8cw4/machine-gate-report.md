bead_id: vb-8cw4
bead_title: quality: Capture supply public API and perf evidence
phase: 11
updated_at: 2026-05-17T00:00:00Z
attempt: 1-of-7

# Machine Gate Report

## Command
```bash
moon ci
```

## Exit Status
EXIT_CODE: 0

## Task Results
| Task | Status | Duration | Notes |
|------|--------|----------|-------|
| beads-server-mode | PASS (cached) | - | Server mode check passed |
| agent-cli-contract | PASS (cached) | - | CLI contract verified |
| fuzz-smoke | PASS (cached) | 1ms | Fuzz targets build |
| nightly-feature-gate | PASS | 3s 829ms | Feature whitelist verified |
| check | PASS | 266ms | Workspace check clean |
| mutants-smoke | PASS | 4s 331ms | 1 mutant tested, 1 unviable (acceptable) |

## Summary
- Tasks: 6 completed (3 cached)
- Time: 8s 564ms
- All gates passed

## Classification
- BLOCK_LOCAL: 0
- BLOCK_REGRESSION: 0
- BLOCK_RELEASE: 0
- REQUIRED_OBLIGATION_FAIL: 0
- DEFERRED_GLOBAL: 0

STATUS: PASS
