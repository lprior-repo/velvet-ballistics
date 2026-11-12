# vb-r4fm9 Handoff

## Defect

`crates/vb_storage/src/events.rs:194-196` — `JournalEvent::attempt()` returned `ticket.attempt` (per-action, always 1) for ticket-bearing events (`ActionScheduledTicket`, `ActionCompletedEnvelope`, `ActionAbandoned`), instead of `ticket.run_attempt` (run-level, incremented across retries).

This caused latest-attempt replay filtering to incorrectly discard ticket-bearing events from retried runs:
- `compute_max_attempt()` sees non-ticket events with `run_attempt=2` → `max_attempt=2`
- Ticket-bearing events from run attempt 2 reported `attempt=1` → filtered as stale
- Result: action scheduling/completion events lost during cold replay

## Fix

```rust
// crates/vb_storage/src/events.rs
Self::ActionScheduledTicket { ticket, .. }
| Self::ActionCompletedEnvelope { ticket, .. }
| Self::ActionAbandoned { ticket, .. } => Some(ticket.run_attempt),
```

Maps ticket-bearing events to run-level attempt for correct replay filtering. Per-action attempt (`ticket.attempt`) remains available for action-level logic; only the accessor changed.

## Verification

### RED reproducer
```bash
python3 /tmp/vb-run-gate.py 420 .beads/vb-r4fm9/evidence/red-event-generation \
  cargo test -p vb_runtime --test vb_r4fm9_event_generation ticket_events_report_run_attempt_on_retry
```
**Result:** Failed with `assertion left == right failed: ticket-bearing event from run attempt 2 must report attempt=2 via accessor (left: Some(1), right: Some(2))`

### GREEN after fix
```bash
python3 /tmp/vb-run-gate.py 420 .beads/vb-r4fm9/evidence/green-event-generation \
  cargo test -p vb_runtime --test vb_r4fm9_event_generation ticket_events_report_run_attempt_on_retry
```
**Result:** Passed

### Full regression suite
```bash
python3 /tmp/vb-run-gate.py 420 .beads/vb-r4fm9/evidence/green-all-tests \
  cargo test -p vb_runtime --test vb_r4fm9_event_generation
```
**Result:** 3/3 passed (`ticket_events_report_run_attempt_on_retry`, `cold_replay_preserves_run_attempt_2_ticket_events`, `completed_retry_replays_as_completed`)

### Existing durable retry tests
```bash
python3 /tmp/vb-run-gate.py 420 .beads/vb-r4fm9/evidence/durable-authority-test \
  cargo test -p vb_runtime --test durable_retry_run_attempt_authority
```
**Result:** 4/4 passed — fix is compatible with existing behavior

## Files changed

- `crates/vb_storage/src/events.rs` — fixed `attempt()` accessor for ticket-bearing events
- `crates/vb_runtime/tests/vb_r4fm9_event_generation.rs` — 3 new regression tests (RED/GREEN/replay/completed-retry)
- `.beads/vb-r4fm9/rust-contract.md` — bead contract
- `.beads/vb-r4fm9/handoff.md` — this document

## Remaining limitations

- Pre-existing fixture compilation errors in `vb_storage` test binaries (missing `run_attempt` field in test ticket constructors) prevent running the full `cargo test -p vb_storage` suite. These are being fixed by LOCAL3090 vb-3gthe; not regressions from this fix.
- Pre-existing clippy warning in `vb_core/src/action/ticket.rs` (`issue_action_ticket` has 8 arguments) is not introduced by this fix.
- Cold replay of run attempt 3+ not explicitly tested, but the fix is symmetric across all run attempts.