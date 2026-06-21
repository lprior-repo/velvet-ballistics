# RE-002: `execute_do_without_contract` always fails and allocates a `String` per call

- **Severity**: Medium
- **Category**: perf
- **Location**: `crates/vb_runtime/src/engine/action.rs:77-105`
- **Confidence**: confirmed

## Description

`execute_do_without_contract` unconditionally returns `CapabilityDenied` after heap-allocating a `String` for a synthetic `Capability::new("__contract_required__", action)`. The function exists for the "no contracts registered" case and is invoked from `handle_do` whenever `contracts.is_empty()`. Under a workflow that mistakenly runs `Do` nodes without a populated registry, every dispatch pays for a `String` allocation just to fabricate an error.

## Evidence

`crates/vb_runtime/src/engine/action.rs:90-104`:

```rust
let input_taint = match run.read_taint(input) {
    Ok(t) => t,
    Err(CoreError::SlotUninitialized { .. }) => Taint::Clean,
    Err(e) => return Err(RuntimeEngineError::Core(e)),
};
if input_taint != Taint::Clean {
    return Err(RuntimeEngineError::TaintViolation { step });
}

let required = vb_core::capability::Capability::new("__contract_required__".into(), action);
Err(RuntimeEngineError::Core(EngineError::CapabilityDenied {
    action,
    required,
    granted: granted.clone(),
}))
```

`handle_do` (handlers/action.rs:29-43) selects this path whenever `contracts.is_empty()`:

```rust
if contracts.is_empty() {
    execute_do_without_contract(run, node_id, action, input, seq, granted, retry_policy)
} else {
    execute_do(...)
}
```

Issues:

1. `execute_do_without_contract` is `Err`-always. Its name implies "execute Do when the contract list is empty" but its body is purely rejection. The handler is essentially saying "Do without contracts is forbidden", which is reasonable policy, but the name is misleading.
2. `Capability::new("__contract_required__".into(), action)` allocates a `String` on every call. The capability name is a constant; the allocation is gratuitous. If the path is hot (a buggy workflow with many `Do` nodes and an empty registry), this multiplies the cost.
3. The `granted.clone()` (also heap-backed via `CapabilitySet`) doubles the allocation.
4. The prior `read_taint` and taint check are dead work because the function always returns `Err`. They produce different error variants depending on the slot state, which is the only reason they are not strictly unreachable.

## Adversarial Check

1. *"The path is never hot — production always registers contracts."* — Then the function is dead defensive code. Either way it should not allocate per call. A `&'static str` capability name (or a `Capability::missing_for(action)` constructor) is allocation-free.
2. *"The taint check is a defense-in-depth invariant."* — Agreed, but it is also unreachable when the function returns `Err` unconditionally. The check exists only to choose which error variant to return; it could be expressed more clearly as `Err(RuntimeEngineError::MissingContract { step, ... })` without the taint work.
3. *"Allocation in a cold path is fine."* — Yes, if it is genuinely cold. The `contracts.is_empty()` branch is determined by the runtime configuration, not by the workflow; an entire run could go down this path for every `Do` node. That makes the allocation a per-step cost, not a cold-path cost.

Severity Medium: not a correctness bug, but on a misconfigured run the handler turns into an allocation-per-Do-node error factory.

## Suggested Fix

- Add a `Capability::missing_contract(action)` const constructor (or `&'static str` name) so no allocation is needed.
- Either return a dedicated `RuntimeEngineError::MissingContract { action, step }` variant or short-circuit at the top of `handle_do` before reading taint.
- Rename `execute_do_without_contract` to `reject_do_without_contract` so its name matches its behavior.
