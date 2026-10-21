# Test Contract: evidence_chain_captures_action_timeout_and_failure

**Location:** `crates/velvet_ballistics/tests/admission_evidence_integration/chunk_003.rs`

## Precondition
Runtime initialized; run submitted with timeout-prone action; runtime ticked.

## Ticket Retrieval
Call `runtime.lookup_pending_action_ticket(run_id, 0)` → `Option<ActionTicket>`.
- `Some(t)`: proceed
- `None`: fail fixture explicitly (missing ticket)

## Ticket Field Invariants
Pass the returned issued ticket unchanged. Preserve its seq, action, attempt, idempotency_key and capacity; do not reconstruct expected identity. The generated draft invented a t.issued field; the actual ActionTicket has no such field and that claim is rejected before application.

## Failure Injection
Submit `NonRetryable Timeout ActionFailure` for retrieved ticket.

## Postcondition Assertions
- `runs_submitted == 1`
- `runs_completed == 0`
- `runs_failed == 1`
- `lookup_pending_action_ticket(run_id, 0)` → `None` (ticket consumed)
- Journal contains exactly one `RunFailed` event for the run

## Constraints
- No production code changes
- No assertion weakening
- No fabricated tickets
- API matches step 0: `Option<ActionTicket>` return type