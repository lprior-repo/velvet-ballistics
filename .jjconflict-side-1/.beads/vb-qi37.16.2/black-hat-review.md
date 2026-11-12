# Black Hat Review — vb-qi37.16.2 State 11 rerun

STATUS: APPROVED

## Findings

- `handle_resume` no longer clobbers post-`drive_run` lifecycle state. AwaitingAction remains `RuntimeState::Resumable`; terminal outcomes remain owned by `apply_drive_result`.
- Regression test `resume_keeps_awaiting_action_resumable_after_resume` proves the previous clobber is dead.
- `ResumeStatus::Resumed` docs and durable-resume tests now match implementation: resume means accepted and driven once, not necessarily finally Running.
- `apply_drive_result` Awaiting* branches are extracted to owner helpers.
- `is_run_tracked` naming/comment aligns the current hydration boundary; full semantic journal hydration remains a known contract/test-double limitation, not silently claimed.
- `specs/ResumeStateMachine.tla` and `.cfg` now exist and TLC passes bounded safety checks.

## Evidence

- `rtk cargo test --package vb_runtime --test durable_resume_red_phase` — PASS, 17 passed.
- `rtk cargo test --package vb_runtime --lib` — PASS, 1368 passed.
- `moon run :quick` — PASS.
- `moon run :test` — PASS, 8015 passed.
- `moon ci` — PASS, 19 tasks completed.
- `tlc -config specs/ResumeStateMachine.cfg specs/ResumeStateMachine.tla` — PASS, 850 generated / 313 distinct / depth 13 / no errors.

## Decision

State 11 approved. Advance to State 12 formal verifier.
