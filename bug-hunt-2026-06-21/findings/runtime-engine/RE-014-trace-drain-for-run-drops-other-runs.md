# RE-014: `TraceRing::drain_for_run` drops non-target events while searching

- **Severity**: Medium
- **Category**: bug
- **Location**: `crates/vb_runtime/src/trace/ring.rs:130-142`
- **Confidence**: confirmed

## Description

`drain_for_run` pops up to `limit` pending events from the shared trace ring and only returns those matching the target run. Non-target events are removed from the pending queue and discarded, so asking for one run's trace can destroy another run's trace evidence.

## Evidence

`crates/vb_runtime/src/trace/ring.rs:130-142`:

```rust
pub fn drain_for_run(&mut self, target: RunId, limit: usize) -> Vec<TraceEvent> {
    let bounded_limit = self.bounded_limit(limit);
    let mut events = Vec::with_capacity(bounded_limit);
    for _ in 0..bounded_limit {
        let Some(event) = self.pop_pending() else {
            return events;
        };
        if event.run_id() == target {
            events.push(event);
        }
    }
    events
}
```

For a pending queue `[run_b_event, run_a_event]`, `drain_for_run(run_a, 1)` pops `run_b_event`, discards it, returns an empty vector, and leaves the target event pending. The non-target event is no longer available to `drain`, `drain_for_run(run_b, ...)`, or any consumer of the pending queue.

## Adversarial Check

If this were intended to drain the shared queue and filter the result, the name and docs would need to say that it discards other runs. The doc says "Drains at most `limit` events for one run," which implies the limit is over returned target events, not over unrelated events consumed while scanning. The separate `snapshot_for_run` method preserves other runs, so this destructive behavior is not an unavoidable part of per-run querying.

## Suggested Fix

Preserve non-target events by temporarily staging them and pushing them back in order, or remove this API and require callers to use `snapshot_for_run` for per-run inspection. If destructive filtering is truly desired, rename it to make the data loss explicit and update the documentation.
