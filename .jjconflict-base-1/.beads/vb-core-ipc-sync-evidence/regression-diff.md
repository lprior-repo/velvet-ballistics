# Regression Diff: vb-core-ipc-sync-evidence

STATUS: PASS

updated_at: 2026-05-17T03:49:00Z

## Changed Files

- `crates/vb_ipc/src/server/impl_tests.rs`: test-only slow-client oracle additions.
- `crates/vb_runtime/src/ipc_refinement.rs`: new pure production-binding refinement summaries and tests.
- `crates/vb_runtime/src/lib.rs`: module export.
- `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`: compile repair for Loom model.
- `crates/vb_runtime/src/models/loom/shutdown_drain.rs`: compile repair for Loom model.
- `verification/tla/IpcSyncEvidence.*`: liveness/fairness/deadlock repairs.
- `verification/verus/*.rs`: proof artifacts carried in workspace.
- `.beads/vb-core-ipc-sync-evidence/*`: evidence artifacts.

## Regression Evidence

- `moon ci --base HEAD --head HEAD --force`; exit 0; `20 tasks completed`.
- `moon run test`; exit 0; `8365 tests run: 8365 passed, 6 skipped`.
- Targeted IPC/runtime/proof commands passed as listed in `machine-gate-report.md` and `formal-verification-report.md`.

## Blocker Classification

- BLOCK_LOCAL: none remaining.
- BLOCK_REGRESSION: none observed.
- BLOCK_RELEASE: none observed.
- REQUIRED_OBLIGATION_FAIL: none observed.
- DEFERRED_GLOBAL: none for the user-named local blockers.
