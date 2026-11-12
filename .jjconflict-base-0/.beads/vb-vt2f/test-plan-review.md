# Test Plan Review — vb-vt2f State 9 trace-eviction stale ask BDD

STATUS: APPROVED

## Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 56-110 require plan inquisition, exact error assertions, mutation survivability, and explicit evidence preconditions.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content observed; agents copy wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-20 require traceable exact behavior evidence; lines 32-48 allow bounded helpers only when assertions stay exact; lines 94-110 require explicit Given preconditions.

## VERDICT: APPROVED

Scope: bead `vb-vt2f` only, State 9 sublane `test-review-trace-eviction-stale-ask-after-black-hat`. This review covers the new RED BDD regression added after black-hat `LETHAL-001`; it does not approve production implementation.

## Contract / Defect Parity

- [PASS] `.beads/vb-vt2f/black-hat-review.md:13-20` and `.beads/vb-vt2f/defects.md:3-9` require proof that stale `answer_ask` rejects immediately even when terminal trace evidence was dropped/evicted from bounded trace history.
- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:720-775` adds exactly that scenario: trace capacity is starved, the stale run has already reached `InspectResponse::NotFound`, retained trace is asserted to contain only `RunSubmitted`, then stale `Runtime::answer_ask(...)` is called.

## Public API / Black-Box Surface

- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:17-21` imports public `vb_runtime` APIs only for runtime/journal/shard/trace/error types.
- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:724,734-743,755-762` exercises public direct runtime API (`Runtime::new`, `snapshot_run`, `list_events`, `answer_ask`) rather than private shard internals.
- [PASS] Static private-surface scan found no `use crate::`, `crate::`, `super::`, `include!`, `#[path]`, `pub(crate)`, or `pub(super)` in the target test file.

## Assertion Sharpness

- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:741-744` proves the terminal event is unavailable from retained trace: exact `vec![TraceEvent::RunSubmitted { run: stale }]`.
- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:755-762` asserts the immediate direct call result exactly: `Err(RuntimeError::RunNotFound)`, not `is_err()` and not delayed `tick_all` evidence.
- [PASS] `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs:763-773` asserts unrelated-run non-mutation by exact equality on snapshot, trace events, active-list vector, and counters.

## Mutation Survivability

- [PASS] Mutant "return `Ok(())` / enqueue stale answer when terminal trace is absent" is killed at line 762; current RED evidence shows `left: Ok(())`, `right: Err(RunNotFound)`.
- [PASS] Mutant "delete unrelated-run preservation" is killed by lines 763-773.
- [PASS] Mutant "pretend terminal trace is still retained" is killed by lines 741-744.

## Findings

- LETHAL: none.
- MAJOR: none.
- MINOR: none.

## Next Route

Route to State 10 implementation repair. The BDD is intentionally RED and strong enough to guard `LETHAL-001`.

STATUS: APPROVED
