# Contract — vb-y9d3v ActionTicket Generation Fence

## Normative Clauses

| ID | Clause |
| --- | --- |
| ACT-001 | An external action completion or failure is authorized only for a live, non-terminal run owned by the shard. |
| ACT-002 | The ticket step must be in bounds, currently `Running`, and be a `Do` node whose action equals the ticket action. |
| ACT-003 | Ticket attempt and capacity must satisfy `capacity > 0` and `1 <= attempt <= capacity`. |
| ACT-004 | The ticket idempotency key must equal `compute_action_idempotency_key(ticket.run, ticket.seq, ticket.action)`. |
| ACT-005 | For external completion/failure, `ticket.attempt` must equal the shard-recorded current attempt for `ticket.step`. Lower and future attempts are invalid authority. |
| ACT-006 | A future attempt within capacity is not retry authority unless the runtime has already scheduled and recorded that attempt. |
| ACT-007 | Invalid action authority must not mutate frame, action attempts, runtime state, journal, trace, counters, or timers. |
| ACT-008 | Completion payload checks for output slot, taint, encoded length, action contract max, and resource max must all pass before `ActionCompletedEnvelope` append. |
| ACT-009 | Failure handling must validate action authority before retry advancement, error-handler mutation, run failure, or `ActionFailed` journal append. |
| ACT-010 | Retry advancement is runtime-owned, bounded by retry metadata, and uses checked sequence/attempt arithmetic. |
| ACT-011 | Retry capacity is a maximum bound, not an authorization token. |
| ACT-012 | Terminal run cleanup fences off later action completions/failures for that run. |
| TMR-001 | A timer fire is authoritative only when its generation equals the current timer entry for the run. |
| TMR-002 | Timer replacement increments generation with checked arithmetic; overflow fails closed. |
| TMR-003 | Cancelled/replaced timer entries are stale and must not resume wait/ask state. |
| VER-001 | Proof artifacts must bind to fresh-main production functions and may not use hardcoded Kani workflow shapes or detached Verus/Flux models as closure. |
| VER-002 | Prior vb-8mdp.5 artifacts are historical context only and cannot be cited as proof approval for this bead. |

## Acceptance Invariants for Downstream States

1. Every generated proof/test must include hostile public `ActionTicket` inputs, not only runtime-generated happy paths.
2. There must be explicit coverage for lower stale, exact current, future within capacity, zero attempt, zero capacity, and over-capacity attempts.
3. There must be explicit coverage that stale/future/invalid key completions and failures leave journal/frame/trace/runtime state unchanged.
4. There must be explicit coverage that retryable failure does not authorize `n+2` before `n+1` is scheduled.
5. There must be explicit coverage for stale timer generation after replacement/cancel.
6. Verifier lanes must be planned against current fresh-main wiring, not missing prior files or nonexistent features.

## Open Domain Questions

- Should `RuntimeError::FutureAttempt { incoming, current }` be added as a stable public variant, or should future-attempt rejection be represented as `InvalidActionCompletion` while preserving behavior? This contract accepts either error surface but not acceptance of future attempts.
