# RP-002: `together_join` silently drops last branch result when it is `List` or `Null`

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/primitives/together.rs:99-105`
- **Confidence**: confirmed

## Description

`together_join` is responsible for appending the final branch's body result to the accumulator (intermediate branches are appended by the *next* `together_branch` call, so the last branch's result is only ever in the output slot when `together_join` runs). When that last result is a `SlotValue::List(_)` or `SlotValue::Null`, the join arm discards it and returns the prior accumulator unchanged, losing data.

## Evidence

`crates/vb_runtime/src/primitives/together.rs:94-111`:

```rust
let acc_value = *run.read_slot(accumulator)?;
let final_list = match acc_value {
    SlotValue::List(id) => {
        // Append the last branch body result if it's not already a list.
        let last_result = *run.read_slot(out)?;
        match last_result {
            SlotValue::List(_) | SlotValue::Null => SlotValue::List(id),   // <-- drop
            other => {
                append_to_accumulator(run, store, accumulator, other, out)?;
                *run.read_slot(accumulator)?
            }
        }
    }
    _ => acc_value,
};
```

Control flow established by `together_branch` (together.rs:62-78): for branches after the first, the previous branch's result is read out of `branch_output` and appended; then control jumps to the next branch entry. So when the **last** branch body completes, nobody appends its result — that append is the responsibility of `together_join`. The accumulator at that moment contains `[result_1, ..., result_{N-1}]` and the output slot contains `result_N`.

Concrete reproduction for 3 branches whose bodies write `I64(1)`, `I64(2)`, `List([I64(3)])`:

| branch body writes | after `together_branch` accumulator | output slot |
|--------------------|--------------------------------------|-------------|
| I64(1)             | `[]` (first branch never appends)    | I64(1)      |
| I64(2)             | `[I64(1)]`                           | I64(2)      |
| List([I64(3)])     | `[I64(1), I64(2)]`                   | List([3])   |

`together_join` then sees `last_result = List([I64(3)])`, takes the drop arm, and `final_list` becomes the accumulator `[I64(1), I64(2)]`. The third branch's contribution is silently lost. The same applies to `Null`: a branch that intentionally returns `Null` is dropped, which is indistinguishable from "branch never ran".

The comment "Append the last branch body result if it's not already a list" suggests the author believed a List result means "the branch already contributed to the accumulator", but `append_to_accumulator` only ever runs from `together_branch` for **earlier** branches; the last branch's body never touches the accumulator.

## Adversarial Check

Considered counter-arguments:

1. *"Last branch is special — by convention it returns the final value, not a contribution."* — Refuted by the structure of `together_branch` which unconditionally appends *previous* results regardless of type. There is no documented asymmetry between branches.
2. *"Branches never return List values."* — `SlotValue::List` is a first-class runtime value (collect, for_each, reduce all produce List). A together branch that runs a sub-collect absolutely can return a List; nothing in the workflow validator (`vb_core::workflow::validation`) restricts branch return types.
3. *"The output slot is separate from the accumulator, so the result is preserved."* — `together_join` *overwrites* the output slot with `final_list` (together.rs:115). After the join, the original last-branch List value is gone.

Severity is High: this is silent data loss in a primitive that exists specifically to compose branch results.

## Suggested Fix

Always append the last branch result unless the caller can explicitly opt out. If the design truly needs a "no contribution" sentinel, reserve `Null` for that purpose but **still append Lists**. Minimal patch:

```rust
match last_result {
    SlotValue::Null => SlotValue::List(id),
    other => {
        append_to_accumulator(run, store, accumulator, other, out)?;
        *run.read_slot(accumulator)?
    }
}
```

Better: unify the "append last result" path with `together_branch`'s append path so there is exactly one place that decides what gets into the accumulator.
