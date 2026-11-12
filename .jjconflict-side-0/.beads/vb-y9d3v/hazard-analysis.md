# Hazard Analysis — vb-y9d3v

| Hazard ID | Class | Hazard | Consequence | Contract control | Proof seed |
| --- | --- | --- | --- | --- | --- |
| H-ACT-001 | Temporal authority | Lower stale attempt completes after retry scheduled | Old side effect overwrites newer run state | Reject `attempt < current` before mutation | PS-ACT-001 |
| H-ACT-002 | Temporal authority | Future attempt supplied by hostile caller within capacity | Caller skips retry workflow and fabricates authority | Reject `attempt > current`; capacity is bound only | PS-ACT-002 |
| H-ACT-003 | Idempotency | Noncanonical key accepted | Duplicate/replay tracking can be bypassed | Verify key before completion mutation | PS-ACT-003 |
| H-ACT-004 | Mutation ordering | Invalid completion/failure appends journal before rejection | Durable false event/replay divergence | Preflight all authority before append | PS-ACT-004 |
| H-ACT-005 | Retry arithmetic | `seq` or `attempt` overflows on retry | Wraparound creates duplicate authority | Checked arithmetic, typed internal error | PS-ACT-005 |
| H-ACT-006 | Retry policy | Retry metadata zero/out-of-range/non-integer | Unbounded or invalid retry loop | Reject policy extraction failures | PS-ACT-006 |
| H-ACT-007 | Taint/resource | Completion downgrades taint or exceeds byte bounds | Secret leak or resource exhaustion | Taint/encoded/resource preflight | PS-ACT-007 |
| H-ACT-008 | Terminal race | Completion/failure arrives after run terminal removal | Zombie mutation or journal after terminal | Live-run lookup and terminal fence | PS-ACT-008 |
| H-TMR-001 | Timer staleness | Replaced/cancelled timer fires from old deadline bucket | Wait/ask resumes incorrectly | Generation equality check and run index removal | PS-TMR-001 |
| H-TMR-002 | Timer arithmetic | Timer generation overflows | Reused generation admits stale fire | Checked generation increment fails closed | PS-TMR-002 |
| H-VER-001 | Verification quality | Kani hardcoded workflow shape or detached Verus/Flux model | Vacuous proof closure | Generator-backed harnesses, production binding | PS-VER-001 |
| H-VER-002 | Evidence reuse | Prior rejected vb-8mdp.5 evidence copied as approval | False closure | Treat as context only; fresh-main artifacts required | PS-VER-002 |

## Current Gap Risks

1. **Future attempts remain representable and accepted in fresh-main helper logic.** This is the primary contract/implementation mismatch. Downstream implementation/proof work must change `validate_ticket_attempt` semantics or record an owner decision that future attempts are intentionally valid.
2. **Primitive public DTO shape remains forgeable.** Proofs/tests must model hostile `ActionTicket` values rather than only runtime-generated values.
3. **Verifier artifacts are missing/unwired.** The existing Kani artifact is suspect because it is unwired, references wrong/private helpers, and uses hardcoded shapes per State 2 map.
4. **Timer generation proof must bridge both indexes.** `fire_expired` must be shown to ignore stale entries after replacement/cancel, not merely drain by deadline.
