# Domain Model — vb-y9d3v ActionTicket Generation Fence

## Scope

Bead `vb-y9d3v` models the domain/type contract for ActionTicket generation fencing, retry authority, stale action authority, stale timer authority, and downstream proof seeds. This State 3 pass edited no production Rust and wrote contract artifacts only.

## Ubiquitous Language

| Term | Meaning | Source refs |
| --- | --- | --- |
| `ActionTicket` | Capability token authorizing exactly one externally completed or failed action attempt for one `(run, step, seq, action, attempt, capacity)` tuple. | `crates/vb_core/src/action.rs:136-153` |
| Scheduled attempt | Attempt value accepted by the shard when an engine `AwaitingAction` is journaled as `ActionScheduledTicket`. | `crates/vb_runtime/src/shard/transitions.rs:88-119` |
| Current attempt | Per-step live generation value in `RunState.action_attempts[step]`; zero means no action attempt is currently scheduled for that step. | `crates/vb_runtime/src/shard/helpers.rs:183-198` |
| Generation fence | Rule that completion/failure authority is valid only when ticket identity, canonical idempotency key, step state, action id, and current attempt generation all match the shard-owned state. | `helpers.rs:28-44`, `chunk_003.rs:48-91` |
| Lower/stale attempt | Ticket attempt lower than `current_attempt`; must not mutate frame, journal, trace, counters, or runtime state. | `helpers.rs:72-94` |
| Future attempt | Ticket attempt greater than `current_attempt`; domain contract says it is forged authority unless it was first scheduled and recorded by the shard. Current fresh-main accepts this within capacity; this is an implementation gap to close downstream. | `helpers.rs:72-94`; `lifecycle_tests/chunk_004.rs` per State 2 map |
| Retry authority | Permission to advance from attempt `n` to `n+1` after a retryable failure, bounded by retry metadata and capacity. Only runtime retry workflow owns this transition. | `helpers.rs:224-294`, `chunk_003.rs:183-195` |
| Canonical idempotency key | Deterministic key `f(run, seq, action)` that must equal the ticket key at completion preflight. Attempt and capacity do not enter this key. | `action.rs:155-173`, `engine/action.rs:202-208` |
| Stale timer authority | A fired timer entry whose generation no longer equals the current run-indexed timer entry; stale entries must not resume a wait/ask. | `timer_wheel.rs:19-37`, `timer_wheel.rs:106-128` |

## Aggregate Roots and Entities

### Run Action Authority Aggregate

- Root: `RunState` for one `RunId` owned by one shard.
- Entities: `RunFrame`, `CompiledWorkflow`, action contracts, per-step `action_attempts`, pending timers, journal sequence state.
- Invariant owner: shard, not the action handler and not IPC/direct callers.

### Action Invocation Entity

- Identity: `(run, step, seq, action, attempt)`.
- Safety bound: `1 <= attempt <= capacity` and `capacity > 0`.
- Authority preconditions:
  1. run exists and is non-terminal;
  2. step is currently `StepState::Running`;
  3. workflow node at `step` is `Do { action, .. }`;
  4. idempotency key equals canonical key for `(run, seq, action)`;
  5. attempt equals shard-recorded current attempt for external completion/failure;
  6. output/failure payload satisfies action/resource/taint bounds before mutation.

### Timer Authority Entity

- Identity: `(run, generation, deadline, kind)`.
- Safety bound: generation starts at `1` and increments with checked arithmetic on replacement.
- Authority precondition: fired entry must still be current for that run when fire/drain logic consumes it.

## Domain Decisions

1. **Exact generation for external authority is normative.** Lower attempts and future attempts are both invalid for action completion/failure unless a future attempt was previously scheduled and recorded by the shard. External callers cannot create retry authority by supplying a larger `attempt` within capacity.
2. **Scheduling may normalize only engine-originated tickets.** `AwaitingAction` produced by deterministic runtime execution may be normalized from zero/current to the scheduled attempt; completion and failure paths must not normalize.
3. **Retry capacity is a bound, not authority.** `capacity` limits attempts; it does not authorize unrecorded future attempts.
4. **Retry advancement is runtime-owned.** On retryable failure, the runtime must compute the next attempt and seq with checked arithmetic, journal/schedule it, and record it before any completion/failure for that attempt is accepted.
5. **Invalid authority is non-mutating.** Any invalid ticket/key/attempt/timer generation must return a typed runtime error before journal append, frame mutation, trace mutation, counter mutation, or runtime-state transition.
6. **Prior vb-8mdp.5 evidence is context only.** Its TLA passes may inspire temporal seeds but do not close Rust implementation proof obligations.

## Illegal States to Make Unrepresentable

| Illegal state | Desired type/domain representation |
| --- | --- |
| `attempt == 0` | `Attempt::new(u16) -> Result<Attempt, AttemptError>`; zero cannot enter core authority checks. |
| `capacity == 0` | `AttemptCapacity::new(u16)` rejects zero. |
| `attempt > capacity` | `BoundedAttempt { attempt, capacity }` smart constructor. |
| Completion/failure using future attempt | `ExternalActionAuthority::match_current(ticket, current)` returns only `Current` or typed rejection; no bool flag. |
| Retry created by external caller | `RetryAuthority` token constructible only from runtime failure workflow and retry policy. |
| Completion with noncanonical key | `CanonicalActionTicket` wrapper produced only after key verification. |
| Timer fired after replacement/cancel | `FreshTimerFire` wrapper produced only by generation equality check against current timer index. |

## Existing Implementation Gaps Against This Contract

- `validate_ticket_attempt` currently rejects zero, over-capacity, and lower stale attempts, but accepts future attempts within capacity. This conflicts with Domain Decision 1 and must be repaired or explicitly waived by owner before proof closure.
- `ActionTicket` remains a public primitive struct; downstream proofs/tests must compensate until stronger newtypes exist.
- `reject_invalid_ticket_key` is private in `lifecycle/chunk_003.rs`; any proof harness must bind to public/preflight behavior or move/expose a pure verifier-safe core intentionally.
