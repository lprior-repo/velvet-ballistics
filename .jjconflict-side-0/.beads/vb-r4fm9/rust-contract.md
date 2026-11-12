# vb-r4fm9: JournalEvent attempt accessor must return run-level attempt for ticket-bearing events

## Defect

`crates/vb_storage/src/events.rs:194-196` — `JournalEvent::attempt()` returns `ticket.attempt` (per-action attempt, always 1 after retry) for `ActionScheduledTicket`, `ActionCompletedEnvelope`, and `ActionAbandoned` events, instead of `ticket.run_attempt` (run-level attempt, incremented across retries).

This causes latest-attempt replay filtering to incorrectly discard ticket-bearing events from a retried run, because:
- `compute_max_attempt()` sees non-ticket events with `run_attempt=2` and computes `max_attempt=2`
- Ticket-bearing events from run attempt 2 report `attempt=1` (per-action)
- `replay_attempt_is_stale(1, 2)` returns true, so these events are filtered out as stale

## Fix

Map ticket-bearing events to `ticket.run_attempt` in `JournalEvent::attempt()`:

```rust
Self::ActionScheduledTicket { ticket, .. }
| Self::ActionCompletedEnvelope { ticket, .. }
| Self::ActionAbandoned { ticket, .. } => Some(ticket.run_attempt),
```

## Verification

1. Reproduce the defect with a RED test: produce a durable run failure, recover_retry, and show that replay discards attempt 2 ticket-bearing events
2. Apply the fix
3. Verify the fix makes the test GREEN
4. Add permanent regression tests covering generation 2+ event extraction and cold replay/resume

## Scope

- `crates/vb_storage/src/events.rs` — the `attempt()` accessor
- Add/update regression tests in `crates/vb_runtime/tests/` and `crates/vb_storage/`
- Preserve per-action bounds/ordinal semantics (ticket.attempt remains for action-level logic)
- Do not alter replay filtering thresholds or semantics