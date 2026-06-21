# RP-012: CollectFinish Outputs The Empty Terminal Sentinel For Multi-Page Collections

- **Severity**: High
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/primitives/collect/mod.rs:240`
- **Confidence**: confirmed

## Description
After the last real page has been processed, `collect_next` overwrites the collector slot with an empty terminal page before jumping to `done`. `collect_finish` then copies that empty sentinel to the output, so multi-page collections finish with an empty list instead of a collected result.

## Evidence
When the cursor reaches the captured item count, `collect_next` selects the terminal writer:

```rust
197:     let plan = build_collect_next_plan(run, store, states, collector_slot, current_id)?;
198:     let Some((state, page, page_len)) = plan else {
199:         return write_terminal_collect_page(run, store, states, collector_slot, done);
200:     };
```

The terminal writer stores an empty list in the same collector slot that `collect_finish` later reads:

```rust
240:     let empty_page = Vec::<SlotValue>::new().into_boxed_slice();
241:     let _page_id = write_collected_page(run, store, collector_slot, empty_page)?;
242:     states.remove(run.run_id(), collector_slot);
243:     jump_to(run, done)
```

`collect_finish` then emits whatever is in `collector_slot`:

```rust
255:     let final_value = *run.read_slot(collector_slot)?;
256:     let final_taint = run.read_taint(collector_slot)?;
257:     let out = require_output(output, step)?;
258:     run.write_slot_with_taint(out, final_value, final_taint)?;
```

For any source requiring more than one page, the terminal path replaces the last real page with `[]` before finish.

## Adversarial Check
An empty page can be a reasonable internal sentinel for loop termination, but it cannot also be the collected result unless the collect contract says every multi-page collection returns empty. The single-page path currently finishes with the first page still in the collector slot, while the multi-page path finishes with an empty page, so the behavior is inconsistent across page boundaries.

## Suggested Fix
Separate the loop-control sentinel from the result slot. Keep the final/accumulated result in a distinct slot or state field, and make `collect_finish` emit that value rather than the terminal empty page.
