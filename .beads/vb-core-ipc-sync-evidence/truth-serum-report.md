# Truth Serum Report: vb-core-ipc-sync-evidence

STATUS: APPROVED

reviewed_at: 2026-05-17T03:52:00Z
state: 13

## Hallucination Checks

- Claim: slow-client oracle exists. Evidence: `rtk cargo test -p vb_ipc slow_client`; `2 passed, 407 filtered out`.
- Claim: production REFINE bindings exist. Evidence: `crates/vb_runtime/src/ipc_refinement.rs`; `rtk cargo test -p vb_runtime ipc_refinement`; `5 passed`.
- Claim: TLA liveness/deadlock no longer deferred. Evidence: both configs have `PROPERTY` lines and `CHECK_DEADLOCK TRUE`; TLC checked temporal branches and found no error.
- Claim: machine gate passed. Evidence: `moon ci --base HEAD --head HEAD --force`; `Tasks: 20 completed`.

## Rejections Considered

- Default `moon ci` failure is real and recorded. It fails before task execution due missing Git `main` in this jj workspace. The equivalent forced explicit-revision CI graph passed.
- No `Ok(_)`, `x == x`, ignored test selection, or zero-test filter was used as acceptance evidence.

## Ruling

STATUS: APPROVED
