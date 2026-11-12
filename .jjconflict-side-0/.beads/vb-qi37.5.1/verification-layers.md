# Verification Layers

## Boundary
- Verified kernel: Idempotency decision table in `crates/vb_validate/src/idempotency_contract.rs`
- Lean contract projection: lean-contract.md with waiver (decision table exhaustively tested)
- Runtime shell: WorkflowParts traversal, contract registry matching, error accumulation
- External systems excluded from formal proof: Fjall storage, IPC, runtime action dispatch

## Layer Assignment

| Contract Clause | Layer | Evidence |
|-----------------|-------|----------|
| I1 (Canonical types) | static/API | compile-time type enforcement |
| I2 (Registry completeness) | integration | `validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract` |
| I3 (Pure action acceptance) | unit + kani | 35 tests + Kani harness `pure_action_always_accepted` |
| I4 (Side-effecting unsafe rejection) | unit + kani | 35 tests + Kani harness `side_effecting_unsafe_rejected` |
| I5 (Side-effecting at-least-once rejection) | unit + kani | 35 tests + Kani harness `side_effecting_at_least_once_rejected` |
| I6 (Side-effecting accepted) | unit + kani | 35 tests + Kani harness `side_effecting_accepts_only_idempotent_external` |
| I7 (Key-required separation) | unit | `runtime_returns_missing_key_when_key_required_action_has_empty_key_slots` |
| I8 (Secret taint key invariant) | unit | `runtime_returns_secret_in_key_when_key_slot_taint_is_secret` + `runtime_returns_secret_in_key_when_key_slot_taint_is_derived_from_secret` |
| I9 (Bounded traversal) | kani + proptest | Kani harness `accumulation_bounded_by_contract_length` + proptest |
| I10 (Deterministic diagnostics) | unit | `collect_returns_same_boxed_violations_when_called_twice_with_same_input` |
| P1 (Structural validation precondition) | static | Gate 12 completeness check |
| P2 (Contract completeness precondition) | integration | `validate_workflow_returns_completeness_error_without_claiming_proof_when_gate_12_fails` |
| Q1 (Success postcondition) | unit | All acceptance tests |
| Q2 (Violation accumulation postcondition) | unit + proptest | `collect_returns_all_boxed_violations_in_input_order_for_multiple_illegal_contracts` |
| Q3 (Completeness failure postcondition) | integration | `validate_workflow_returns_action_contract_missing_when_do_node_has_no_matching_contract` |
| Q4 (No mutation postcondition) | unit | `validate_workflow_leaves_parts_and_contracts_equal_to_original_after_validation` |
| Q5 (No effect postcondition) | static | no-external-effects integration test |
| Q6 (Stable diagnostics postcondition) | unit | deterministic order tests |

## Lean Scope

- Theorem module: VbCore.Idempotency
- Rust target: is_statically_idempotent_contract, validate_action_idempotency_contract
- Abstraction relation: 45-case enum match table is total and deterministic
- Shell exclusions: No I/O, storage, IPC, action dispatch, or runtime effects in pure kernel
- Non-goals: Lean not required due to exhaustive testing + Kani formal verification

## Fuzz Targets

1. **verifier_gates**: Bounded IR + action contracts → typed Result
2. **cli_verifier_proof_boundary**: Workflow source bytes + optional registry bytes

## Static Gates

- `moon ci`: All tests, clippy, format
- `cargo-mutants`: >= 90% kill rate
- `cargo-fuzz`: No crashes on bounded inputs
- `cargo kani`: All 5 harnesses proven

## Waivers

- Lean projection: Decision table is exhaustively tested via 35 unit tests + 5 Kani harnesses; formal Lean projection provides no additional assurance for this bounded enum domain.
- Performance claims: No performance-critical hot path in verifier; cold-path only.
