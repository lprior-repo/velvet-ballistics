# Formal Proportional Waiver: vb-fnbv

## Bug Description

TLA spec modeling artifact in `AwaitAsk` action. The TLA specification sets timer state (`pendingTimerKind`, `pendingTimerStep`) **before** calling `AppendEvent`. If `AppendEvent` returns `JournalFull`, the timer state is already mutated — this is the counterexample TLC exposed.

## Affected Spec

`specs/AskAnswerLifecycle.tla` lines 96–112

```
\* TLA sets timer state BEFORE AppendEvent — modeling artifact
pendingTimerKind := kind;
pendingTimerStep := step;
AppendEvent(AskScheduled(...));
\* If AppendEvent fails with JournalFull, timer state already mutated
```

## Affected Invariant

`AskTimerImpliesAskScheduled` — asserts that any pending ask timer implies a durable `AskScheduled` journal event exists.

## Root Cause

TLA spec defect: `AwaitAsk` action sets `pendingTimerKind` and `pendingTimerStep` before calling `AppendEvent`. On `JournalFull` error, the timer state is already committed to the model variables, violating `AskTimerImpliesAskScheduled`.

Rust implementation is **correct**: `await_timer` (transitions.rs:161–173) appends `AskScheduled` journal event **before** inserting `pending_timers`. If journal append fails, the timer is never registered and the error propagates typed to the caller.

## Rust Evidence

- `crates/vb_runtime/src/transitions.rs:161–173`: `await_timer` orders journal append before timer registration — correct by construction
- `crates/vb_runtime/src/shard/tests/chunk_013.rs`: test `runtime_ask_timer_append_failure_does_not_register_pending_timer` passes, proving the Rust behavior

```rust
// transitions.rs:161–173 — correct ordering
let event = AskScheduled { ... };
let encoded = codec::encode_event(&event)?;
self.journal.append(encoded)?;          // 1. append first
self.pending_timers.insert(timer_key);  // 2. only register on success
```

## Accepted Risk

**Terminal error semantics.** When `JournalFull` is returned from `await_timer`:
- Run transitions to `Failed` terminal state
- No scheduling proceeds for that ask
- No silent data loss — error is typed and propagated
- Caller must handle `JournalFull` explicitly

This is an intentional boundary condition. The system fails visibly rather than silently corrupting timer state.

## Proportionality

| Evidence | Scope |
|---|---|
| Behavior test `runtime_ask_timer_append_failure_does_not_register_pending_timer` | Targeted unit — journal append failure path |
| `AskAnswerLifecycle.tla` TLC model | Formal proof of TLA spec defect, not Rust bug |
| Rust `await_timer` code review | transitions.rs:161–173 — manual trace of correct ordering |

Risk is **scoped to `AskAnswer` lifecycle terminal error path**. No other timer or journal path is affected.

## Waiver Justification

This is a **TLA modeling gap**, not a production bug.

1. **TLA defect**: `AwaitAsk` in `AskAnswerLifecycle.tla` mutates timer state before `AppendEvent` call
2. **Rust correct**: `await_timer` implements correct ordering — journal append precedes timer registration
3. **Risk scope**: Terminal error path only; no silent data loss; typed error propagation
4. **Evidence proportional**: One targeted unit test + TLA counterexample evidence + code review traces the full gap

The `AskTimerImpliesAskScheduled` invariant violation exists only in the TLA spec artifact. The Rust implementation satisfies the intended invariant by construction.

---

**Waiver approved**: 2026-05-30  
**Bead**: vb-fnbv  
**Type**: formal-proportional-waiver  
**Scope**: AskAnswerLifecycle terminal error semantics
