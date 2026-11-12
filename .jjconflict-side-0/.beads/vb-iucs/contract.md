# Contract: vb-iucs

## Requirement

Recover the rejected proof integration target from issue context and existing repository evidence, then approve only the scoped proof repair if raw artifacts prove it.

## Scope

- Gate 8 accessor validation Kani harnesses must include success and rejection cases.
- StepState runtime transition predicate must be bound to the proof kernel and checked by Kani parity over all state pairs.
- StepState Verus mirror must verify the reviewed transition matrix and document its production binding path.
- BudgetArithmetic TLA+ must model bounded Rust integer behavior with explicit overflow/underflow outcomes.
- Evidence must prevent overclaiming full validation pipeline composition.

## Out Of Scope

- No new production code changes in this recovery attempt.
- No invented proof target if the recovered target were missing.
- No claim that Gate 8 Kani proves full pipeline composition.
- No claim that Gate 8 Verus or Gate 8 Miri ran here.

## Acceptance

- `vb-iucs` artifacts identify the target and raw evidence.
- State 13 decision is approved only for scoped proof integration.
- Bookmark `go-skill-p0-vb-iucs` is pushed if State 13 is approved.
- Stop before main merge.
