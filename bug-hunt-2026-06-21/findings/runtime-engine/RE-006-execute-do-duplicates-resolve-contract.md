# RE-006: `execute_do` duplicates `resolve_contract` inline instead of calling it

- **Severity**: Info
- **Category**: simplification
- **Location**: `crates/vb_runtime/src/engine/action.rs:31-43`
- **Confidence**: confirmed

## Description

`execute_do` resolves an action contract by indexing into `registry_contracts` and filtering by id, which is the exact logic of the `resolve_contract` helper exported two functions below. The duplicated code can drift out of sync with the canonical helper.

## Evidence

`crates/vb_runtime/src/engine/action.rs:31-43` (inline in `execute_do`):

```rust
let action_index = usize::from(action.get());
let resolved = registry_contracts
    .get(action_index)
    .filter(|c| c.id == action)
    .ok_or(ActionError::UnknownAction { action })?;
```

`crates/vb_runtime/src/engine/action.rs:211-221` (the helper):

```rust
pub fn resolve_contract(
    action: ActionId,
    contracts: &[ActionContract],
) -> RuntimeEngineResult<&ActionContract> {
    let index = usize::from(action.get());
    contracts
        .get(index)
        .filter(|c| c.id == action)
        .ok_or(ActionError::UnknownAction { action })
        .map_err(RuntimeEngineError::Action)
}
```

The two implementations are byte-for-byte identical in their resolution logic. The helper is also `pub` exported via `engine/mod.rs:38-40`, so it is part of the public API. There is no reason for the inline copy.

## Adversarial Check

`execute_do` takes `_contract: &ActionContract` and `registry_contracts: &[ActionContract]`. The handler `handle_do` (handlers/action.rs:36-43) calls `resolve_contract(action, contracts)?` and *also* passes the same `contracts` slice to `execute_do` as `registry_contracts`. So the caller already resolved via the helper — and then `execute_do` resolves again inline. Two resolutions for one dispatch.

This is purely a simplification / consistency nit, no behavior impact.

## Suggested Fix

In `handle_do` pick one path:

```rust
pub(crate) fn handle_do(...) -> RuntimeEngineResult<RuntimeSignal> {
    let seq = SeqNo::new(run.executed());
    if contracts.is_empty() {
        return execute_do_without_contract(run, node_id, action, input, seq, granted, retry_policy);
    }
    let contract = resolve_contract(action, contracts)?;
    execute_do(run, node_id, action, input, seq, contract, contracts, granted, retry_policy)
}
```

Then `execute_do` should not re-resolve; it should trust the `contract: &ActionContract` parameter it was already given and drop the `registry_contracts` slice. The existing parameter is currently named `_contract` and unused — that is the smoking gun: the caller already paid for resolution, the callee ignores it and re-resolves.
