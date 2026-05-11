# Test Plan Review Rerun 2: vb-qi37.3

STATUS: APPROVED

## Startup / Doctrine Checked

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: Mode 1 plan review requires pure doc analysis for contract parity, exact assertions, trophy allocation, boundary completeness, mutation survivability, and evidence audit.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same Mode 1 rules; no conflict observed. Agents copy would win if conflict existed.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: evidence must be traceable, bounded/reproducible, explicit about setup/side effects, and preserve failure locality.

## Files Reviewed

- Current repaired `.beads/vb-qi37.3/test-plan.md`.
- Latest rejection in `.beads/vb-qi37.3/test-plan-review.md` before overwrite.
- Upstream approved artifacts remain in force: `contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, and `contract-verification-review.md`.

## Prior Blocker Verification

1. Recovery assertion repaired: `.beads/vb-qi37.3/test-plan.md:242-246` now requires `Err(RecoveryError::ReplayDivergence { step: StepIdx::ZERO, detail })` with exact public predicate `detail.contains("multiple runs")`; no `or equivalent` remains.
2. Capacity-full evidence test repaired: `.beads/vb-qi37.3/test-plan.md:318-322` uses concrete public `EvidenceCollector::push_slot_written_with_extra` with a prefilled `EvidenceCollector::with_capacity(2)`; no wrapper placeholder remains.
3. Fuzz ownership repaired: `.beads/vb-qi37.3/test-plan.md:352-368` makes fuzz formal-verifier-owned for this phase under `FUZZ-CODEC-001`; State 5 must not choose a fuzz path.
4. Sequence gap repaired: `.beads/vb-qi37.3/test-plan.md:266-270` and `.beads/vb-qi37.3/test-plan.md:425` require exact `Err(JournalError::SequenceGap { expected: EventSeq::new(2), actual: EventSeq::new(3) })`, no collect hydration call, and tempdir cleanup by drop.

## Plan Strength Check

- ERR-004..ERR-008 have exact target `EngineError` variants/kinds/fields via `CollectPageOrderViolation`, `CollectExtraHydrationFailed`, and `CollectEvidenceCapacityExceeded`.
- State 5 must-write tests have names, entry points, deterministic Given/When/Then, exact assertions, and commands.
- Hydration/schema/evidence capacity/boundary cases are split into exact scenarios.
- Proptest is concrete for State 5 unless formal verifier explicitly re-approves `PROP-COLLECT-001`; fuzz is unambiguously formal-verifier-owned.
- Mutation-killer mapping names exact tests.
- Wait/ask and cross-crate recovery scenarios specify integration surfaces, deterministic setup, journal/prefix construction, side effects/cleanup, and exact state/page assertions.

## Findings

- No blocking findings remain.

## Decision

- owner_state: none
- rerun_from: none
- State 4 can exit: YES
