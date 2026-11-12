# Moon Report — vb-qi37.16.2 rerun

STATUS: PASS

## Commands

- `rtk cargo fmt --check` — initially failed due formatting; repaired with `rtk cargo fmt`.
- `rtk cargo test --package vb_runtime --lib resume_keeps_awaiting_action_resumable_after_resume` — PASS, 1 passed.
- `rtk cargo test --package vb_runtime --test durable_resume_red_phase` — initially failed 2 stale AlreadyRunning expectations; repaired; final PASS, 17 passed.
- `rtk cargo test --package vb_runtime --lib` — PASS, 1368 passed.
- `moon run :quick` — PASS.
- `moon run :test` — PASS, 8015 passed.
- `moon ci` — PASS, 19 tasks completed, 2 cached.

## Classification

No State 8 blocker remains after repair. Current blocker is State 12 formal verifier, not CI.
