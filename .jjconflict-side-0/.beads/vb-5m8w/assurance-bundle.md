# Assurance Bundle: vb-5m8w Step Budget Suspension

STATUS: APPROVED

## Scope

- Bead: `vb-5m8w` — Add TLA+ Step Budget Model.
- Workspace: `/home/lewis/src/go-skill-vb-5m8w`.
- State: 13 evidence package.
- Evidence doctrine: Truth Serum startup files were read: `/home/lewis/.claude/skills/truth-serum/SKILL.md` and `/home/lewis/.agents/skills/truth-serum/SKILL.md`. Both require direct command evidence, no delegated proof laundering, and explicit unavailable proof labels; no conflict found, `.agents` is controlling.

## Command Evidence Index

| ID | Evidence class | Command / source | Observed result | Status |
|---|---|---|---|---|
| CMD-ISO | RAW-CURRENT | `pwd -P` | `/home/lewis/src/go-skill-vb-5m8w`, `EXIT:0` | PASS |
| CMD-PATHS | RAW-CURRENT | Python path check for bead, proof, test, TLA, Kani artifacts | all required paths exist and are non-empty, `EXIT:0` | PASS |
| CMD-JSONL | RAW-CURRENT | Python JSONL/traceability check | `traceability-matrix.jsonl: rows=21`, `proof-obligations.jsonl: rows=15`, `proof-obligations.planned.jsonl: rows=15`, `verification-ledger.jsonl: rows=15`, `missing=[]`, `EXIT:0` | PASS |
| CMD-TLA | RAW-CURRENT | `tla2tools verification/tla/StepBudgetSuspension.tla -config verification/tla/StepBudgetSuspension.cfg` | `Model checking completed. No error has been found.`, `6224 states generated`, `3324 distinct states found`, depth `14`, `EXIT:0` | PASS |
| CMD-CORE-TEST | RAW-CURRENT | `cargo +nightly test -p vb_core --test vb_5m8w_step_budget_suspension -- --nocapture` | `11 passed; 0 failed`, `EXIT:0` | PASS |
| CMD-RUNTIME-TEST | RAW-CURRENT | `cargo +nightly test -p vb_runtime --test vb_5m8w_step_budget_suspension_runtime -- --nocapture` | `6 passed; 0 failed`, `EXIT:0` | PASS |
| CMD-NEXTEST | RAW-CURRENT | `cargo +nightly nextest run -p vb_core -p vb_runtime -E 'test(/budget\|Budget\|StepBudgetExhausted\|AwaitingAction\|AwaitingWait\|AwaitingAsk\|evidence/)'` | `439 tests run: 439 passed, 3091 skipped`, `EXIT:0` | PASS |
| CMD-PROP | RAW-CURRENT | `PROPTEST_CASES=1024 cargo +nightly test -p vb_core -p vb_runtime step_budget -- --nocapture` | selected tests passed across `vb_core` and `vb_runtime`, `EXIT:0` | PASS |
| CMD-MOON | RAW-CURRENT | `moon ci` | `Tasks: 23 completed`, `10900 tests run: 10900 passed, 44 skipped`, mutants smoke `1 mutant tested: 1 caught`, `EXIT:0` | PASS |
| CMD-KANI-STRUCT | RAW-CURRENT | `cargo kani -p vb_core --lib --harness kani_step_budget_try_take_arbitrary --no-assertion-reach-checks` | `SUMMARY: ** 0 of 1939 failed`, `VERIFICATION:- SUCCESSFUL`, `Complete - 1 successfully verified harnesses, 0 failures, 1 total`, `EXIT:0` | PASS |
| CMD-KANI-BOUNDARY | RAW-CURRENT BLOCKER + HISTORICAL RAW ARTIFACT | Boundary chain command from `kani-report.md` rerun by State 13 | current rerun reached Kani then failed with `fatal error: ... Disk quota exceeded`, `EXIT:1`; historical raw output is recorded at `/home/lewis/.local/share/opencode/tool-output/tool_e3cb05216001ncUMkdOGwZP0nQ` and accepted by State 11/12, but not counted as a current rerun | BLOCKED_CURRENT / ACCEPTED_HISTORICAL |

## Review Evidence Index

| Review artifact | Status | Scope |
|---|---:|---|
| `.beads/vb-5m8w/contract-verification-review.md` | APPROVED | contract, TLA metadata, obligation ledger, traceability, waiver quality |
| `.beads/vb-5m8w/proof-review.md` | APPROVED | TLA, Kani, Verus waiver, proof boundary |
| `.beads/vb-5m8w/test-plan-review.md` | APPROVED | scenario plan and repair closure |
| `.beads/vb-5m8w/test-suite-review.md` | APPROVED | exact assertions, compile/run evidence, no weak test patterns |
| `.beads/vb-5m8w/formal-verification-report.md` | APPROVED | State 11 proof/test/machine gate ledger |
| `.beads/vb-5m8w/black-hat-review.md` | APPROVED | adversarial contract parity, panic vectors, DDD/Farley/simplicity |

## Requirement Traceability Matrix

| Clause | Requirement | Contract source | Proof/test evidence | Review evidence | Command evidence | Final status |
|---|---|---|---|---|---|---|
| PRE-001 | Drive slice starts from valid resumable or terminal-aware run state | `contract.md` PRE-001 | `TLA-BUDGET-005`; tests `given_budget_suspended_run_when_fresh_budget_scheduled_then_run_resumes_from_same_pc`, `given_terminal_run_when_resume_attempted_then_invalid_resume_error` | contract/proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| PRE-002 | Budget is bounded and clamped to `MAX_STEP_BUDGET` | `contract.md` PRE-002 | `TLA-BUDGET-001`, `VERUS-BUDGET-001` waiver, `KANI-BUDGET-001`; test `given_budget_above_max_when_constructed_then_clamped_to_max_step_budget` | Verus waiver accepted; black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-NEXTEST, CMD-PROP, CMD-MOON; KANI boundary historical accepted/current blocked | APPROVED_WITH_CURRENT_BOUNDARY_BLOCKER_NOTED |
| PRE-003 | Step starts only after successful budget consumption | `contract.md` PRE-003 | `TLA-BUDGET-003`; test `given_positive_budget_when_step_starts_then_budget_decrements_before_execution` | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| PRE-004 | Zero budget must not start or complete a step | `contract.md` PRE-004 | `TLA-BUDGET-002`, `TLA-BUDGET-004`, `TEST-BUDGET-001`; test `given_zero_budget_when_drive_runs_then_no_step_started_or_succeeded_evidence` | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| PRE-005 | Model includes exact bounded arithmetic and invalid arithmetic states | `contract.md` PRE-005 | `TLA-BUDGET-001`; StepCounterOverflow test waiver accepted as unreachable through safe public/test-only construction | contract/proof/test/black-hat approved | CMD-TLA, CMD-JSONL, CMD-MOON | APPROVED |
| POST-001 | Zero budget returns graceful `StepBudgetExhausted`, not terminal failure | `contract.md` POST-001 | `TLA-BUDGET-002`, `TEST-BUDGET-001`; core/runtime tests | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| POST-002 | Positive budget decrements exactly once per deterministic step | `contract.md` POST-002 | `TLA-BUDGET-003`, Kani boundary, property tests | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-PROP, CMD-NEXTEST; KANI boundary historical accepted/current blocked | APPROVED_WITH_CURRENT_BOUNDARY_BLOCKER_NOTED |
| POST-003 | Exhaustion preserves PC/frame/run state except consumed effects | `contract.md` POST-003 | `TLA-BUDGET-004`, `KANI-BUDGET-002`, `TEST-BUDGET-001`; core/runtime tests | proof/test/black-hat approved | CMD-TLA, CMD-KANI-STRUCT, CMD-CORE-TEST, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| POST-004 | Budget-exhausted run remains eligible for reschedule | `contract.md` POST-004 | `TLA-BUDGET-005`, runtime reschedule tests | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| POST-005 | Runtime lifecycle preserves run and schedules continuation | `contract.md` POST-005 | `TLA-BUDGET-002`, `TLA-BUDGET-005`, runtime lifecycle test | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| POST-006 | External suspensions remain distinct from budget exhaustion | `contract.md` POST-006 | `TLA-BUDGET-006`, AwaitingAction/Wait/Ask tests | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| INV-001 | Budget stays in `0..=MAX_STEP_BUDGET`; no wrap/underflow | `contract.md` INV-001 | `TLA-BUDGET-001`, Kani boundary, property tests | proof/test/black-hat approved | CMD-TLA, CMD-PROP, CMD-NEXTEST, CMD-MOON; KANI boundary historical accepted/current blocked | APPROVED_WITH_CURRENT_BOUNDARY_BLOCKER_NOTED |
| INV-002 | `try_take` at zero returns false and leaves state unchanged | `contract.md` INV-002 | `TLA-BUDGET-002`, `KANI-BUDGET-002`, core tests | proof/test/black-hat approved | CMD-TLA, CMD-KANI-STRUCT, CMD-CORE-TEST, CMD-PROP, CMD-NEXTEST | APPROVED |
| INV-003 | `try_take` positive decrements once and returns true | `contract.md` INV-003 | `TLA-BUDGET-003`, Kani boundary, core/property tests | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-PROP, CMD-NEXTEST; KANI boundary historical accepted/current blocked | APPROVED_WITH_CURRENT_BOUNDARY_BLOCKER_NOTED |
| INV-004 | Step evidence requires consumed budget | `contract.md` INV-004 | `TLA-BUDGET-003`, runtime evidence tests | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| INV-005 | Budget exhaustion is non-terminal scheduler suspension | `contract.md` INV-005 | `TLA-BUDGET-002`, runtime lifecycle tests | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| INV-006 | Exhaustion before step start cannot advance PC/mutate/write | `contract.md` INV-006 | `TLA-BUDGET-004`, `KANI-BUDGET-002`, core/runtime tests | proof/test/black-hat approved | CMD-TLA, CMD-KANI-STRUCT, CMD-CORE-TEST, CMD-RUNTIME-TEST, CMD-NEXTEST | APPROVED |
| INV-007 | Completed consumed steps remain durable after later exhaustion | `contract.md` INV-007 | `TLA-BUDGET-004`, core/runtime durability tests | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-RUNTIME-TEST, CMD-NEXTEST | APPROVED |
| INV-008 | External suspension does not emit false `StepSucceeded` | `contract.md` INV-008 | `TLA-BUDGET-006`, AwaitingAction/Wait/Ask tests | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| INV-009 | Fair fresh budgets eventually progress/suspend/finish/error | `contract.md` INV-009 | `TLA-BUDGET-005`, reschedule tests | proof/test/black-hat approved | CMD-TLA, CMD-CORE-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |
| INV-010 | Legacy terminal exhaustion model is forbidden | `contract.md` INV-010 | `TLA-BUDGET-002`, `REVIEW-BUDGET-001`, tests assert suspended/non-terminal behavior | proof/test/black-hat approved | CMD-TLA, CMD-RUNTIME-TEST, CMD-NEXTEST, CMD-MOON | APPROVED |

## Waivers and Blockers

- `VERUS-BUDGET-001`: APPROVED waiver. No Verus PASS is claimed; accepted by contract-verification, proof, formal, and black-hat reviews with compensating TLA/Kani/test evidence.
- `LEAN-BUDGET-001` and optional lanes: not applicable by approved obligation ledger.
- Current State 13 boundary-Kani rerun: BLOCKED by local disk quota during Kani preprocessing. This bundle does not launder that failed rerun as a current PASS. Historical raw output and State 11/12 accepted reports remain the evidence for `KANI-BUDGET-001`.

## Final Bundle Decision

All 21 contract clauses are mapped to contract rows, proof/test evidence, review evidence, command evidence, and final status. No missing artifact path or traceability row was found. The only current rerun gap is explicitly labeled as a disk-quota blocker for boundary Kani, with historical raw output identified separately. Final status: APPROVED.
