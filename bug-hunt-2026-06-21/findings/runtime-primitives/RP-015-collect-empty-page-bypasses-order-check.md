# RP-015: CollectNext Lets Empty Collector Pages Bypass Current-Page Validation

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/primitives/collect/mod.rs:191`
- **Confidence**: confirmed

## Description
`collect_next` treats any empty list currently in the collector slot as terminal before it verifies that the observed page is the side table's expected `current_page`. A stale or overwritten empty page can therefore remove active pagination state and jump to `done` without a page-order violation.

## Evidence
The empty-page terminal branch runs before `build_collect_next_plan`, which is where `require_current_page` is called:

```rust
191:     let current_id = expect_list(*run.read_slot(collector_slot)?)?;
192:     let current = store.list(current_id)?;
193:     if current.is_empty() {
194:         states.remove(run.run_id(), collector_slot);
195:         return jump_to(run, done);
196:     }
197:     let plan = build_collect_next_plan(run, store, states, collector_slot, current_id)?;
```

The actual state/page identity check is later:

```rust
220:     let state = states.require_current_page(run.run_id(), collector_slot, current_id)?;
```

While an active collect state exists, real current pages are non-empty because empty sources finish in `collect_start` and terminal empty pages are written only when state is removed.

## Adversarial Check
The empty terminal sentinel is legitimate only after the collector has already reached terminal state. With an active state entry, an empty collector page means the collector slot no longer matches the expected current page and should be classified as duplicate, stale, out-of-order, or invariant violation. The current ordering silently converts that corruption into successful termination.

## Suggested Fix
Validate the observed `current_id` against `CollectStates` before accepting an empty page as terminal. If no state exists, allow idempotent terminal handling; if state exists, require the expected current page and cursor-terminal condition before removing state.
