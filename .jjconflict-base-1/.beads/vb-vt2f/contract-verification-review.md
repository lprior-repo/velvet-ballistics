# Contract Verification Review — vb-vt2f stale ask oracle repair rerun

STATUS: APPROVED

## Scope

- Bead: `vb-vt2f`.
- State/sublane: State 6 `contract-verification-after-stale-ask-test-oracle-repair`.
- Review goal: confirm contract ERR-004, repaired Kani projection, and executable BDD oracle all require immediate `Err(RuntimeError::RunNotFound)` / projected `Err(KernelRuntimeError::RunNotFound)` for stale `answer_ask` without overclaiming concrete-runtime Kani equivalence.

## Startup Rules Read

- `/home/lewis/.claude/skills/contract-verification-reviewer/SKILL.md`: lines 22-32 require JSONL validation, TLA+/Verus-first rigor, executable obligations, source-lint/test-style separation, and no hallucinated evidence; lines 127-152 require proof-obligation schema/status checks and TLA/Kani boundary discipline.
- `/home/lewis/.agents/skills/contract-verification-reviewer/SKILL.md`: same content read; no conflict observed, and this agents copy wins if conflict exists.

## Files Reviewed

- `.beads/vb-vt2f/contract.md`
- `.beads/vb-vt2f/verification-layers.md`
- `.beads/vb-vt2f/proof-obligations.jsonl`
- `.beads/vb-vt2f/proof-obligations.planned.jsonl`
- `.beads/vb-vt2f/proof-evidence.md`
- `.beads/vb-vt2f/proof-review.md`
- `.beads/vb-vt2f/test-writer-report.md`
- `.beads/vb-vt2f/test-plan-review.md`
- `.beads/vb-vt2f/test-suite-review.md`
- `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`
- `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs`

## Command Evidence

- `pwd -P && test -s ... && jq -c . .beads/vb-vt2f/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-vt2f/proof-obligations.planned.jsonl >/dev/null && python -c ...` in `/home/lewis/src/bd-vb-vt2f-bdd` -> PASS; printed `/home/lewis/src/bd-vb-vt2f-bdd`, `proof-obligations.jsonl: valid jsonl lines=40 required_schema=ok status=planned`, `proof-obligations.planned.jsonl: valid jsonl lines=40 required_schema=ok status=planned`, `input files present: ok`.
- Content search for `ERR-004|InvalidAskTicket|RunNotFound|stale ask|Stale` -> PASS; found contract ERR-004 at `contract.md:65`, proof repair/evidence at `proof-evidence.md:300-306,334-359`, proof review approval at `proof-review.md:16-20,56-62`, test writer repair at `test-writer-report.md:20-24,38-50`, and test reviews approved at `test-plan-review.md:15-29,39` plus `test-suite-review.md:27-52,68-70`.
- Content search in `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs` -> PASS; `TicketShape::Stale` is excluded from the success arm and returns `Err(KernelRuntimeError::RunNotFound)` at lines 150-153 and 165-166; only `TicketShape::Matching` reaches the success branch at lines 259-263; stale/wrong/absent assertions require `RunNotFound` at lines 265-270.
- Content search/read in `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` -> PASS; `test_direct_api_answer_ask_rejects_stale_ticket_without_mutating_unrelated_run` asserts immediate `runtime.answer_ask(...) == Err(RuntimeError::RunNotFound)` at lines 687-695 and preserves unrelated snapshot/trace/active-list/counters at lines 697-708.

## Findings

- LETHAL: none.
- MAJOR: none.
- MINOR: none.

## Coverage Decision

- Contract ERR-004 parity: APPROVED. `contract.md:65` requires stale or mismatched ask tickets to return an exact public typed error and not mutate another run.
- Kani projection parity: APPROVED, with boundary limits. The repaired owner-authorized projection kernel models stale `answer_ask` and stale `tick_after_answer` as `KernelRuntimeError::RunNotFound`; proof evidence records full Kani PASS for the repaired harness (`0 of 489 failed`, `7 of 7 cover properties satisfied`, `VERIFICATION:- SUCCESSFUL`).
- Executable BDD oracle parity: APPROVED. The direct API BDD test now expects immediate `Err(RuntimeError::RunNotFound)` from stale `Runtime::answer_ask(...)`; the current red nextest output is valid failing-first evidence against production and is not a contract/proof/test-oracle mismatch.
- Projection boundary: APPROVED. `verification-layers.md:53-58` and `proof-review.md:20,60` keep Kani claims limited to owner-authorized projection kernels only. This review does not treat Kani as executable equivalence for concrete `Runtime`/`Shard`/store/scheduler behavior.
- Source-lint/test-style separation: APPROVED. No source-lint obligation is used to reject test helper structure; the test oracle is judged by exact public behavior assertions and execution evidence.
- Waiver/Verus stance: APPROVED for this rerun scope. No new Verus proof is overclaimed; `WAIVER-VERUS-VT2F-002` remains tied to explicit compensating evidence and projection-risk acceptance conditions.

## Final Decision

STATUS: APPROVED

Next route: State 10 implementation repair. Production must make concrete `Runtime::answer_ask(...)` reject stale ask tickets immediately with `Err(RuntimeError::RunNotFound)` while preserving unrelated-run non-mutation, then rerun the direct API acceptance target and affected formal/review gates as directed by the controller.
