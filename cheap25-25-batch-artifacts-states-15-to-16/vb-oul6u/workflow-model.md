# Workflow Model — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Workflow: `Runtime::collect_metrics` per-shard metric emission

This workflow describes the per-shard iteration of `Runtime::collect_metrics(&self)`. The bead affects only the inner branch that computes `trace_ring_fill_pct`. The full loop is included for context.

## Legal States

| State | Predicate |
|-------|-----------|
| `NotStarted` | The per-shard iteration has not yet begun (out of scope for this bead; covered by the for-loop prelude at `runtime.rs:569`). |
| `GatheringCounters` | The shard's `Counters::snapshot()` and the six sibling metrics (`active_runs`, `queue_depth`, `queue_remaining`, `pending_timers`, `frame_pool_free`, `frame_pool_total`) have been computed via `u32::try_from(...).unwrap_or(u32::MAX)` (`runtime.rs:570-577`). |
| `GatheringTraceRingMetrics` | `trace_capacity` and `trace_len` have been read from `shard.trace_ring()` (`runtime.rs:578-579`). This is the state the bead modifies. |
| `ComputingFillRatio` | The trace ring fill ratio is being computed. Pre-bead: via `(trace_len as f32) / (trace_capacity as f32)` under a local `#[allow(clippy::as_conversions)]`. Post-bead: via `f32::from(trace_len_u32) / f32::from(trace_capacity_u32)` where `trace_*_u32 = u32::try_from(trace_*).unwrap_or(0)`. |
| `SentinelFillRatio` | `trace_capacity == 0` — the metric returns `0.0` without computing a ratio. (Pre- and post-bead: identical behaviour.) |
| `EmittingShardSnapshot` | The `ShardMetricsSnapshot { ..., trace_ring_fill_pct, counters }` value is pushed into `shards: Vec<ShardMetricsSnapshot>` (`runtime.rs:597-607`). |
| `AggregatingFleetTotals` | The five fleet-wide sums (`runs_active`, `runs_waiting`, `runs_failed_total`, `runs_finished_total`, `steps_total`) are updated via `saturating_add` (`runtime.rs:590-594`). |
| `Finished` | The loop has terminated; `RuntimeMetricsSnapshot { shards, runs_active, ..., steps_total }` is returned (`runtime.rs:610-617`). |

## State Transitions

```
NotStarted
   │ for (index, shard) in self.shards.iter().enumerate()
   ▼
GatheringCounters
   │ read counters, six sibling metrics
   ▼
GatheringTraceRingMetrics
   │ read trace_capacity : usize, trace_len : usize
   ▼
   ├─ trace_capacity == 0 ──► SentinelFillRatio ──► trace_ring_fill_pct = 0.0 ──► EmittingShardSnapshot
   │
   └─ trace_capacity > 0 ──► ComputingFillRatio
                                  │ narrow to u32
                                  │ promote to f32
                                  │ divide
                                  ▼
                              EmittingShardSnapshot
                                  │
                                  ▼
                              AggregatingFleetTotals
                                  │ saturating_add
                                  ▼
                              Finished (after loop)
```

## Guards

| Guard | Location (post-bead) | Predicate | Behaviour on violation |
|-------|----------------------|-----------|------------------------|
| Zero-denominator guard | `runtime.rs:580` (branch selector) | `trace_capacity > 0` | If `trace_capacity == 0`, returns `0.0`. Never divides by zero. |
| Bounded-narrowing guard | `u32::try_from(...).unwrap_or(0)` | implicit: capacity ≤ documented max `4096`; pending ≤ capacity | If the capacity invariant were violated at runtime, falls back to `0` for both sides, producing `0.0 / 0.0 = NaN` which would silently corrupt the metric. This is acceptable because the invariant is enforced upstream (`TraceRing::new` clamps `capacity.max(1)` and the configuration cap is documented). The bead must document this fallback choice in the source comment. |

## Commands

| Command | Effect | Lane |
|---------|--------|------|
| `Runtime::collect_metrics(&self)` | Returns a fresh `RuntimeMetricsSnapshot` | Pure read; no mutation |

## Events

The fix does not introduce new events. The existing `RuntimeMetricsSnapshot` is the only output event.

## Terminal Outcomes

| Outcome | Description | Publicly Observable |
|---------|-------------|---------------------|
| `Ok(snapshot)` | A fully populated `RuntimeMetricsSnapshot` whose `shards[i].trace_ring_fill_pct` is in `[0.0, 100.0]` for the documented capacity range | Yes, identical to pre-bead observable output |

## Retries / Cancellation

- No retries, no cancellation, no timeout. `collect_metrics` is a synchronous pure read.

## Idempotence

- `collect_metrics` is idempotent over `&self` snapshots. Calling it twice with no intervening mutations returns structurally identical snapshots (modulo `Vec` allocation). The bead preserves this property.

## Concurrency

- `Runtime::collect_metrics` holds `&self`; it does not mutate shared state. No new concurrency hazards are introduced. The existing `Runtime` mutability rules are unchanged (out of scope).

## Hazards (workflow-level)

| Hazard | Description | Mitigation |
|--------|-------------|-----------|
| Silent NaN on invariant violation | If a future change broke `TraceRing::new`'s `capacity.max(1)` clamp, the `unwrap_or(0)` fallback would produce `0.0 / 0.0 = NaN`. | The replacement MUST NOT relax the upstream invariant. The fallback value `0` (not `u32::MAX`) is the closest to a sentinel under that hypothetical. A source comment should record this. |
| Lint re-introduction | A future patch could re-add `#[allow(clippy::as_conversions)]` on a sibling line. | The `forbidden-scan` AST scanner (`docs/master/section-041-forbidden-scan-contract.md:26`) targets this exact class of suppression. Downstream black-hat review must run it. |
| Field-type drift | A future patch could widen `trace_ring_fill_pct` to `f64` and break the IPC wire format. | `vb_ipc/src/metrics.rs:37` re-declares the field as `f32`. Any drift must be reflected in both declarations and the IPC serializer. Out of scope for this bead. |
| Ratio saturation by sentinel | If `unwrap_or(0)` were ever changed to `unwrap_or(u32::MAX)`, the denominator could become huge, producing a `ratio ≈ 0.0` that is unobservable from the public field but defeats the sentinel intent. | Pin the fallback in `domain-model.md` and `type-contracts.md`; the source comment must restate `unwrap_or(0)`. |

## Workflow Invariants

- WF-INV-001: For every `trace_capacity > 0`, the value of `trace_ring_fill_pct` equals `f32::from(trace_len_u32) / f32::from(trace_capacity_u32) * 100.0` where `trace_*_u32 = u32::try_from(trace_*).unwrap_or(0)`.
- WF-INV-002: For every `trace_capacity == 0`, the value of `trace_ring_fill_pct` equals `0.0`.
- WF-INV-003: The numeric value of `trace_ring_fill_pct` matches the pre-bead observable value within 1 ULP for every `trace_capacity ∈ [1, 2^20]` and `trace_len ∈ [0, trace_capacity]`. (Pinned by the three RA-003 tests.)