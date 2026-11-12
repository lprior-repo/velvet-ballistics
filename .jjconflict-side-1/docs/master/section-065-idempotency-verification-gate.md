---
section: 65
title: "Idempotency Verification Gate"
parent: velvet-ballistics-MASTER.md
---

## 65. Idempotency Verification Gate


### Principle

Every retry requires idempotency proof. Unsafe retry is rejected by default.

### Relationship to Existing `Idempotency` Classification

The existing `Idempotency` enum (Section 19) classifies actions for taint propagation and replay behavior:

```rust
// Section 19 — existing, in production
pub enum Idempotency {
    DeterministicPure,      // pure computation, no side effects
    IdempotentExternal,     // external call, safe to repeat
    AtLeastOnceExternal,    // external call, may execute more than once
}
```

Phase 38 extends `ActionContract` with two additional fields. These do NOT replace `Idempotency` — they refine retry decisions that `Idempotency` alone cannot express:

```rust
// Phase 38 — extends ActionContract
pub enum SideEffect {
    Pure,           // no observable side effects (maps to DeterministicPure)
    LocalRead,      // reads local state only
    LocalWrite,     // writes local state
    ExternalRead,   // reads external state
    ExternalWrite,  // writes external state (maps to AtLeastOnceExternal)
    Process,        // spawns or manages a process
    UnsafeShell,    // arbitrary shell execution
}

pub enum RetrySafety {
    Idempotent,                // safe to retry unconditionally
    RequiresIdempotencyKey,    // safe with a valid idempotency key
    NotRetrySafe,              // retry rejected by default
    Unknown,                   // retry rejected
}
```

Mapping rules between the two classification systems:

| `Idempotency` | Implies `SideEffect` | Implies `RetrySafety` |
|----------------|---------------------|----------------------|
| `DeterministicPure` | `Pure` | `Idempotent` |
| `IdempotentExternal` | `ExternalRead` or `ExternalWrite` | `Idempotent` or `RequiresIdempotencyKey` (action-specific) |
| `AtLeastOnceExternal` | `ExternalWrite` | `NotRetrySafe` unless key provided |

Actions declare `Idempotency` at compile time (existing). Phase 38 adds `SideEffect` and `RetrySafety` as additional action contract fields. The verifier uses all three to make retry decisions.

### Idempotency Verification Rules

| Side effect | Default retry rule |
|-------------|-------------------|
| `Pure` | Retry allowed |
| `LocalRead` | Retry allowed if action declares `Idempotent` |
| `ExternalRead` | Retry allowed if action declares `Idempotent` |
| `ExternalWrite` | Requires idempotency proof |
| `LocalWrite` | Requires idempotency proof or explicit policy override |
| `Process` | Retry rejected by default |
| `UnsafeShell` | Retry rejected by default |
| `Unknown` | Retry rejected |

### Idempotency Proof Requirements

For side-effecting actions (`ExternalWrite`, `LocalWrite`), the verifier requires:

```yaml
idempotency:
  required: true
  field: idempotency_key
  default: "$run.id:$step.id"
```

### Idempotency Key Restrictions

Reject idempotency keys that contain:

- `$secrets.*` — secret-tainted values in keys leak information
- `$attempt.number` — unless explicitly allowed by policy
- Random or time functions — keys must be deterministic

Valid key ingredients:

- Run ID
- Workflow digest
- Step ID or step index
- Loop item index
- Gather page cursor hash
- Trigger unique key

### Verification Gate Behavior

The verifier checks each `Do` node in the IR:

1. Look up the action's `SideEffect` and `RetrySafety`.
2. If the action is reachable from a `RetryCheck` node, verify retry is allowed.
3. If retry requires an idempotency key, verify the key is present and well-formed.
4. If retry is not safe, verify no `RetryCheck` can reach this action.
5. Emit `IdempotencyViolation` error if retry safety is not proven.

### Terminology Note

The verifier performs **idempotency attestation**, not idempotency proof. The verifier can require that an idempotency key is present and well-formed, and that the action contract declares idempotent behavior. It cannot prove that calling an external service twice with the same key will not create two side effects — that depends on external behavior. The word "proof" is reserved for properties the verifier can establish from the workflow alone.

---
