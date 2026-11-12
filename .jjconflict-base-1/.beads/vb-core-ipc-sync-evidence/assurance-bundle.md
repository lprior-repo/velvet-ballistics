# Assurance Bundle: vb-core-ipc-sync-evidence

STATUS: APPROVED

updated_at: 2026-05-17T03:51:00Z
state: 13

## Requirement Evidence

- REQ-IPC-001 strict admission: Verus `5 verified`; REFINE-IPC-001 production binding test passed.
- REQ-IPC-002 capacity bounds: Verus `6 verified`; Loom bounded queue `2 passed`; REFINE-IPC-002 production binding test passed.
- REQ-IPC-003 terminal race: Verus runtime transitions `7 verified`; Loom cancel/completion `2 passed`; REFINE-IPC-003 production binding test passed.
- REQ-IPC-004 timer race: Verus runtime transitions `7 verified`; Loom timer/cancel `1 passed`; REFINE-IPC-004 production binding test passed.
- REQ-IPC-005 shutdown drain: Verus runtime transitions `7 verified`; Loom shutdown/drain `3 passed`; REFINE-IPC-005 production binding test passed.
- REQ-IPC-006 slow client: `rtk cargo test -p vb_ipc slow_client`; `2 passed`.
- REQ-IPC-ALL gate: `moon ci --base HEAD --head HEAD --force`; `20 tasks completed`.

## Formal Evidence

- Main TLA model: `28060 states generated, 5136 distinct states found, 0 states left on queue`; temporal branches checked; no error.
- Capacity-1 TLA model: `15781 states generated, 2997 distinct states found, 0 states left on queue`; temporal branches checked; no error.
- Verus total: 18 verified, 0 errors.

## Machine Evidence

- `moon run lint-src`; pass.
- `moon run test`; `8365 passed, 6 skipped`.
- `moon ci --base HEAD --head HEAD --force`; pass.

## Decision

STATUS: APPROVED
