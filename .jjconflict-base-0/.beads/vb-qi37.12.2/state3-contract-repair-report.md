# State 3 Contract Repair Report - vb-qi37.12.2

STATUS: CONTRACT_NARROWED

## Skill Files Cited

- `/home/lewis/.claude/skills/rust-contract/SKILL.md`: lines 8, 13-18, 22-25, 135-159, and 178-197 require contract-first artifacts, TLA+/Verus split, compact JSONL obligations with owner_state/rerun_from, no implementation, and valid JSONL.
- `/home/lewis/.agents/skills/rust-contract/SKILL.md`: same effective version 2.6.0; per startup rule this agents copy wins on conflict. No conflict observed.

## Decision

Narrowing is defensible and required. The previous R5 demanded source preservation across resume boundaries even though public semver policy requires `ResumeError::JournalAppendFailed` to remain a unit variant. That combination is impossible without a semver break or a fake ambient side channel.

## Revised R5

R5 now requires semver-compatible semantic preservation:

- no false success;
- failed `Resumed` append restores `Resumable`;
- conversion/fallback behavior is deterministic;
- hidden stale-source theft is forbidden;
- exact source detail is guaranteed only when a public error shape or owner-approved explicit non-ambient API actually carries and binds it.

## Updated Artifacts

- `contract.md`
- `domain-model-review.md`
- `tla-spec.md`
- `lean-contract.md`
- `verification-layers.md`
- `proof-obligations.jsonl`
- `traceability-matrix.jsonl`
- `state3-contract-repair-report.md`

## Routing

- State 4/7: consume narrowed obligations; optionally add/waive the small TLA+ workflow model.
- State 8: repair tests that expect source identity from unit `JournalAppendFailed`; add deterministic fallback/no-stale-source assertions instead.
- State 10: implement only semver-compatible fallback semantics; do not use globals, thread locals, task locals, cached stale errors, or hidden side channels to fake source binding.
- State 12: review against narrowed R5; source binding is required only for public source carriers or approved explicit non-ambient APIs.

## Owner Decision Still Needed Only If

If stakeholders still require exact per-error source identity from `ResumeError::JournalAppendFailed`, route to `STATUS: BLOCKED_OWNER_DECISION` and choose one:

1. semver-breaking public error shape that carries source; or
2. requirement relaxation as written in narrowed R5.
