# Contract: vb-fzgdn — deterministic delayed-action timer seam

## Acceptance contract
Runtime behavior-affecting timer registration, authority, firing, journaling, and replay state must use deterministic numeric time rather than `std::time::Instant`.

## Downstream implementation obligations
1. `PendingTimer`, `TimerEntry`, and `ShardCommand::TimerFired` successors for this seam carry `TimerDeadline`, not `Instant`.
2. Timer registration does not call `Instant::now()` in behavior-affecting acceptance path.
3. Deterministic seam accepts explicit numeric `TimerTick`/`TimerDuration`/`TimerDeadline` through smart constructors.
4. `WaitUntil`, `WaitEvent`, and `Ask` slot-derived time values validate before pending timer mutation.
5. Mutation-capable fire requires full `TimerAuthority`; run-only fire remains fail-closed.
6. Invalid authority cannot remove pending entry, journal success, enqueue delayed action, or advance run frame.
7. Duplicate delayed-action keys: identical duplicate is idempotent, divergent duplicate is typed conflict.
8. Timer registry and delayed-action index are bounded.
9. Clock advancement is explicit and monotonic.
10. Zero-delay behavior is deterministic and documented.

## Non-goals
No production Rust, tests, verifier artifacts, JSON/YAML/HTTP runtime payloads, wall-clock sleeps in core, or claim that TLA design smoke closes Rust-local obligations.

## Open domain decisions
Zero-delay policy; exact public API names; absolute-vs-normalized `WaitUntil`; migrate old `TimerWheel` vs new numeric registry.
