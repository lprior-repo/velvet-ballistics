# Proof Plan Review Input

## Review Request

Review the refreshed State 4 attempt 3 proof plan for `vb-qi37.1.6` after repaired State 3. The plan converts repaired contracts into verifier-specific obligations without editing source code, tests, proof/model/harness/spec files, dependencies, or CI config.

## Inputs Reviewed By Planner

- `.beads/vb-qi37.1.6/STATE.md`
- `.beads/vb-qi37.1.6/contract.md`
- `.beads/vb-qi37.1.6/proof-obligations.jsonl`
- `.beads/vb-qi37.1.6/traceability-matrix.jsonl`
- `.beads/vb-qi37.1.6/delivery-scope.jsonl`
- `.beads/vb-qi37.1.6/codebase-map.md`
- `.beads/vb-qi37.1.6/tla-spec.md`
- `.beads/vb-qi37.1.6/verification-layers.md`
- `.beads/vb-qi37.1.6/proof-review.md`
- `.beads/vb-qi37.1.6/proof-findings.jsonl`
- `.beads/vb-qi37.1.6/proof-repair-guide.md`
- `.beads/vb-qi37.1.6/contract-verification-review.md`
- `.beads/vb-qi37.1.6/proof-evidence.md` as prior context only

## Planner Output To Review

- `.beads/vb-qi37.1.6/proof-strategy.md`
- `.beads/vb-qi37.1.6/proof-obligations.planned.jsonl`

## Required Review Checks

- Every planned row has the required schema fields: `id`, `requirement_id`, `contract_clause`, `risk`, `verifier`, `artifact`, `command`, `expected_evidence`, `assumptions`, `required`, `mode`, `owner_state`, `rerun_from`, `status`, and `waiver`.
- Every required row maps to one or more repaired contract clauses from `contract.md` and at least one traceability/proof-obligation source row.
- `PRE-006` is present in Verus, integration, and mutation rows.
- TLA+ coverage remains mandatory for PRE-002, PRE-003, POST-002 through POST-007, INV-002, INV-003, INV-004, and INV-007.
- TLA+ repair planning addresses State 6 findings: real TLC execution, active `EventuallyRecoveredOrRejected` or equivalent bounded liveness property, and transition-level event lifecycle mapping or reviewer-approved narrowing.
- Verus coverage remains mandatory for PRE-004, PRE-006, POST-001, POST-008, INV-001, INV-004, INV-005, and INV-006.
- Verus planning treats the prior abstraction proof as insufficient until production-shape mapping/refinement is written and reviewed.
- Kani/proptest/integration/mutation rows support exact typed-error and bounded recovery evidence without replacing TLA+ or Verus.
- Fuzz, Loom, Miri, theorem-kernel, and dependency lanes are not silently omitted; they are waived or not applicable with reasons and follow-up triggers.
- `blocked_tooling` rows are explicit for known TLC and canonical proof gate blockers.
- No row claims executable proof evidence already passed.

## Known Risks To Challenge

- `SlotWrittenEvent` lacks an explicit durable taint field; proof/test evidence must prevent secret-taint downgrade in snapshot-tail recovery.
- Lifecycle diagnostics `RunResumed`, `RunRetried`, and `RunAnswered` currently lack ordered sequence authority and must not affect recovered state unless modeled as sequenced facts.
- Pending action hydration must fail closed for unsupported/non-idempotent state; resolved action tickets must not duplicate effects.
- Collect pagination state lives in `SlotWrittenEvent.extra`; recovery evidence must show cursor/page/order/identity survives and corrupt/wrong identity fails typed.
- Public restart CLI path is UNKNOWN; plan uses crate-level integration as acceptance unless later discovery finds a stable command.
- `moon run :verify-proof` is currently blocked by a gauntlet script parse failure and cannot be treated as proof evidence until repaired.
- Direct TLC command is currently blocked by missing `tla2tools.jar` and cannot be treated as proof evidence until repaired or replaced by an equivalent checked runner.

## Expected Reviewer Verdict Format

- `APPROVED` if the planned obligations are traceable, sufficient, and waiver/blocker rationale is acceptable.
- `REJECTED` with blocking row IDs and exact missing clause/risk if any mandatory risk is uncovered or overclaimed.
- `CONDITIONAL` only when the condition is a precise edit to `proof-obligations.planned.jsonl` or `proof-strategy.md`.
