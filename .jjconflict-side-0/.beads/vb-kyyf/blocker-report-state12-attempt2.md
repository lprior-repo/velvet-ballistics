# vb-kyyf blocker report — State 12 attempt 2

STATUS: BLOCKED

## Blocker

State 12 black-hat attempt 2 rejected `vb-kyyf` after State 8/9/10/11 repairs.

Primary defect:

- `BDD-KYYF-002` still does not prove real dropped/reopened CLI `replay/events/inspect` reproducibility.
- Evidence shows durable storage has 4 events, but CLI reports `storage is held by an active writer`, exits success, and emits `events=0`.
- This is a bead-local public-surface contract failure, not unrelated global debt.

## Required route from reviewer

`black-hat-review.md` and `defects.md` route to:

1. State 8 test-writer for BDD-KYYF-002 assertion hardening.
2. State 10 CLI/storage implementation repair if the hardened test exposes behavior gaps.
3. State 11 formal verification rerun.
4. State 12 black-hat rerun.

## Retry/cap conflict

- State 8 test-writer has already reached attempt 7/7.
- A normal State 8 attempt 8 would violate go-skill retry policy.

## Owner decision required

Choose one:

1. Authorize an explicit cap-unblock lane for State 8 BDD-KYYF-002 assertion hardening.
2. Authorize a nonstandard direct State 10 repair without State 8 hardening, accepting reviewer-route deviation.
3. Provide an explicit waiver for the CLI public-surface evidence requirement.

No landing is allowed until this blocker is resolved.
