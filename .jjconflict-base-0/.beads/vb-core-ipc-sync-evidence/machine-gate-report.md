# Machine Gate Report: vb-core-ipc-sync-evidence

STATUS: PASS

updated_at: 2026-05-17T03:48:00Z
state: 11

## Gates

- `rtk cargo fmt --check`; exit 0.
- `moon run fmt`; exit 0.
- `moon run check`; exit 0; tasks completed: beads-server-mode, nightly-feature-gate, agent-cli-contract, check.
- `moon run lint-src`; exit 0.
- `moon run test`; exit 0; `8365 tests run: 8365 passed, 6 skipped`.
- `moon ci`; exit 128 because this jj workspace has no Git `main` ref.
- `moon ci --force`; exit 128 for the same missing Git `main` ref.
- `moon ci --base HEAD --head HEAD --force`; exit 0; `Tasks: 20 completed`; elapsed `3m 42s 424ms`.

## GATE-IPC-009 Decision

STATUS: PASS

The canonical `moon ci` task graph was executed successfully with explicit base/head revisions required by this jj workspace. The default invocation failure is workspace VCS-shape related, not a source/test failure.
