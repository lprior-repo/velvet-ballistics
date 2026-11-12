# Red Queen Report — vb-qi37.16.2 State 11 rerun

STATUS: PASS

## Adversarial result

- Regression target `resume_keeps_awaiting_action_resumable_after_resume` proves `handle_resume` does not overwrite `RuntimeState::Resumable` after an AwaitingAction drive.
- Durable resume red-phase suite passes after stale action-awaiting re-resume expectations were aligned to the repaired state machine.
- TLC model artifacts for `ResumeStateMachine` exist and pass bounded safety checks.

## Evidence

- `rtk cargo test --package vb_runtime --lib resume_keeps_awaiting_action_resumable_after_resume` — PASS, 1 passed.
- `rtk cargo test --package vb_runtime --test durable_resume_red_phase` — PASS, 17 passed.
- `tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla` — PASS, 850 generated / 313 distinct / depth 13 / no errors.
