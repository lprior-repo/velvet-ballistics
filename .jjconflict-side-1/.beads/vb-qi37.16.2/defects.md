# Defects — vb-qi37.16.2

STATUS: STATE_11_RESOLVED_STATE_12_BLOCKED

## Resolved

| Defect | Resolution |
|---|---|
| `handle_resume` clobbered post-drive state | Removed overwrite; `apply_drive_result` owns post-drive lifecycle state. |
| Missing AwaitingAction regression | Added `resume_keeps_awaiting_action_resumable_after_resume`. |
| Stale tests assumed second resume becomes AlreadyRunning | Repaired expectations to Resumed because action-awaiting workflow remains Resumable. |
| Missing ResumeStateMachine formal artifacts | Added `specs/ResumeStateMachine.tla` and `.cfg`; TLC passes. |

## Remaining blocker

State 12 formal verifier is blocked:

- `verus not found` for required Verus obligations.
- `rtk cargo test --package vb_storage --test replay_resume -- --nocapture` fails because `vb_storage` has no test target named `replay_resume`.

owner_state: 12
rerun_from: 12
