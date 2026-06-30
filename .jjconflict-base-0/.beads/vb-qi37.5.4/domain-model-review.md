# Domain Model Review — vb-qi37.5.4

## Bead
- **bead_id**: vb-qi37.5.4
- **phase**: State 3 (Contract)
- **workspace**: /home/lewis/src/vb-qi37-5-4

---

## DDD Analysis: Idempotency Gate Domain Model

### 1. Idempotency Enum — Illegal State Prevention

**Current shape**:
```rust
pub enum Idempotency { DeterministicPure = 0, IdempotentExternal = 1, AtLeastOnceExternal = 2 }
```

**Assessment**: Three-variant enum is appropriate. Each variant represents a distinct delivery semantics class. The numeric discriminants are implementation details and do not introduce illegal states.

**Scott Wlaschin check**:
- ✅ No primitive obsession — Idempotency is a semantic enum, not a u8
- ✅ No bool flags — behavior branching is via match, not bool parameters
- ✅ No Option-as-state-machine — each variant is semantically distinct
- ✅ Domain errors are explicit variants — IdempotencyViolation covers all failure modes

**Potential concern**: The three variants do not prevent an illegal combination on their own — the illegal combination emerges from the interaction of `Idempotency × SideEffect × RetrySafety`. The decision table is the correct enforcement mechanism.

**Verdict**: Type design is sound. Illegal states are prevented by the decision table function, not by type construction.

---

### 2. IdempotencyViolation Error Lattice

**Current shape**:
```rust
pub enum IdempotencyViolation {
    MissingKey(SideEffect),
    SecretInKey(u32),
    RandomInKey(u32),
    TimeInKey(u32),
}
```

**Assessment**: Error lattice is well-structured:
- `MissingKey` carries the `SideEffect` that triggered the requirement — allows caller to understand which operation needs a key
- `SecretInKey(u32)`, `RandomInKey(u32)`, `TimeInKey(u32)` each carry the slot index — allows exact pinpointing of the offending key slot
- Four variants are mutually exclusive and collectively exhaustive for the runtime gate

**Scott Wlaschin check**:
- ✅ Each variant carries exactly the data needed for diagnostics
- ✅ No variant is "missing" — all four runtime failure modes are covered
- ✅ No boolean soup — slot taint category is an enum variant, not a bool flag

**Invariant enforcement**: `verify_idempotency` must return exactly one variant on failure. INV-004 requires proof that the first failing taint check short-circuits and does not accumulate multiple violations.

---

### 3. ActionContract — Parse-Don't-Validate Boundary

**Current shape**:
```rust
pub struct ActionContract {
    pub id: u32,
    pub input_slot_count: u32,
    pub output_slot_count: u32,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub timeout_ms: u64,
    pub idempotency: Idempotency,
    pub side_effect: SideEffect,
    pub retry_safety: RetrySafety,
    pub required_capabilities: SlotIdx,
}
```

**Assessment**: `ActionContract` is a raw input/construction type. The idempotency contract validation happens at the boundary:

1. **Boundary layer (vb_validate)**: `is_statically_idempotent_contract` parses and validates the contract tuple, returning `Err` for illegal combinations.
2. **Core (vb_core)**: Functions accept `ActionContract` and assume it has already passed static validation.
3. **Runtime layer (vb_core)**: `verify_idempotency` validates key slots at execution time.

**Scott Wlaschin check**:
- ✅ Parse-don't-validate boundary is explicit — `is_statically_idempotent_contract` is the parsing function
- ✅ Core does not re-validate static contract properties — acceptance is trusted after static gate
- ✅ Runtime key validation is a second boundary — separate from static contract validation

**Contract parity concern**: `check_idempotency_gates` in vb_compile implements the same decision table logic as `is_statically_idempotent_contract` in vb_validate. If these diverge, illegal states become representable at compile time. This is the highest-risk invariant in the bead scope.

---

### 4. Decision Table — Explicit State Transitions vs Boolean Soup

**Decision table truth table**:

| side_effect | retry_safety | idempotency | result |
|---|---|---|---|
| None | any | any | Ok |
| ≠ None | Unsafe | any | Err(SideEffectingRetryUnsafe) |
| ≠ None | any | AtLeastOnceExternal | Err(SideEffectingAtLeastOnceExternal) |
| ≠ None | any | DeterministicPure | Err(SideEffectingDeterministicPure) |
| ≠ None | Safe\|KeyRequired | IdempotentExternal | Ok |

**Assessment**: The decision table is a pure function `f(side_effect, retry_safety, idempotency) → Result`. This is not a boolean soup — each branch is an explicit, named error case. The table can be read as a state machine with 5 branches.

**Scott Wlaschin check**:
- ✅ No boolean flags control branching — match on enum variants
- ✅ No nullable fields encoding lifecycle — all fields are non-optional
- ✅ Explicit error variants for each rejection case
- ✅ Confluence property: same input always produces same output (INV-003)

**Implementation note**: The 5-branch `match` in `is_statically_idempotent_contract` must be exhaustive. The compiler enforces 3-variant `Idempotency` × 5-variant `SideEffect` × 3-variant `RetrySafety` = 45 possible combinations, but only 5 are valid outcomes. Verus proof should cover all 45 combinations to verify confluence.

---

### 5. Type-State Analysis of verify_idempotency Runtime Path

**Runtime path**:
```
verify_idempotency(contract, key_slots, frame)
  └─▶ MissingKey check ──── if key_slots.is_empty() and contract.idempotency == IdempotentExternal
       │
       └─▶ validate_idempotency_key_ingredients(key_slots, frame)
            ├─▶ for each slot_idx in key_slots:
            │    ├─▶ SecretTaint check ──▶ Err(SecretInKey(slot_idx))
            │    ├─▶ Random check ──────▶ Err(RandomInKey(slot_idx))
            │    └─▶ TimeDependent check ─▶ Err(TimeInKey(slot_idx))
            └─▶ all clean ──▶ Ok(())
```

**Assessment**: The runtime path is a linear traversal with early exit on first taint detection. This is a straightforward state machine with deterministic transitions.

**Scott Wlaschin check**:
- ✅ Each taint check is a distinct branch — no boolean flag accumulation
- ✅ Early exit on first error — INV-004 ensures single-variant return
- ✅ Slot index is carried in error variants — exact diagnostic location
- ✅ No hidden state dependency between checks — each check is independent

**Taint propagation**: The taint of a slot value is determined by the computation that produced it (e.g., `Random`, `SecretTaint`, `TimeDependent`). The `RunFrame` exposes slot values and their taint metadata. Key slots are validated by reading their taint bits, not by re-executing the computation.

---

## Summary: Type Integrity Gate

| Check | Status | Notes |
|---|---|---|
| No primitive obsession in domain interfaces | ✅ PASS | Idempotency, SideEffect, RetrySafety are semantic enums |
| No bool control flags in domain signatures | ✅ PASS | Branching via match on enums |
| No Option-as-state encoding | ✅ PASS | No Option fields encoding lifecycle |
| All raw input parsed before core | ✅ PASS | is_statically_idempotency_contract is the parse boundary |
| All domain errors are enumerable | ✅ PASS | 3 static variants + 4 runtime variants |
| State machine is explicit | ✅ PASS | 5-branch decision table as pure function |
| Transitions are one-way where required | ✅ PASS | No stateful state machines — pure functions |
| Illegal transitions are unconstructable | ⚠️ WATCH | Contract parity between vb_compile and vb_validate is the critical invariant |
| Pattern matches are exhaustive | ✅ PASS | Compiler enforces exhaustiveness on 3-variant enums |

---

## Critical Invariant: Compile/Runtime Parity

The highest-risk item in this bead scope is `check_idempotency_gates` (vb_compile) and `is_statically_idempotent_contract` (vb_validate) implementing identical decision table logic in separate code paths.

**Threat**: If one function is updated and the other is not, the compile-time gate and the validation gate will accept/reject different sets of contracts. A contract that passes compile-time checks could fail at runtime validation.

**Required proof**: Kani harness that feeds the same 45 combinations to both functions and verifies they produce identical results.

---

## Open Items for Proof Planning

1. Decision table confluence — Kani covers all 45 combinations
2. Parity between vb_compile and vb_validate gates — Kani cross-function comparison
3. verify_idempotency single-variant return — Verus invariant proof
4. Key slot taint propagation — Miri for slot index arithmetic safety
5. 3 static rejection variants are mutually exclusive — Verus proof
