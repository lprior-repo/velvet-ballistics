bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 11
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Formal Verification Report

### Obligations

| ID | Requirement | Verifier | Result | Evidence |
|---|---|---|---|---|
| vb-2bzz-obl-1 | EARS-1: ActionAbiMismatch | unit-test | PASS | `action_abi_mismatch_returns_typed_error` asserts exact variant + action_id |
| vb-2bzz-obl-2 | EARS-2: PolicyDigestMismatch | unit-test | PASS | `policy_digest_mismatch_returns_typed_error` asserts exact variant + step |
| vb-2bzz-obl-3 | EARS-3: Explicit input | unit-test | PASS | Empty input tests return Ok without guessing |

### Decision

STATUS: APPROVED

All required obligations have passing evidence. No formal proof (TLA+/Verus/Kani) required for this API surface change — unit tests provide sufficient evidence for typed error returns.
