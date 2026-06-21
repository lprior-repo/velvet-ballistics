# RP-011: CollectStart Skips Body For Non-Empty Single-Page Collections

- **Severity**: High
- **Category**: bug
- **Location**: `crates/vb_runtime/src/primitives/collect/mod.rs:147`
- **Confidence**: confirmed

## Description
`collect_start` writes the first page but jumps directly to `done` when that page exhausts the source list. Any non-empty collection with `len <= page_size` therefore never executes the collect body for its only page.

## Evidence
`collect_start` writes a non-empty first page, then delegates terminal handling:

```rust
64:     let page = core::mem::take(&mut plan.page);
65:     let current_page =
66:         write_collected_page_with_taint(run, store, plan.collector, page, plan.source_taint)?;
67:     finish_collect_start_page(run, states, plan, current_page, time_limit_ms, body, done)
```

The terminal-page branch removes state and jumps to `done` instead of `body`:

```rust
147:     if plan.page_len >= plan.item_count {
148:         states.remove(run.run_id(), plan.collector);
149:         return jump_to(run, done);
150:     }
151:     upsert_started_collect(run, states, &plan, current_page, time_limit_ms)?;
152:     jump_to(run, body)
```

For any non-empty source where `item_count <= page_size`, `page_len == item_count`, so the only page is never processed by the body.

## Adversarial Check
The empty-source path is separate at lines 54-62, so this is not about intentionally skipping work for an empty list. The `collect_page` function says it dispatches the current page to the body for processing, and multi-page collections do execute `body` for earlier pages. Treating the only/final page as already processed creates input-size-dependent behavior rather than a deliberate terminal optimization.

## Suggested Fix
For every non-empty page, jump to the body. Persist enough state for the final page, such as a terminal-after-body flag or a state with `cursor == item_count`, so `collect_next` can terminate only after the body has processed that page.
