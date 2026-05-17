# Theorem Kernel Projection: vb-qi37.16.3 State 3

## Boundary

- **TLA+-owned temporal model**: RetryFSM.tla, RetryJournal.tla (see tla-spec.md)
- **Verus-owned Rust core**: Pure retry logic in `vb_runtime/src/shard/helpers.rs` and `vb_runtime/src/shard/lifecycle.rs`
- **Theorem-owned kernel**: None for this bead - Verus handles all Rust-local pure critical behavior
- **Rust/runtime shell**: I/O, async scheduling, journal durability (Fjall), external action boundary
- **External systems excluded from theorem proof**: Fjall storage, external action runtime

## Theorem-Owned Clauses

None - Verus is sufficient for all Rust-local pure critical behavior in this bead scope.

Rationale:
- `record_retry_attempt` monotonicity is expressible in Verus as a loop invariant
- PC reset correctness is a straightforward state transition proof in Verus
- Ticket attempt bounds are arithmetic properties expressible in Verus spec functions
- No algebraic state lattice, protocol refinement, or arithmetic bound theorems require Lean extraction

## Lean Theorem Obligations

None - this bead has no theorem kernel beyond what Verus can express.

## Waivers

- **WAIVER-THM-001**: Not applicable
  - Owner: vb-qi37.16.3 State 3
  - Reason: Verus is sufficient for all Rust-local pure critical behavior
  - Scope: All contract clauses in this bead
  - Expiry: Never (permanent until bead scope changes)
  - Compensating evidence: Verus proof obligations in proof-obligations.jsonl

## Verus-Owned Clauses (Contract Clarity)

The following contract clauses are owned by Verus (not Lean):

| Contract Clause | Verus Target | Verus Surface |
|----------------|--------------|---------------|
| INV-001 | `vb_runtime::shard::helpers::record_retry_attempt` | spec fn + loop invariant |
| INV-005 | `vb_runtime::shard::lifecycle::apply_action_failure_to_state` | proof fn for PC reset |
| PRE-002 | `vb_runtime::shard::helpers::validate_ticket_attempt` | spec fn with bounds |
| POST-006 | `vb_runtime::shard::helpers::record_retry_attempt` | postcondition `attempt >= ticket.attempt` |
| POST-007 | `vb_runtime::shard::helpers::validate_ticket_attempt` | postcondition `StaleAttempt` error |