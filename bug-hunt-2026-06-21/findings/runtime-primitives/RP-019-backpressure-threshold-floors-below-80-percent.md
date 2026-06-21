# RP-019: Backpressure Threshold Floors Below The Documented 80 Percent Capacity

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/action_queue/queue.rs:150`
- **Confidence**: confirmed

## Description
The action completion queue documents warnings at 80% capacity, but `backpressure_threshold` uses integer floor division. Many capacities warn substantially below 80%, producing noisy and misleading backpressure signals.

## Evidence
The threshold computes `capacity * 8 / 10` and floors the result:

```rust
150: fn backpressure_threshold(capacity: ActionQueueCapacity) -> usize {
151:     match capacity
152:         .get()
153:         .checked_mul(8)
154:         .and_then(|scaled| scaled.checked_div(10))
155:     {
156:         Some(threshold) => threshold.max(1),
157:         None => capacity.get(),
158:     }
159: }
```

Examples: capacity 2 warns at depth 1 (50%), capacity 3 warns at depth 2 (66%), capacity 6 warns at depth 4 (66%), and capacity 9 warns at depth 7 (77%).

## Adversarial Check
Early warning could be a deliberate conservative policy, but the module contract says warnings are emitted when the queue reaches 80% capacity, not when it approaches an implementation-specific lower floor. The enqueue path uses `depth >= threshold`, so the floored threshold directly changes externally observable warning behavior.

## Suggested Fix
Use checked ceiling arithmetic for the threshold, for example `ceil(capacity * 8 / 10)`, while preserving the minimum threshold of 1 and the existing overflow fallback.
