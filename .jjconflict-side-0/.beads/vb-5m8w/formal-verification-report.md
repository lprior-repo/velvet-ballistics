# Formal Verification Report

STATUS: APPROVED

## Startup Doctrine Cited
- `/home/lewis/.claude/skills/formal-verifier/SKILL.md`: lines 21-31 require approved formal plan, every obligation accounted, scope-before-status, missing tools fail closed, and no hallucinated evidence.
- `/home/lewis/.agents/skills/formal-verifier/SKILL.md`: same content and controlling if conflict appears; no conflict found.

## Inputs
- proof-obligations.jsonl: present, valid JSONL.
- traceability-matrix.jsonl: present, valid JSONL.
- delivery-scope.jsonl: present, valid JSONL.
- baseline-report.md: present; baseline `moon ci` passed.
- tla-spec.md / lean-contract.md: present.
- contract-verification-review.md: `STATUS: APPROVED`.

## Tool Availability
- tlc / TLC: available via `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`; `tla2tools` available.
- apalache-mc: available.
- verus: available, but Verus lane is explicitly waived/non-required.
- lake: available.
- aeneas / charon: missing; no required Aeneas lane.
- hax: missing; no required Hax lane.
- cargo kani: available.
- moon: available.
- jq: available.
- Optional second-ring tools missing where not required: crux-mir, cargo-careful, cargo-fuzz, cargo-bolero, lockbud, cargo-asm, cargo semver/provenance tools, crux, saw, stateright.

## Obligation Results
- `TLA-BUDGET-001..006`: PASS. TLC exited 0 with no errors, 6224 generated states, 3324 distinct states, depth 14.
- `VERUS-BUDGET-001`: WAIVED. Approved non-required Verus waiver; no detached Verus PASS claimed.
- `KANI-BUDGET-001`: PASS. Boundary package/lib harness command chain exited 0; Kani reported successful verification with zero failed checks.
- `KANI-BUDGET-002`: PASS. Structural production-bound `kani_step_budget_try_take_arbitrary` exited 0; `0 of 1939 failed`; one harness verified.
- `TEST-BUDGET-001`: PASS. Scoped nextest exited 0; `439 passed`, `3091 skipped`.
- `PROP-BUDGET-001`: PASS. `PROPTEST_CASES=1024` scoped cargo test exited 0; selected StepBudget tests/proptests passed.
- `CI-BUDGET-001`: PASS. `moon ci` exited 0; `Tasks: 23 completed`; workspace tests `10900 passed`, `44 skipped`; mutants smoke `1/1 caught`.
- `REVIEW-BUDGET-001`: PASS. Independent contract/proof/test reviews are approved.
- `LEAN-BUDGET-001`: WAIVED/not applicable by approved obligation; no theorem-owned clause.
- `OTHER-LANES-001`: WAIVED/not applicable by approved obligation; no triggered optional lane.

## Waivers
- `VERUS-BUDGET-001`: accepted non-required waiver with owner, expiry, limitation, and compensating TLA/Kani/test evidence.
- `LEAN-BUDGET-001` and `OTHER-LANES-001`: not applicable per approved obligation rows.

## Residual Risk
- No blocking formal, machine-gate, local, regression, or release failure remains for State 11.

## Decision
APPROVED/PASS. Advance to State 12 black-hat review.
