# Type Contracts: vb-fzgdn

## Primitive-obsession rejection
No bare `u64`, `Instant`, `Duration`, strings, booleans, unchecked casts, or `Option` lifecycle state may represent timer behavior inside the runtime core. External primitives must cross smart constructors before shard mutation.

## Value objects
| Type | Constructor contract | Error seed |
|---|---|---|
| `TimerTick` | Valid `u64` logical tick within configured horizon if enabled. | `ClockTickOutOfRange` |
| `TimerDuration` | `raw <= max_wait_duration_ticks`; zero admitted only by explicit policy. | `TimerDurationTooLarge`, `ZeroDelayRejected` |
| `TimerDeadline` | Direct validated absolute tick or checked `current_tick + duration`. | `TimerDeadlineOverflow`, `TimerDeadlineBeforeCurrent` |
| `TimerGeneration` | Checked non-wrapping successor. | `TimerGenerationExhausted` |
| `TimerCapacity` | `1..=configured_max_pending_timers`. | `InvalidTimerCapacity` |
| `DelayedActionKey` | Fixed-width deterministic key. | `InvalidDelayedActionKey` |
| `TimerAuthority` | Constructed only from an admitted pending entry. | no unchecked public constructor |

## Closed enums / sum types
- `TimerKind = Wait | Ask | Retry | DelayedAction(ActionId)`.
- `DuplicateAdmission = New | ExistingIdentical(TimerAuthority) | Conflict`.
- `TimerFireOutcome = Fired | Rejected(TimerFireError)`.
- `TimerAdmissionOutcome = Scheduled(TimerAuthority) | AlreadyScheduled(TimerAuthority) | Rejected(TimerAdmissionError)`.

## Illegal states made unrepresentable
- Missing vs zero delay: command variant plus `TimerDuration`, not `Option<u64>`.
- Run-only mutation: mutation-capable fire requires `TimerAuthority`.
- Wall-clock authority: pending entries and fire commands for this seam have no `Instant` field.
- Wrong-kind fire: kind is carried by both authority and pending entry.
- Duplicate ambiguity: identical duplicate and divergent conflict are separate outcomes.
- Capacity ambiguity: capacity is explicit and checked before insertion.

## Typestate
`ExternalTimerRequest -> ParsedTimerRequest -> ValidatedTimerAdmissionPlan -> PendingTimer/PendingDelayedAction -> FireableTimerAuthority -> FiredTimerOutcome | RejectedTimerFire`.

Only `ValidatedTimerAdmissionPlan` can mutate the registry. Only `FireableTimerAuthority` can request run-state advancement.

## Boundary parsers
- Public Rust API parser converts caller numeric ticks/durations/keys.
- Workflow slot parser converts approved `SlotValue` encodings; rejects absent, negative, non-integer, non-finite, or oversized values.
- IPC parser, if exposed, validates binary envelope before constructing timer types.

## Serialization
Journal/snapshot payloads store fixed-width numeric run, step, tick, deadline, generation, kind, action/key fields. Platform-dependent `Instant` serialization is forbidden for replay-visible timer state.
