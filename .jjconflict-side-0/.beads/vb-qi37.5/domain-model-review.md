# Domain Model Review: vb-qi37.5

## Verdict

STATUS: NEEDS INDEPENDENT REVIEW

State 3 defines the contract boundary but does not approve itself. A `contract-verification-reviewer` must approve or reject before proof planning, tests, or implementation consume it.

## Domain Model Strengths

- The domain separates action contract metadata, static decision, certificate summary, runtime admission, and replay behavior.
- State 2 already found key proof surfaces in `vb_core`, `vb_validate`, `vb_compile`, `vb_storage`, and `vb_runtime`.
- The contract makes the validate/compile decision mismatch explicit rather than hiding it behind tests.

## Domain Model Risks

- `DeterministicPure` with side effects is semantically suspicious. Either it is not pure, or the side effect must be modeled as replay-safe with an explicit key/policy. Default contract stance is reject.
- Certificate fields can become ceremonial if `VerificationProof.idempotency_keyed` and `idempotency_attested` are not derived from accepted contracts.
- Storage/runtime proof schema mismatch (`ADMISSION_GATE_COUNT` 2 versus runtime `REQUIRED_GATE_COUNT` 15) can make a correct verifier unusable at admission.
- Duplicate completion semantics must distinguish same ticket/key and same digest from stale/conflicting completion.

## Required Reviewer Questions

- Is `vb_validate::idempotency_contract::is_statically_idempotent_contract` the canonical decision table?
- Should random/time idempotency key ingredient rejection be in this bead or a follow-up?
- Does certificate evidence need ordered arrays, sets, or deterministic sorted action IDs?
- Are duplicate completion and stale completion owned by `vb_core::action`, `vb_runtime`, `vb_storage::recovery`, or a cross-crate contract?

## Scott-DDD Notes

- Prefer a typed `IdempotencyDecision`/`IdempotencyEvidence` domain object instead of boolean proof flags when implementation begins.
- Make illegal states unrepresentable: accepted artifacts should not construct without a proof schema version, gate count, and idempotency evidence status.
- Error variants should encode reason and action identity, not free-form strings.
