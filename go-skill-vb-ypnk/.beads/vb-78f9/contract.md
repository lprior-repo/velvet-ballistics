# Contract: Action Contract Schema Validation (vb-78f9)

## Bead Overview
**Bead:** vb-78f9 — verifier/runtime: Action contract schema validation
**Phase:** Contract synthesis
**Workspace:** vb-78f9-ws

---

## 1. System Scope

The action contract system spans three layers:

| Layer | Crate | Responsibility |
|-------|-------|----------------|
| Core ABI | `vb_core::action` | Types, taint propagation, idempotency validation, dispatch validation |
| Runtime | `vb_runtime::action` | `ActionRegistry`, `IdempotencyTracker`, table-driven dispatch |
| Engine | `vb_runtime::engine::action` | `execute_do`, `resume_action_outcome`, ticket issuance |
| Verification UI | `vb_ui::verify::action_policy` | Per-Do-node policy analysis and reporting |

The system enforces that every `Do` IR node's action invocation is covered by a registered `ActionContract` before run admission. The contract declares idempotency, side-effect, retry-safety, timeout, capability requirements, and byte limits. Runtime dispatch is purely numeric — no string lookup.

---

## 2. Core Types (Source of Truth in `vb_core::action`)

```rust
// ActionContract — static contract registered at compile time
pub struct ActionContract {
    pub id: ActionId,                      // numeric dispatch key
    pub input_slot_count: u16,             // slots consumed
    pub output_slot_count: u16,            // slots produced
    pub max_input_bytes: u32,              // byte limit for encoded input
    pub max_output_bytes: u32,             // byte limit for encoded output
    pub timeout_ms: u64,                    // wall-clock limit per attempt
    pub idempotency: Idempotency,           // DeterministicPure | IdempotentExternal | AtLeastOnceExternal
    pub side_effect: SideEffect,           // None | Writes | Sends | Creates | Destroys
    pub retry_safety: RetrySafety,         // Safe | KeyRequired | Unsafe
    pub required_capabilities: Box<[Capability]>,
}

// Idempotency — retry and replay behavior contract
pub enum Idempotency {
    DeterministicPure = 0,      // pure computation, no side effects
    IdempotentExternal = 1,     // safe to retry with same idempotency_key
    AtLeastOnceExternal = 2,    // may execute more than once
}

// SideEffect — classifies observable side effects
pub enum SideEffect {
    None = 0,
    Writes = 1,
    Sends = 2,
    Creates = 3,
    Destroys = 4,
}

// RetrySafety — whether retry is safe without an idempotency key
pub enum RetrySafety {
    Safe = 0,           // always safe to retry
    KeyRequired = 1,    // safe only with idempotency key
    Unsafe = 2,         // never safe to retry
}

// ActionTicket — tracks one action invocation across suspension boundaries
pub struct ActionTicket {
    pub run: RunId,
    pub step: StepIdx,
    pub seq: SeqNo,             // monotonic per-run sequence
    pub action: ActionId,
    pub attempt: u16,           // 1-indexed attempt number
    pub idempotency_key: u128,   // deterministic hash of (run, seq, action)
    pub capacity: u16,           // max attempts allowed
}

// ActionOutcome — terminal outcome of one action invocation
pub enum ActionOutcome {
    Ready(ActionOutputReady),    // success with output value
    Suspended(ActionTicket),     // waiting for external completion
    Failed(ActionFailure),       // terminal failure
}

// ActionError — typed dispatch/completion errors
pub enum ActionError {
    UnknownAction { action: ActionId },
    InvalidTicket,
    PayloadTooLarge { max_bytes: u32, actual_bytes: u32 },
    OutputSlotOutOfBounds { slot: u16, max_slots: u16 },
    NonIdempotentReplayBlocked,
    CompletionAlreadyRecorded,
    QueueFull,
    EncodingFailed,
    DispatchFailed,
}

// IdempotencyViolation — key validation failures
pub enum IdempotencyViolation {
    MissingKey(SideEffect),
    SecretInKey(u32),    // slot index
    RandomInKey(u32),
    TimeInKey(u32),
}
```

---

## 3. EARS Preconditions and Postconditions

### 3.1 `ActionRegistry::register`

**Precondition (U):** When `contract.id` is already registered at that slot index.

**Precondition (C):** None (always applicable).

**Postcondition:**
- On success: the `ActionContract` is stored at index `contract.id.get()` and `resolve_compile_time(contract.id)` returns `Ok(&contract)`.
- On error: returns `Err(ActionError::DispatchFailed)` if slot is already occupied; `Err(ActionError::UnknownAction)` if `id.get() >= 65_535`.

---

### 3.2 `ActionRegistry::resolve_compile_time`

**Precondition (U):** When `action` is not registered.

**Postcondition:** Returns `Err(ActionError::UnknownAction { action })`.

**Precondition (C):** When `action` is registered with a matching `ActionId`.

**Postcondition:** Returns `Ok(&ActionContract)` where `contract.id == action`.

---

### 3.3 `ActionRegistry::dispatch`

**Precondition (C):** `input.action` is registered and `contract.id == input.action`.

**Postcondition:** Returns `Ok(ActionOutcome::Suspended(ticket))` where `ticket.idempotency_key == compute_idempotency_key(run, seq, action)`.

**Precondition (E1):** `max_input_bytes == 0 && input_slot_count > 0`.

**Postcondition:** Returns `Err(ActionError::PayloadTooLarge { max_bytes: 0, actual_bytes: 0 })`.

**Precondition (E2):** `input.action` is not registered.

**Postcondition:** Returns `Err(ActionError::UnknownAction { action })`.

---

### 3.4 `execute_do`

**Precondition (C):** Action is resolved from registry and `input_taint == Taint::Clean` OR `idempotency != DeterministicPure`.

**Postcondition:** Returns `Ok(RuntimeSignal::AwaitingAction(ticket))` where `ticket.attempt == 1` and `ticket.idempotency_key == compute_idempotency_key(run.run_id(), seq, action)`.

**Precondition (E1):** `idempotency == DeterministicPure && input_taint != Taint::Clean`.

**Postcondition:** Returns `Err(RuntimeEngineError::TaintViolation { step })`.

**Precondition (E2):** Required capability not in `granted`.

**Postcondition:** Returns `Err(RuntimeEngineError::Core(EngineError::CapabilityDenied { action, required, granted }))`.

**Precondition (E3):** Action not found in registry.

**Postcondition:** Returns `Err(RuntimeEngineError::Action(ActionError::UnknownAction { action }))`.

---

### 3.5 `resume_action_outcome`

**Precondition (C1):** `outcome` is `Ready(ready)` and `ready.output_slot` is within `contract.output_slot_count`.

**Postcondition:** Writes `ready.value` and `ready.taint` to `ready.output_slot`; returns `Ok(RuntimeSignal::Continue)`.

**Precondition (C2):** `outcome` is `Failed(failure)` and `failure.retry_policy == Retryable` and `attempt < capacity`.

**Postcondition:** Returns `Ok(RuntimeSignal::AwaitingAction(retry_ticket))` where `retry_ticket.attempt = attempt + 1` and `retry_ticket.seq = seq + 1`.

**Precondition (E1):** `outcome` is `Failed` with exhausted attempts or `NonRetryable`.

**Postcondition:** Returns `Err(RuntimeEngineError::RetryExhausted { action, attempts })` or `Err(RuntimeEngineError::Core(EngineError::UnsupportedPrimitive))`.

---

### 3.6 `validate_action_dispatch`

**Precondition (C):** `input_slot` is readable and `output_slot < frame.slot_count`.

**Postcondition:** Returns `Ok(())`.

**Precondition (E1):** `input_slot` is out of bounds or uninitialized.

**Postcondition:** Returns `Err(ActionError::DispatchFailed)`.

**Precondition (E2):** `output_slot >= frame.slot_count`.

**Postcondition:** Returns `Err(ActionError::DispatchFailed)`.

---

### 3.7 `propagate_action_taint`

**Postcondition (pure/idempotent):** Returns `input_taint` unchanged (join is identity).

**Postcondition (AtLeastOnce):** Returns `Taint::DerivedFromSecret` if `input_taint != Taint::Clean`; otherwise `Taint::Clean`.

---

### 3.8 `verify_idempotency`

**Precondition (C1):** `side_effect == SideEffect::None`.

**Postcondition:** Returns `Ok(())`.

**Precondition (C2):** `retry_safety == Safe`.

**Postcondition:** Returns `Ok(())`.

**Precondition (C3):** `retry_safety == KeyRequired` and `key_slots` contains only `Taint::Clean` slots.

**Postcondition:** Returns `Ok(())`.

**Precondition (E1):** `retry_safety == Unsafe`.

**Postcondition:** Returns `Err(IdempotencyViolation::MissingKey(side_effect))`.

**Precondition (E2):** `retry_safety == KeyRequired` and any `key_slots` has `Taint::Secret` or `Taint::DerivedFromSecret`.

**Postcondition:** Returns `Err(IdempotencyViolation::SecretInKey(slot_index))`.

---

### 3.9 `IdempotencyTracker::mark_completed`

**Precondition (C):** `ticket.idempotency_key` is not already completed.

**Postcondition:** Records completion; returns `Ok(())`. Evicts oldest entry if at capacity.

**Precondition (E):** `ticket.idempotency_key` already completed.

**Postcondition:** Returns `Err(ActionError::CompletionAlreadyRecorded)`.

---

## 4. Invariants

### 4.1 ActionRegistry Invariants

- **IR1:** After `register(contract)`, `resolve_compile_time(contract.id)` returns `Ok(&contract)` where `contract.id` matches exactly.
- **IR2:** `register` is idempotent: re-registering the same `ActionId` with identical contract returns `Ok(())` only if the slot is `Empty`. If `Registered`, returns `Err(ActionError::DispatchFailed)`.
- **IR3:** `len()` equals `max(action_id.get()) + 1` across all registered actions (sparse array with gap slots).
- **IR4:** `registered_contracts()` returns contracts in strictly ascending `ActionId` order.

### 4.2 ActionTicket Invariants

- **IT1:** `idempotency_key` is deterministic: `compute_idempotency_key(run, seq, action)` called twice with same inputs produces identical `u128`.
- **IT2:** `attempt` is always 1-indexed; never 0.
- **IT3:** `seq` is strictly monotonic within a `RunId`.

### 4.3 Taint Propagation Invariants

- **TT1:** `propagate_action_taint(DeterministicPure, Taint::Secret) == Taint::Secret` (no downgrade).
- **TT2:** `propagate_action_taint(DeterministicPure, Taint::DerivedFromSecret) == Taint::DerivedFromSecret` (no downgrade).
- **TT3:** `propagate_action_taint(AtLeastOnceExternal, Taint::Secret) == Taint::DerivedFromSecret` (upgrades Secret).
- **TT4:** `propagate_action_taint(AtLeastOnceExternal, Taint::Clean) == Taint::Clean`.

### 4.4 IdempotencyTracker Invariants

- **IIT1:** After `mark_completed(ticket)`, `is_completed(&ticket) == true`.
- **IIT2:** After `mark_completed(ticket)`, `mark_completed(ticket)` returns `Err(CompletionAlreadyRecorded)`.
- **IIT3:** At capacity, the oldest entry by insertion order is evicted before inserting a new one.
- **IIT4:** Eviction is FIFO by `order` vector, wrapping at capacity via `cursor`.

### 4.5 ActionPolicyReport Invariants (UI verifier)

- **AP1:** `strict_eligible == true` implies `idempotency_class == DeterministicPure`, `has_timeout == true`, and `issues.is_empty()`.
- **AP2:** `issues` contains `MissingTimeout` iff `timeout_ms == 0` or no contract found.
- **AP3:** `issues` contains `MissingIdempotency` iff no contract found (idempotency is `Unknown`).
- **AP4:** `issues` contains `UnsafeRetry` iff contract exists and `retry_safety == Unsafe`.
- **AP5:** Duplicate `Do` nodes with the same `action` are deduplicated to one report.

---

## 5. Error Taxonomy

### 5.1 ActionError Variants

| Variant | Section 17 Code | Condition |
|---------|-----------------|-----------|
| `UnknownAction { action }` | `REFERENCE_MISSING` | ActionId not registered |
| `InvalidTicket` | — | Ticket does not match in-flight action |
| `PayloadTooLarge { max_bytes, actual_bytes }` | `PAYLOAD_TOO_LARGE` | Encoded input exceeds `max_input_bytes` |
| `OutputSlotOutOfBounds { slot, max_slots }` | — | Output slot index >= declared output count |
| `NonIdempotentReplayBlocked` | — | Non-idempotent action replay attempted |
| `CompletionAlreadyRecorded` | — | Duplicate completion for same idempotency_key |
| `QueueFull` | `QUEUE_FULL` | Action dispatch queue at capacity |
| `EncodingFailed` | `ACTION_FAILED` | Postcard encoding of output failed |
| `DispatchFailed` | `ACTION_FAILED` | Dispatch validation failed |

### 5.2 IdempotencyViolation Variants

| Variant | Trigger |
|---------|---------|
| `MissingKey(SideEffect)` | `RetrySafety::Unsafe` or `KeyRequired` with empty key_slots |
| `SecretInKey(u32)` | Idempotency key ingredient slot has `Taint::Secret` or `Taint::DerivedFromSecret` |
| `RandomInKey(u32)` | Key ingredient slot has random-generated value (scaffold) |
| `TimeInKey(u32)` | Key ingredient slot has time-dependent value (scaffold) |

### 5.3 RuntimeEngineError Variants (action-related)

| Variant | Trigger |
|---------|---------|
| `RuntimeEngineError::TaintViolation { step }` | `DeterministicPure` action with tainted input |
| `RuntimeEngineError::Action(ActionError::UnknownAction)` | Action not found in registry |
| `RuntimeEngineError::RetryExhausted { action, attempts }` | All retry attempts exhausted |
| `RuntimeEngineError::Core(EngineError::CapabilityDenied { action, required, granted })` | Capability check failed |

---

## 6. Acceptance Tests

### 6.1 Happy Path

| ID | Test | Contract |
|----|------|----------|
| HA1 | `execute_do` with clean input and registered `DeterministicPure` action | Returns `AwaitingAction(ticket)` with `attempt=1` and valid idempotency key |
| HA2 | `execute_do` with `AtLeastOnceExternal` and `Taint::Secret` input | Returns `AwaitingAction` with `Taint::DerivedFromSecret` propagated |
| HA3 | `ActionRegistry::dispatch` on registered action | Returns `Suspended(ticket)` with ticket fields from input |
| HA4 | `IdempotencyTracker::mark_completed` on new key | Returns `Ok(())`, `is_completed == true` |
| HA5 | `resume_action_outcome(Ready)` writes output slot | Output slot contains `ready.value` and `ready.taint` |
| HA6 | `resume_action_outcome(Failed/Retryable)` below capacity | Returns `AwaitingAction` with incremented attempt and seq |
| HA7 | `verify_idempotency(Safe)` always passes | Returns `Ok(())` regardless of key_slots |
| HA8 | `analyze_action_policies` on fully-covered workflow | All reports have `strict_eligible=true` |
| HA9 | `compute_idempotency_key` is deterministic | Same inputs produce identical `u128` |
| HA10 | `propagate_action_taint(DeterministicPure, Clean)` | Returns `Clean` |

### 6.2 Error Path

| ID | Test | Contract |
|----|------|----------|
| EA1 | `execute_do` with `Taint::Secret` input on `DeterministicPure` | Returns `Err(TaintViolation)` |
| EA2 | `ActionRegistry::dispatch` unknown action | Returns `Err(UnknownAction)` |
| EA3 | `IdempotencyTracker::mark_completed` duplicate key | Returns `Err(CompletionAlreadyRecorded)` |
| EA4 | `verify_idempotency(Unsafe)` always fails | Returns `Err(MissingKey(side_effect))` |
| EA5 | `verify_idempotency(KeyRequired)` with secret key slot | Returns `Err(SecretInKey(slot))` |
| EA6 | `resume_action_outcome(Failed/NonRetryable)` | Returns `Err(UnsupportedPrimitive)` |
| EA7 | `resume_action_outcome(Failed/Retryable)` at capacity | Returns `Err(RetryExhausted)` |
| EA8 | `validate_action_dispatch` with uninitialized input slot | Returns `Err(DispatchFailed)` |
| EA9 | `validate_action_dispatch` with out-of-bounds output slot | Returns `Err(DispatchFailed)` |
| EA10 | `dispatch` with `max_input_bytes=0` and `input_slot_count>0` | Returns `Err(PayloadTooLarge)` |
| EA11 | `ActionRegistry::register` duplicate ActionId | Returns `Err(DispatchFailed)` |
| EA12 | `ActionRegistry::register` with `id >= 65535` | Returns `Err(UnknownAction)` |
| EA13 | `resume_action_outcome` with unknown `output_slot` | Propagates `SlotOutOfBounds` error from `write_slot_with_taint` |
| EA14 | `analyze_action_policies` on Do node with no contract | Report has `MissingTimeout`, `MissingIdempotency`, not `UnsafeRetry` |
| EA15 | `analyze_action_policies` with `RetrySafety::Unsafe` contract | Report has `UnsafeRetry` issue |
| EA16 | `execute_do_without_contract` with `Taint::Secret` input | Returns `Err(TaintViolation)` |

---

## 7. File Reads Performed Before Writing

The following files were read to synthesize this contract:

| File | Purpose |
|------|---------|
| `velvet-ballistics-MASTER.md` | Naming contract, Rust rules, Section 17/19 action ABI specs |
| `crates/vb_core/src/action.rs` | Core ABI types, `ActionContract`, `ActionTicket`, `ActionError`, taint propagation, idempotency validation |
| `crates/vb_runtime/src/action.rs` | `ActionRegistry`, `IdempotencyTracker`, dispatch, registration |
| `crates/vb_runtime/src/engine/action.rs` | `execute_do`, `resume_action_outcome`, `compute_idempotency_key`, capability checks |
| `crates/vb_ui/src/verify/action_policy.rs` | `analyze_action_policies`, `ActionPolicyReport`, `IdempotencyClass`, `PolicyIssue` |

---

## 8. Constraints from Master Contract

- `#![forbid(unsafe_code)]` — no `unsafe` blocks in first-party code
- No `.unwrap()`, `.expect()`, `panic!`, `dbg!`
- No unchecked indexing — all slot access via checked `read_slot`/`write_slot`
- All action errors are typed `ActionError` variants with stable codes
- `FiniteF64` reject rule applies to any numeric action inputs/outputs
- Binary IPC frame format uses `VBLT` magic `0x56424C54`
- `ActionContract` is part of the stable binary contract — fields must not change layout without migration
