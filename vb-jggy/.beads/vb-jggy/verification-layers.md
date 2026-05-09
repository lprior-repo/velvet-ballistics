# Verification Layers: vb-jggy

## Boundary

- **Verified kernel**: `vb_runtime::shard::helpers` — pure functions `validate_ticket_attempt`, `record_scheduled_attempt`, `normalize_scheduled_ticket`; and `lifecycle.rs` — run admission and completion paths.
- **Runtime shell**: `vb_runtime::shard::Shard` — single-threaded mutable state; no concurrent access to `action_attempts`.
- **External systems excluded from formal proof**: Fjall journal durability (covered by integration tests), artifact store admission (pre-existing), IPC wire protocol (out of scope).

## Layer Assignment

| Contract Clause | Primary Layer | Secondary Layer | Rationale |
|-----------------|---------------|-----------------|-----------|
| PRE-001 | `manual-qa` | `static-scan` | Code review of master-doc Section 72; scan for missing attempt tracking |
| PRE-002 | `manual-qa` | `static-scan` | Code review of completion path; scan for missing `validate_ticket_attempt` calls |
| PRE-003 | `manual-qa` | `static-scan` | Same as PRE-002 for failure path |
| PRE-004 | `static-scan` | `miri` | Verify `StaleAttempt` variant exists; Miri for UB-free error handling |
| POST-001 | `proptest` | `kani` | Property: zero-initialized `action_attempts` at admission; Kani for bounded model |
| POST-002 | `kani` | `proptest` | Property: first ticket.attempt == 1; Kani for state transition proof |
| POST-003 | `proptest` | `manual-qa` | Property: journal events carry correct attempt; fuzz journal event serialization |
| POST-004 | `kani` | `miri` | Property: stale gate precedes journal append; Kani for ordering proof |
| POST-005 | `kani` | `proptest` | Property: stale attempt returns error before mutation; Kani bounded model |
| POST-006 | `proptest` | `kani` | Property: counter monotonicity; proptest for many interleavings |
| INV-001 | `kani` | `proptest` | One latest attempt per step; Kani for all reachable states |
| INV-002 | `kani` | `proptest` | Older attempts cannot win; Kani for transition safety |
| INV-003 | `kani` | `miri` | Attempt check before mutation; Kani for pre-condition ordering |
| INV-004 | `proptest` | `kani` | Monotonic non-decrease; proptest for adversarial sequences |

## Lean Scope

- **Theorem module**: `vb_runtime::shard::helpers::validate_ticket_attempt`
- **Rust target**: `validate_ticket_attempt` (pure, no side effects)
- **Abstraction relation**: `state.action_attempts[step]` models the physical per-step counter; `ticket.attempt` models the incoming attempt from the engine.
- **Theorem shape**: If `validate_ticket_attempt(state, ticket)` returns `Ok(())`, then `ticket.attempt >= state.action_attempts[ticket.step.as_usize()]` and `ticket.attempt > 0` and `ticket.attempt <= ticket.capacity`.
- **Inputs**: `state: RunState`, `ticket: ActionTicket`
- **Outputs**: `RuntimeResult<()>`
- **Non-goals**: Async I/O, journal persistence, multi-threaded concurrency, artifact store.

## Waivers

- **Waiver for POST-003 (journal event attempt field)**: Fuzz testing of `RuntimeJournalEvent` serialization/deserialization is covered by existing `cargo-fuzz` corpus for `vb_storage`. No duplicate proof needed.
- **Waiver for INV-001 (single latest attempt)**: Single-threaded shard guarantees; proven by architecture contract (no `Arc<Mutex<RunState>>`). No additional concurrent proof required.
- **Waiver for external system integration**: Fjall durability verified by existing integration tests in `vb_storage`. Manual QA gate covers end-to-end durability.
