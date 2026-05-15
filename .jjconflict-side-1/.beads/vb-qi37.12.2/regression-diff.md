# Regression Diff — vb-qi37.12.2 State 11 Rerun

STATUS: APPROVED

## Prior State 11 blockers rechecked

- API compatibility: RESOLVED — `cargo semver-checks -p vb_runtime --baseline-rev HEAD` passed 196 checks with 56 skips; no semver-major failures remained.
- Mutation: RESOLVED — filtered scoped resume/is_resumable `cargo-mutants` run tested 6 mutants, caught 5, marked 1 unviable, and missed 0.

## Current regression classification

- No bead-local failures.
- No new release blockers observed in `vb_ipc` check/test gate.
- No waiver or deferred-global entry required for State 11 rerun.
