# Waiver Candidates — vb-rpch Verus/Flux/Rust

No behavior-affecting waivers are proposed.

## Tooling/scope blockers

### Flux RS — BLOCKED_TOOLING

- Evidence command: `cargo flux --version`
- Workdir: `/home/lewis/src/vb-jpq7-jj-fix`
- Observed output: `error: no such command: flux`
- Applies to: `INV-002`, `INV-003`, `INV-004`, `INV-005`, `PRE-001`, `PRE-002`, `POST-009`
- Expiry: recheck before State 11 production annotation work and before any State 5 Flux claim.
- This is not a behavior waiver and must not be cited as a Flux pass.

## Existing deferred runtime gaps preserved, not expanded

- GAP-3 Action ABI lookup and policy digest lookup remain existing scope deferrals tracked outside this suffix.
- TerminalStateMismatch public API gap remains existing deferred scope.
- None of those deferrals may be used to weaken the required Verus/Flux/Rust obligations listed here.
