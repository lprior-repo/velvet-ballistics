# Contract Specification - vb-d8md3

## Context
- Feature: surface backpressure warning send outcomes from `BoundedActionCompletionQueue` (`crates/vb_runtime/src/action_queue.rs`)
- Current state: `enqueue` discards `try_send` result (`Ok | Full | Disconnected => {}`)
- Branch `bead/vb-d8md3` (b9ebb9091) prototype used `Delivered` for "nothing sent" cases; rejected here as misleading - this contract defines an explicit `NotSent` variant instead

## Postconditions
- POST-001: `enqueue_with_backpressure_warning_outcome(ticket) -> (Result<(), ActionQueueError>, BackpressureWarningOutcome)` reports `Delivered` only when a warning was actually sent
- POST-002: Outcome is `NotSent` when below threshold, when no channel is configured, and when enqueue fails with `QueueFull`
- POST-003: Outcome is `ChannelFull` / `ChannelDisconnected` when `try_send` fails for those reasons
- POST-004: Existing `enqueue` behavior unchanged (same enqueue result, same warning emission); it delegates to the outcome-reporting path and discards the outcome
- POST-005: Focused tests in `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs` cover delivered / not-sent-below-threshold / not-sent-no-channel / channel-full / disconnected / queue-full cases

## Invariants
- INV-001: No unsafe, unwrap-in-production, panic, unchecked arithmetic introduced; threshold uses existing `backpressure_threshold`

## Non-goals
- Changing threshold, queue capacity semantics, warning payload, or any caller
