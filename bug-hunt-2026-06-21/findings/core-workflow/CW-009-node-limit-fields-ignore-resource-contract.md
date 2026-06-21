# CW-009: Node limit fields ignore the declared resource contract

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_core/src/workflow/validation/nodes/kinds.rs:48-86`
- **Confidence**: confirmed

## Description

The resource contract declares limits for retry attempts, branch fanout, and collection sizes, but node validation discards or minimally checks the corresponding node fields. Workflows can claim a restrictive contract while carrying loop, collect, retry, or fanout values that exceed it.

## Evidence

```rust
// resource_contract.rs:33
pub max_retry_attempts: u16,
// resource_contract.rs:35
pub max_fanout: u16,
// resource_contract.rs:37
pub max_collect_items: u32,
```

```rust
// kinds.rs:48
CompiledNodeKind::ForEachStart {
    input,
    item_slot,
    limit: _,
    body,
    done,
} => validate_for_each_start(*input, *item_slot, *body, *done, parts),
```

```rust
// kinds.rs:80
CompiledNodeKind::CollectStart {
    source,
    limit: _,
    page_size: _,
    body,
    done,
} => validate_slot_and_steps(*source, *body, *done, parts),
```

```rust
// common.rs:155
pub(crate) fn validate_repeat_start(
    max_attempts: u16,
    body: StepIdx,
    done: StepIdx,
    parts: &WorkflowParts,
) -> Result<(), WorkflowError> {
    validate_nonzero_u16(max_attempts, "max_retry_attempts")?;
    validate_two_steps(body, done, parts)
}
```

Branch tables only require at least one route:

```rust
// branch_tables.rs:16
if branch_count == 0 && otherwise.is_none() {
    Err(WorkflowError::EmptyBranchTable)
} else {
    Ok(())
}
```

`validate_resource_contract` validates primary table counts, expression stack, and `max_transitions_per_tick`; it does not compare these node-local limits to the contract.

## Adversarial Check

This is not just duplicate budget enforcement. The contract fields are explicit admission promises, while the shown validators either ignore the IR values (`limit: _`, `page_size: _`) or only reject zero. A workflow with `resource_contract.max_retry_attempts = 3` and `RepeatStart { max_attempts: 65_535, ... }` is not rejected by the node/resource validators shown here. A `CollectStart` with `page_size = 0` also passes this validation path even though zero-sized pages cannot make forward collection progress unless a separate sentinel semantics exists.

## Suggested Fix

Thread `ResourceContract` into these node validators and enforce each field at the boundary: non-zero and `<= max_retry_attempts` for retry/repeat counts, non-zero and `<= max_collect_items` for collection/for-each limits and page sizes, and branch counts `<= max_fanout` for choose/together variants. If any zero value is intentionally a sentinel, replace the raw integer with an enum/newtype that makes that state explicit.
