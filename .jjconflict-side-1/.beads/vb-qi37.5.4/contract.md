# Contract Specification — vb-qi37.5.4

## Context

- **Feature**: Idempotency gate evidence — static and runtime verification that action contracts declare compatible idempotency, side-effect, and retry-safety classifications.
- **Domain terms**:
  - `Idempotency` — DeterministicPure (pure computation), IdempotentExternal (externally idempotent with key), AtLeastOnceExternal (at-least-once delivery)
  - `SideEffect` — None, Writes, Sends, Creates, Destroys
  - `RetrySafety` — Safe, KeyRequired, Unsafe
  - `IdempotencyViolation` — MissingKey, SecretInKey(u32), RandomInKey(u32), TimeInKey(u32)
  - `ActionContract` — full contract record with idempotency, side_effect, retry_safety fields
  - `is_statically_idempotent_contract` — compile-time/static decision table
  - `verify_idempotency` — runtime gate enforcing key taint checks
  - `check_idempotency_gates` — compile-time gate in vb_compile (must parity with vb_validate)
- **Assumptions**:
  - `side_effect == None` is always Ok regardless of retry_safety or idempotency
  - `retry_safety == Unsafe` with `side_effect != None` is always rejected regardless of idempotency
  - `idempotency == AtLeastOnceExternal` with `side_effect != None` is always rejected
  - `idempotency == DeterministicPure` with `side_effect != None` is always rejected
  - `idempotency == IdempotentExternal` with `side_effect != None` and `retry_safety in {Safe, KeyRequired}` is Ok
  - Key slots are validated at runtime only; static gate does not check key contents
  - `check_idempotency_gates` in vb_compile must accept/reject the exact same contract combinations as `is_statically_idempotent_contract` in vb_validate
- **Open questions**: None

---

## Preconditions

- PRE-001: `is_statically_idempotent_contract` accepts any `ActionContract` where `side_effect == None` regardless of idempotency or retry_safety.
- PRE-002: `is_statically_idempotent_contract` applies the 5-branch decision table to any `ActionContract` with `side_effect != None`.
- PRE-003: `verify_idempotency` requires `contract.idempotency == IdempotentExternal` and non-empty `key_slots` to proceed to key-ingredient validation.
- PRE-004: `validate_idempotency_key_ingredients` requires non-empty `key_slots` and a valid `frame` with slot value access.

---

## Postconditions

- POST-001: `is_statically_idempotent_contract` returns `Ok(())` iff the contract satisfies the decision table (branch 1 or branch 5).
- POST-002: `is_statically_idempotent_contract` returns `Err(SideEffectingRetryUnsafe)` iff `side_effect != None` and `retry_safety == Unsafe`.
- POST-003: `is_statically_idempotent_contract` returns `Err(SideEffectingAtLeastOnceExternal)` iff `side_effect != None` and `idempotency == AtLeastOnceExternal`.
- POST-004: `is_statically_idempotent_contract` returns `Err(SideEffectingDeterministicPure)` iff `side_effect != None` and `idempotency == DeterministicPure`.
- POST-005: `verify_idempotency` returns `Ok(())` iff all key slots pass taint checks (no SecretTaint, Random, or TimeDependent values).
- POST-006: `verify_idempotency` returns `Err(MissingKey(SideEffect))` iff idempotency requires a key but `key_slots` is empty.
- POST-007: `verify_idempotency` returns `Err(SecretInKey(u32))` iff a key slot index carries SecretTaint.
- POST-008: `verify_idempotency` returns `Err(RandomInKey(u32))` iff a key slot index carries Random.
- POST-009: `verify_idempotency` returns `Err(TimeInKey(u32))` iff a key slot index carries TimeDependent.
- POST-010: `check_idempotency_gates` (vb_compile) and `is_statically_idempotent_contract` (vb_validate) produce identical accept/reject results for all `ActionContract` inputs.

---

## Invariants

- INV-001: `IdempotencyViolation` variants are exhaustive and mutually exclusive (no overlapping error conditions).
- INV-002: `IdempotencyContractViolation` variants cover all three rejection branches and nothing else.
- INV-003: The decision table is confluent — given the same `(side_effect, retry_safety, idempotency)` triple, `is_statically_idempotent_contract` always returns the same result.
- INV-004: `verify_idempotency` returns exactly one `IdempotencyViolation` variant on the first failing taint check (no dual reporting).
- INV-005: Key slot taint is monotonic — once a slot is marked tainted it cannot become clean within the same frame.

---

## Error Taxonomy

### Static Gate Errors (IdempotencyContractViolation)
- `SideEffectingRetryUnsafe { action, side_effect, idempotency, retry_safety }` — side-effecting action with Unsafe retry safety
- `SideEffectingAtLeastOnceExternal { action, side_effect, idempotency, retry_safety }` — side-effecting action with AtLeastOnceExternal idempotency
- `SideEffectingDeterministicPure { action, side_effect, idempotency, retry_safety }` — side-effecting action with DeterministicPure idempotency

### Runtime Gate Errors (IdempotencyViolation)
- `MissingKey(SideEffect)` — IdempotentExternal requires a key but none provided
- `SecretInKey(u32)` — key slot index carries SecretTaint
- `RandomInKey(u32)` — key slot index carries Random
- `TimeInKey(u32)` — key slot index carries TimeDependent

### Workflow Errors (IdempotencyContractError)
- `ActionContractMissing { action_id, node_index }` — workflow references missing contract
- `ActionContractOrphan { action_id }` — contract present but workflow does not reference it
- `IdempotencyViolations(IdempotencyContractErrors)` — one or more contract-level violations

---

## Contract Signatures

```rust
// vb_validate — static gate
pub fn is_statically_idempotent_contract(
    contract: &ActionContract
) -> Result<(), IdempotencyContractViolation>

pub fn validate_action_idempotency_contract(
    contract: &ActionContract
) -> Result<(), IdempotencyContractViolation>

pub fn validate_workflow_idempotency_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract]
) -> IdempotencyContractResult<()>

pub fn collect_idempotency_contract_violations(
    action_contracts: &[ActionContract]
) -> Result<(), IdempotencyContractErrors>

// vb_core — runtime gate
pub fn verify_idempotency(
    contract: &ActionContract,
    key_slots: &[SlotIdx],
    frame: &RunFrame
) -> Result<(), IdempotencyViolation>

pub fn validate_idempotency_key_ingredients(
    key_slots: &[SlotIdx],
    frame: &RunFrame
) -> Result<(), IdempotencyViolation>

// vb_compile — compile-time gate (must match vb_validate)
pub fn check_idempotency_gates(contracts: &[ActionContract]) -> Result<(), IdempotencyContractError>
```

---

## Verus-Owned Clauses

- INV-001, INV-002, INV-003: Decision table confluence — pure function, provable by exhaustive case analysis in Verus
- INV-004: Single-variant error return — Verus spec can enforce at most one violation reported
- PRE-001 through PRE-004: Precondition invariants on pure types
- POST-001 through POST-009: Postconditions on pure functions

---

## TLA+-Owned Clauses

- Decision table as state machine: 5 branches, each a distinct action
- `verify_idempotency` runtime path as TLA+ next-state relation
- Key taint propagation as TLA+ predicates over slot state
- No temporal/liveness claims needed — all properties are safety invariants
- See `tla-spec.md` for full model

---

## Theorem-Owned Clauses

None. Rust-local pure logic is covered by Verus. No algebraic theorem kernel required.

---

## Non-goals

- Proof that vb_runtime chunk_001.rs exists — DEFERRED_GLOBAL, outside this bead's scope
- Fuzzing of ActionContract construction — covered by proptest existing suite
- Concurrency/loom for journal replay — replay is single-threaded sequential
- Performance benchmarking — not a performance-critical hot path
