# VB-D8MD3 Own Audit — Backpressure Warning Outcome Surfacing

## Scope

Historical branch `b9ebb909182a` (vb-d8md3) added `BackpressureWarningOutcome` enum and `enqueue_with_backpressure_warning_outcome` method to `BoundedActionCompletionQueue`. This audit classifies the added API, its tests, and its production utility against current `main` at `505464`.

## Historical API

**Added types:**
- `BackpressureWarningOutcome` enum: `Delivered`, `ChannelFull`, `ChannelDisconnected`

**Added method:**
- `enqueue_with_backpressure_warning_outcome(ticket) -> (Result<(), ActionQueueError>, BackpressureWarningOutcome)`

**Added tests (7):**
- `backpressure_outcome_delivered_when_warning_sent`
- `backpressure_outcome_delivered_without_channel`
- `backpressure_outcome_delivered_below_threshold`
- `backpressure_outcome_delivered_on_queue_full`
- `backpressure_outcome_delivered_via_channel_with_backpressure`
- `backpressure_outcome_channel_disconnected_when_receiver_dropped`
- `backpressure_outcome_consistent_with_enqueue_method`

## Current State (main at 505464)

`BoundedActionCompletionQueue` exists in `crates/vb_runtime/src/action_queue.rs` but lacks both the `BackpressureWarningOutcome` enum and the `enqueue_with_backpressure_warning_outcome` method. The current `enqueue` method silently ignores the outcome of `try_send`, collapsing all three `TrySendError` variants to no-op.

## Production Callers

**None.** The `BoundedActionCompletionQueue` is used exclusively in:
- Its own unit tests (`crates/vb_runtime/src/action_queue.rs` `unit_tests` module)
- Integration tests (`crates/vb_runtime/tests/vb_wave16_mqa.rs`)
- Workspace tests (`crates/workspace_tests/tests/restate_fifo_backpressure_tests.rs`)
- Scheduler/shutdown test fixtures (`restate_sync_drr_scheduler_tests.rs`, `restate_shutdown_drain_tests.rs`)
- Dedicated test suite (`crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs`)

The production `Shard` type uses a separate `command_queue` field (a different queue implementation). No production code path instantiates `BoundedActionCompletionQueue` or calls its backpressure API.

## Behavior Classification

### Already-Preserved Behavior (in current main)

| Feature | Status |
|---------|--------|
| Bounded capacity enforcement | ✅ Present |
| 80% ceiling backpressure threshold | ✅ Present (`backpressure_threshold`) |
| Backpressure warning via channel | ✅ Present (`backpressure_tx`) |
| FIFO dequeue ordering | ✅ Present |
| Remaining capacity tracking | ✅ Present |
| Poison recovery on lock | ✅ Present |
| Capacity validation (0 and >MAX) | ✅ Present |

### Useful Missing Behavior

**The honest warning send outcome.** The current `enqueue` method discards `try_send` results. Callers cannot distinguish:
- Warning successfully sent to receiver
- Channel full (warning dropped silently)
- Receiver dropped (channel disconnected)

This is a legitimate observability gap for queue users who need to track whether backpressure signals were received.

### Flawed Historical Design

**The `Delivered` variant is overloaded with three distinct meanings:**

1. `Delivered` when warning was actually sent and received
2. `Delivered` when no channel is configured (`backpressure_tx == None`)
3. `Delivered` when queue is full (no enqueue happened, no warning attempted)
4. `Delivered` when below threshold (no warning needed or attempted)

A caller receiving `Delivered` cannot determine whether a warning was actually delivered, merely that no error occurred. This defeats the purpose of outcome surfacing.

**Better design would use a four-state outcome:**
- `WarningDelivered` — warning was sent and channel accepted it
- `ChannelFull` — warning dropped because channel was full
- `ChannelDisconnected` — warning dropped because receiver was gone
- `NotAttempted` — no warning needed (below threshold, queue full, or no channel)

Alternatively, separate the concerns into two return values:
- `enqueue` result: `Result<(), ActionQueueError>` (same as current)
- Warning outcome: `Option<WarningSendOutcome>` where `WarningSendOutcome` has three variants (`Delivered`, `ChannelFull`, `ChannelDisconnected`) and `None` means not attempted

### Historical Test Flaws

**Test 1: `backpressure_outcome_delivered_on_queue_full`**
- Names the test "delivered on queue full" but checks for `Delivered` when no enqueue occurred
- Queue full means the ticket was NOT enqueued, so no warning was attempted
- The outcome should be "not attempted" not "delivered"

**Test 2: `backpressure_outcome_delivered_via_channel_with_backpressure`**
- Creates queue with capacity 5 and backpressure channel
- Uses `make_ticket(3)` for the 4th enqueue (ticket ID should be 3, not reused)
- The test logic is correct but the ticket ID reuse is confusing

**Test 3: `backpressure_outcome_delivered_without_channel`**
- Correctly identifies that without a channel, the outcome is `Delivered`
- But this is the overloaded/ambiguous case — "delivered" when nothing was delivered

## Recommendations

1. **Do not cherry-pick as-is.** The API has fundamental naming/semantics flaws.
2. **If warning outcome surfacing is desired**, redesign with `NotAttempted` state or `Option<WarningSendOutcome>`.
3. **No production urgency.** No callers exist, so this is not a regression.
4. **Label as cold API.** Until wired to actual shard command queue or real callers, this should be documented as a queue surface not yet integrated into runtime scheduling.
