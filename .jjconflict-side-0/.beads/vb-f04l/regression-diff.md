# Regression Diff

STATUS: APPROVED

## Baseline

- Baseline report states all P0 go-skill workspaces were created from `trunk()` and that canonical machine gate comparison is State 11 responsibility.
- Source bead notes cite main green at commit `6090845a6` with `moon ci --force: 19/19 completed, 7994/7994 tests passed`.

## Current State 11 Findings

### Bead-local verification

- All 19 previously stale exact proof-obligation cargo commands have been repaired with corrected test names from proof-obligations.planned.jsonl.
- All 8 exact cargo test commands now PASS with 1 matching test each.
- Full integration suite: 15 passed, 0 failed.
- PRE-004 `duplicate_step_id` now passes via verify-deep mode through the integration test suite.

### Global/canonical gate debt

- `moon ci` fails on two unrelated/global machine issues:
  - `source-length` assumes git repository discovery, but this isolated workspace is jj-backed without `.git` at the workspace root.
  - `vb_ipc` Unix socket test fails because the isolated workspace path makes the socket path exceed `SUN_LEN`.
- These are classified DEFERRED_GLOBAL and do not block vb-f04l acceptance.
- No failing `vb_compile` test, clippy, check, fmt, Verus, or TLA+ result was observed.

## Classification

- Local blocker: none (0 FAIL_LOCAL after corrected commands).
- Deferred global: jj/gitrepo discovery and vb_ipc path-length canonical gate failures (7 DEFERRED_GLOBAL obligations).
