bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 12
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Black Hat Review — 5 Phases

### PHASE 1: Contract & Bead Parity

| Clause | Status | Evidence |
|---|---|---|
| EARS-1: ActionAbiMismatch on exact mismatch | PASS | `check_action_abi_digests` returns `RecoveryError::ActionAbiMismatch { action_id }` |
| EARS-2: PolicyDigestMismatch on exact mismatch | PASS | `check_policy_digests` returns `RecoveryError::PolicyDigestMismatch { step }` |
| EARS-3: Explicit verifier input, no guessing | PASS | Both functions take `entries` tuples; empty input returns `Ok(())` |
| INV-1: No false positive on missing ABI data | PASS | `check_action_abi_digests_empty_input_returns_ok` |
| INV-2: No false positive on missing policy data | PASS | `check_policy_digests_empty_input_returns_ok` |
| INV-3: Matching digests return Ok | PASS | `action_abi_match_returns_ok`, `policy_digest_match_returns_ok` |
| INV-4: Error carries exact action_id/step | PASS | Tests assert exact field values |

### PHASE 2: Farley Engineering Rigor

| Constraint | Status | Evidence |
|---|---|---|
| Functions under 25 lines | PASS | `check_action_abi_digests`: 10 lines, `check_policy_digests`: 10 lines |
| Max 5 parameters | PASS | Both functions take 1 parameter (`entries` slice) |
| Pure logic separated from I/O | PASS | Both functions are pure comparison loops — no journal I/O |
| Tests assert behavior not implementation | PASS | Tests assert exact error variants and field values |

### PHASE 3: Holzman Rust (Big 6)

| Rule | Status | Evidence |
|---|---|---|
| No unsafe/unwrap/expect/panic/todo/dbg | PASS | `#![forbid(unsafe_code)]`, no banned macros |
| Parse don't validate | N/A | No parsing boundary — comparison functions |
| Types as documentation | PASS | Uses `ActionId`, `StepIdx`, `WorkflowDigest` — no raw primitives |
| Explicit workflows | PASS | Simple comparison, no state machines |
| Newtypes for domain types | PASS | All domain types are newtypes from `vb_core` |
| No boolean parameters | PASS | No boolean parameters in new functions |

### PHASE 4: Scott Wlaschin DDD

| Rule | Status | Evidence |
|---|---|---|
| No Option-based state machines | PASS | No Option in control flow |
| CUPID properties | PASS | Composable, simple, predictable, idiomatic, domain-based |
| No panic vector | PASS | No unwrap/expect/panic |

### PHASE 5: Bitter Truth

| Rule | Status | Evidence |
|---|---|---|
| No cleverness | PASS | Straightforward iteration with early return |
| YAGNI | PASS | No generic handlers or abstract traits |
| Readable and boring | PASS | Painfully obvious comparison logic |

## Verdict

STATUS: APPROVED

Implementation exactly matches contract. Functions are pure, under 25 lines, single parameter, use domain types, and return typed errors with exact identifiers. Tests cover mismatch, match, and empty-input cases with exact assertions.
