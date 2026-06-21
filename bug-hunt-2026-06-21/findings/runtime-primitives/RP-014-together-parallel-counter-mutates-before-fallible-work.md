# RP-014: Together Mutates Parallel In-Flight Counters Before Fallible Work Commits

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/primitives/together.rs:37`
- **Confidence**: confirmed

## Description
`together_start` increments the parallel in-flight counter before output validation, allocation, slot writes, and the branch jump can fail. `together_join` similarly decrements the counter before validating and writing the final output. Errors after those mutations leave the frame's parallel bound accounting corrupted.

## Evidence
Start increments the counter, then performs multiple fallible operations:

```rust
37:     run.add_parallel_in_flight(count)?;
38:     let iter_output = require_output(output, run.pc())?;
39:     let state = store.insert_list(Vec::<SlotValue>::new().into_boxed_slice())?;
40:     run.write_slot(iter_output, SlotValue::List(state))?;
```

Join decrements first, then performs fallible output resolution, reads, appends, and writes:

```rust
91:     run.sub_parallel_in_flight(branch_count.get())?;
92:     let out = require_output(output, step)?;
93:     // Read the accumulator list built by together_branch invocations.
94:     let acc_value = *run.read_slot(accumulator)?;
...
115:     run.write_slot_with_taint(out, final_list, combined_taint)?;
```

`require_output`, store insertion, slot reads, accumulator append, and slot writes all return errors, but there is no rollback guard.

## Adversarial Check
This is not limited to impossible allocation failure. A malformed compiled step with a missing output slot causes `require_output` to fail immediately after the counter mutation. If the failed `RunFrame` is persisted, retried, inspected, or used to mark the workflow failed, the resource accounting no longer describes the actual active branches.

## Suggested Fix
Make the counter update the final committed mutation, or protect it with a rollback guard. For `together_start`, validate the output and prepare the accumulator before incrementing. For `together_join`, perform reads/appends/final writes first, then decrement only after the join result is committed.
