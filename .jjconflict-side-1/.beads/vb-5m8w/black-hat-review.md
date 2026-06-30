# Black Hat Review: vb-5m8w Step Budget Suspension

STATUS: APPROVED

## Startup Doctrine Cited

- `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md`: lines 12-16 require contract/bead parity first and rejection on parity failure; lines 18-21 require Farley rigor and behavior-focused deterministic tests; lines 23-33 require Holzman Rust/DDD checks and panic-vector review; lines 35-38 require simplicity/YAGNI review.
- `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md`: same doctrine, controlling on conflict; no conflict found.
- `response-template.md`: not present in either black-hat skill directory, so this artifact uses findings-first review format with explicit verdict.

## Findings

No blocking defects found.

## Evidence Reviewed

- Contract: `.beads/vb-5m8w/contract.md`.
- Proof/model artifacts: `verification/tla/StepBudgetSuspension.tla`, `verification/tla/StepBudgetSuspension.cfg`, `crates/vb_core/src/kani_step_budget_try_take_arbitrary.rs`, `.beads/vb-5m8w/proof-evidence.md`, `.beads/vb-5m8w/kani-report.md`, `.beads/vb-5m8w/tla-report.md`, `.beads/vb-5m8w/formal-verification-report.md`.
- Tests: `crates/vb_core/tests/vb_5m8w_step_budget_suspension.rs`, `crates/vb_runtime/tests/vb_5m8w_step_budget_suspension_runtime.rs`, `.beads/vb-5m8w/test-report.md`, `.beads/vb-5m8w/test-suite-review.md`.
- Production surface: `crates/vb_core/src/engine/signals.rs`, `crates/vb_core/src/engine/run_loop.rs`, `crates/vb_runtime/src/engine/drive.rs`, `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs`.

## Phase 1: Contract & Bead Parity

APPROVED.

- Contract requires bounded clamped budget, zero-budget graceful suspension, no pre-step mutation/evidence, completed-step durability, external-suspension distinction, and shard reschedule semantics.
- Production matches: `StepBudget::new` clamps; `StepBudget::try_take` returns `Ok(false)` at zero; core/runtime drive loops consume budget before step execution/evidence; shard lifecycle keeps `StepBudgetExhausted` runs via `DriveContinue`.
- TLA config checks the required invariants/properties: bounded arithmetic, non-terminal exhaustion, preservation, no false success on external suspension, suspension disjointness, and max-budget decrement.
- Tests cover the observable contract at core, runtime evidence, external suspension, and shard lifecycle levels.

## Phase 2: Farley Engineering Rigor

APPROVED WITH WAIVER.

- Existing production function `drive_deterministic_full` has more than five parameters, but it predates this bead and is not expanded by State 10. The waiver is acceptable because State 12 is reviewing this bead's step-budget delivery, not unrelated API ergonomics.
- New bead tests are deterministic and assert behavior, not implementation-only trivia: exact signals, PC/executed counters, step states, slot contents, evidence absence/presence, and shard retention.
- No hidden I/O was added to pure budget behavior. No production code changes were made in State 10.

## Phase 3: Holzman Rust / DDD

APPROVED.

- Budget exhaustion is represented by explicit sum-type variants (`EngineSignal::StepBudgetExhausted`, `RuntimeSignal::StepBudgetExhausted`), not boolean state.
- The public `StepBudget` constructor parses/clamps at the boundary and keeps the raw counter private.
- Run workflow states remain explicit state-to-state transitions through frame state and shard lifecycle.
- The Kani harness uses generated bounded shapes and production `StepBudget::try_take`/`RunFrame` observables rather than a single fixed dummy frame.

## Phase 4: Simplicity & Panic Vector

APPROVED.

- Reviewed changed bead test/proof harness surfaces do not introduce `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, or ignored/sleep-based weak tests.
- Error paths propagate typed `Result` or explicit signals.
- The `StepCounterOverflow` executable-test waiver is acceptable because `StepBudget::remaining` is private, all public constructors clamp, and the defensive branch is unreachable through safe public/test-only construction; compensating TLA/Kani/property evidence exists.

## Phase 5: Bitter Truth / Legibility

APPROVED.

- The delivery is boring and direct: formal model, Kani harness, focused tests, and no speculative production abstractions.
- No performance, release, dependency, network, storage, parser, or API-compatibility claims were made without evidence.

## Gate Evidence Accepted

- TLC: PASS, `6224` states generated, `3324` distinct states, depth `14`.
- Kani boundary harnesses: PASS, zero failed checks.
- Kani structural harness: PASS, `0 of 1939 failed`, one harness verified.
- Scoped nextest: PASS, `439 passed`, `3091 skipped`.
- Scoped proptest command: PASS with `PROPTEST_CASES=1024`.
- `moon ci`: PASS, `23` tasks completed; workspace tests `10900 passed`, `44 skipped`; mutants smoke `1/1 caught`.

## Verdict

APPROVED. Proceed to State 13 evidence packaging. Do not create `defects.md`; there are no mandated fixes for this bead.
