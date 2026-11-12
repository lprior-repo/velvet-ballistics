# Assurance Bundle - vb-core-trigger-contract

STATUS: APPROVED

## Scope

- Bead: `vb-core-trigger-contract`
- Requirement: Align manual, schedule, event, and webhook trigger authoring contract across `vb_yaml`, `vb_validate`, and `vb_compile`.
- Implementation commit: `831c38db` (`vb-core-trigger-contract: align trigger error types and event field name`)
- Current `origin/main`: `50c68e8b473f2b71f314079749bf63a84363bd5d`

## Origin/Main Evidence

- Command: `git rev-parse HEAD origin/main && git merge-base --is-ancestor 831c38db origin/main`
- Result: `HEAD` and `origin/main` both resolved to `50c68e8b473f2b71f314079749bf63a84363bd5d`; merge-base command exited successfully, proving `831c38db` is reachable from `origin/main`.
- Command: `/usr/bin/git --no-pager show --name-only --oneline --no-renames 831c38db`
- Result: implementation commit touched only trigger contract files in `vb_yaml`, `vb_validate`, and `vb_compile`.

## State 5-12 Verification

- Artifact finding: clean `origin/main` did not contain `.beads/vb-core-trigger-contract/` before this State 13-15 repair.
- External legacy State tracker copies were found under `/home/lewis/src/vb-go-skill/p0-wave-20260515*/.beads/vb-core-trigger-contract/STATE.md`, but those are not `origin/main` artifacts and were treated as context only.
- Code finding: `831c38db` is on `origin/main`; active scoped tests pass on the current clean landing worktree.
- Residual code note: `crates/vb_validate/src/schema/validation.rs` still contains an orphan legacy `event` field path using `type`; no module reference was found by active search. The active validator path in `schema_fields.rs` uses `name`.

## Active Execution Evidence

- Command: `rtk cargo test -p vb_yaml -p vb_validate -p vb_compile trigger --lib`
- Observed result: `cargo test: 47 passed, 1345 filtered out (3 suites, 0.05s)`
- Command: `rtk cargo test -p vb_yaml unsupported_trigger --lib`
- Observed result: `cargo test: 1 passed, 203 filtered out (1 suites, 0.00s)`
- Command: `rtk cargo test -p vb_yaml -p vb_validate -p vb_compile --lib`
- Observed result: `cargo test: 1392 passed (3 suites, 15.51s)`

## Requirement Mapping

| Requirement | Evidence | Disposition |
| --- | --- | --- |
| `ipc`/`http` authoring triggers reject through typed unsupported-trigger errors | `crates/vb_yaml/src/error.rs`; `crates/vb_yaml/src/ast/parse_trigger.rs`; `rtk cargo test -p vb_yaml unsupported_trigger --lib` | COVERED |
| `event` trigger authoring uses `name` in active parser and validator path | `crates/vb_yaml/src/ast/parse_trigger.rs`; `crates/vb_validate/src/schema_fields.rs`; scoped trigger tests | COVERED |
| Manual, webhook, schedule, and event trigger tests still pass | `rtk cargo test -p vb_yaml -p vb_validate -p vb_compile trigger --lib` | COVERED |
| Relevant crates pass their full library suites on current `origin/main` | `rtk cargo test -p vb_yaml -p vb_validate -p vb_compile --lib` | COVERED |
| Implementation reached remote main | `git merge-base --is-ancestor 831c38db origin/main` | COVERED |

## Decision

The implementation code is on `origin/main`, scoped and full relevant library tests pass, and the missing State 13-15 evidence is now packaged. State 13 is approved for landing/closure.
