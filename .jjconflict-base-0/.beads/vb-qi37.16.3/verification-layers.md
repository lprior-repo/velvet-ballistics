# Verification Layers: vb-qi37.16.3 State 3 Durable Retry Transition

## Boundary

- **Verus-owned kernel**: Pure retry helpers in `vb_runtime/src/shard/helpers.rs` and `vb_runtime/src/shard/lifecycle.rs`
- **TLA+ temporal model**: RetryFSM.tla, RetryJournal.tla (see tla-spec.md)
- **Theorem projection**: None (Verus sufficient - see lean-contract.md)
- **Runtime shell**: `handle_action_failure`, journal append, external action boundary
- **External systems excluded from formal proof**: Fjall/vb_storage durability layer, external action runtime

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layers |
|-----------------|---------------|------------------|
| PRE-001 | verus | unit (lifecycle.rs tests) |
| PRE-002 | verus | kani (bounded model check) |
| PRE-003 | unit | integration (Shard tests) |
| PRE-004 | verus | tla-plus (RetryFSM) |
| POST-001 | verus | tla-plus (RetryFSM) |
| POST-002 | verus | unit |
| POST-003 | unit | tla-plus (RetryFSM Exhausted path) |
| POST-004 | tla-plus | integration (journal integration test) |
| POST-005 | verus | unit |
| POST-006 | verus | proptest |
| POST-007 | verus | unit |
| INV-001 | verus | tla-plus (monotonicity in model) |
| INV-002 | tla-plus | integration (retry exhaustion test) |
| INV-003 | tla-plus | integration (journal replay) |
| INV-004 | unit | integration |
| INV-005 | verus | tla-plus (PC reset model) |

## Verus Scope

- **Rust target**: `vb_runtime::shard::helpers::record_retry_attempt`, `validate_ticket_attempt`, `retry_is_available`, `retry_policy_after_action`
- **Spec/proof function**: `spec_record_retry_attempt`, `proof_record_retry_attempt_preserves_monotonicity`
- **Invariants**:
  - `action_attempts[step]` monotonic non-decrease
  - `ticket.attempt >= 1 && ticket.attempt <= ticket.capacity`
  - `retry_policy.max_attempts > 0`
- **Trusted boundary**: Validated `RunState` and `ActionTicket` constructors
- **Shell exclusions**: I/O, async scheduling, storage, wall-clock time
- **Evidence command**: `verus crates/vb_runtime/src/shard/helpers.rs`

## TLA+ Scope

- **Module/model path**: `specs/RetryFSM.tla`, `specs/RetryJournal.tla`
- **Variables**: `runs`, `actionAttempts`, `framePC`, `stepState`, `journal`, `maxAttempts`, `retryPolicy`, `stepHasRetryCheck`
- **Actions**: `Init`, `ActionFailed`, `StaleCompletionRejected`, `RetryNow`
- **Safety invariants**: `NoDoubleRetryAfterExhaustion`, `NoStaleCompletion`, `JournalIdempotency`, `FramePCResetOnRetry`
- **Temporal properties**: `EventuallyTerminalOrExhausted`, `EventuallyJournalAppended`
- **Fairness/deadlock stance**: Weak fairness on retry transitions; no deadlock in retry state machine
- **Refinement boundary**: `handle_action_failure` refines `ActionFailed`; `validate_ticket_attempt` refines `StaleCompletionRejected`
- **Evidence command**:
  ```bash
  tlc -config specs/RetryFSM.cfg specs/RetryFSM.tla
  tlc -config specs/RetryJournal.cfg specs/RetryJournal.tla
  ```

## Non-goals

- Fjall storage durability proofs (handled by vb_storage contract)
- External action boundary delivery guarantees
- End-to-end CLI integration with real database

## Waivers

The following waivers have been issued due to toolchain unavailability (see `formal-waivers.jsonl`):

| Waiver ID | Clause | Layer | Reason | Expiry |
|-----------|--------|-------|--------|--------|
| WAIVER-VERUS-001 | PRE-002 | verus | Verus toolchain not installed | State 12 |
| WAIVER-VERUS-002 | INV-001 | verus | Verus toolchain not installed | State 12 |
| WAIVER-VERUS-003 | POST-006 | verus | Verus toolchain not installed | State 12 |
| WAIVER-VERUS-004 | POST-001 | verus | Verus toolchain not installed | State 12 |
| WAIVER-VERUS-005 | PRE-004 | verus | Verus toolchain not installed | State 12 |
| WAIVER-KANI-001 | PRE-002 | kani | No #[kani::proof] harnesses exist | State 12 |

**Compensating evidence**: 1364 passing tests (1337 lib + 18 integration + 9 durable retry red-phase) verified by red-queen-report.md confirm implementation correctness.

**Install commands**:
- Verus: `cargo install verus --locked`
- Kani harness: Add `#[kani::proof] fn harness_validate_ticket_attempt()` to vb_runtime/src/shard/helpers.rs