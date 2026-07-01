# Error Taxonomy — vb-vzo9b

> **Scope.** All `RecoveryError` variants reachable from
> `fuzz_recovery_decode` post-fix. The fuzz body does not introduce new error
> variants; it consumes the production `RecoveryError` via the existing
> `assert_typed_recovery_error` sink at `fuzz/src/journal_target/errors.rs:57-72`.
> This taxonomy enumerates each variant, its trigger, and how it is consumed.

## `RecoveryError` — exhaustive variant list (per `errors.rs:60-70`)

| ID | Variant | Trigger in `fuzz_recovery_decode` | Post-fix handling |
|---|---|---|---|
| E-1 | `RecoveryError::Journal(JournalError)` | Propagated from the underlying journal decoder. Reachable in principle; not exercised by the current fuzz body's input shape (`Vec` of in-memory `JournalEvent`s, no journal I/O). | Sink — `assert_typed_recovery_error` enumerates `Journal(_)`. |
| E-2 | `RecoveryError::WorkflowSourceDigestMismatch { .. }` | Could fire inside `apply_summary_event_checked` if the constructed event's workflow mismatch rules trigger; current fuzz payload does not exercise this. | Sink. |
| E-3 | `RecoveryError::CompiledIrDigestMismatch { .. }` | Ditto. | Sink. |
| E-4 | `RecoveryError::ActionAbiMismatch { .. }` | Ditto. | Sink. |
| E-5 | `RecoveryError::PolicyDigestMismatch { .. }` | Ditto. | Sink. |
| E-6 | `RecoveryError::NonIdempotentActionBlocked { .. }` | Reachable for `ActionScheduled`/`ActionResolved` events. The fuzz body does not construct such events. | Sink. |
| E-7 | `RecoveryError::ReplayDivergence { step, detail }` | Triggered in two distinct sub-cases (see E-7a / E-7b). | Sink — `ReplayDivergence { .. }`. |
| E-7a | `ReplayDivergence { detail: "recovery summary received events for multiple runs".to_owned() }` | `apply.rs:108-114` — `events` slice contains two events with different `run_id`. **Not reachable** from the current fuzz body (single-event or empty). | Sink. |
| E-7b | `ReplayDivergence { detail: format!("overflow sentinel sequence N is not valid", ...) }` | `apply.rs:115-122` — any event has `seq == EventSeq::MAX`. **Not reachable** (`seq = EventSeq::new(1)`). | Sink. |
| E-8 | `RecoveryError::SlotTaintReadFailed { .. }` | Reachable only via `SlotWritten` events with bad taint; fuzz body emits none. | Sink. |
| E-9 | `RecoveryError::CorruptSlotTaint { .. }` | Ditto. | Sink. |
| E-10 | `RecoveryError::NoRecoveryData { run: RunId::new(0) }` | `apply.rs:89-91` — empty `events` slice (the `S-Odd` branch). **This is the only error that fires today.** | Sink — enumerated via `NoRecoveryData { .. }`. |

**Catch-all.** `errors.rs:71` has a `_ => {}` arm. This is intentional: any new
variant added to `RecoveryError` without updating the sink will route through
the catch-all silently. The post-fix body must therefore keep the catch-all
arm (mirroring existing project convention) but **must not** rely on it for
the `NoRecoveryData` variant — `NoRecoveryData` is enumerated explicitly.

## Sink (`assert_typed_recovery_error`) — formal contract

```rust
pub(super) fn assert_typed_recovery_error(error: vb_storage::recovery::RecoveryError) {
    use vb_storage::recovery::RecoveryError;
    match error {
        RecoveryError::Journal(_)
        | RecoveryError::WorkflowSourceDigestMismatch { .. }
        | RecoveryError::CompiledIrDigestMismatch { .. }
        | RecoveryError::ActionAbiMismatch { .. }
        | RecoveryError::PolicyDigestMismatch { .. }
        | RecoveryError::NonIdempotentActionBlocked { .. }
        | RecoveryError::ReplayDivergence { .. }
        | RecoveryError::SlotTaintReadFailed { .. }
        | RecoveryError::CorruptSlotTaint { .. }
        | RecoveryError::NoRecoveryData { .. } => {}
        _ => {}
    }
}
```

`fuzz/src/journal_target/errors.rs:57-72`.

**Contract.**

- For every variant enumerated in the `match`, the call is non-panicking.
- For every variant NOT enumerated, the catch-all `_ => {}` is also non-panicking.
- Therefore the function returns `()` for every possible `RecoveryError`.
- Therefore the post-fix fuzz body **must never panic on the error path** —
  any panic must come from the `assert_eq!` line, never from
  `assert_typed_recovery_error`.

## Forbidden handling

| Forbidden pattern | Reason |
|---|---|
| `unwrap()`/`expect()` on the recovery `Result`. | The fuzz body is a domain-facing test; panics with explicit error strings belong in `Debug`-formatted `assert_eq!`, not in `unwrap`. |
| Adding a new local error type to the fuzz body. | The contract says the fuzz body is a *sink* for production errors, not a producer. |
| Re-enumerating `RecoveryError` inside `readback.rs`. | The single existing source of truth is `errors.rs:57-72`; duplicating it risks divergence. |
| Calling `assert_typed_recovery_error` with the wrong variant (e.g. passing a `JournalError` to it). | The function's signature accepts only `vb_storage::recovery::RecoveryError`; type-safety is enforced by the compiler. |

## Error-rail vs success-rail in the post-fix body

```
                  ┌───────────────────────────────────┐
                  │        RecoveryResult<T>          │
                  └──┬────────────────────────────┬──┘
                     │                            │
            ┌────────▼────────┐         ┌────────▼────────┐
            │ Ok(hydration)   │         │ Err(error)      │
            │                 │         │                 │
            │ if non-empty:   │         │ assert_typed_   │
            │   assert_eq!(..) │         │ recovery_error  │
            │ else: no-op     │         │                 │
            └─────────────────┘         └─────────────────┘
```

The success rail uses the (now exhaustive) `assert_eq!`. The error rail uses
the existing `assert_typed_recovery_error`. There is exactly one success rail
and one error rail; no third rail is introduced.

## Strong-Pattern Reference

`crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` follows the
same error-rail pattern but inverts the polarity: it tests the `Err` cases
(via `matches!(result, Err(NoRecoveryData { run }) if run == RunId::new(0))`).
The post-fix `readback.rs` body pairs the production-style `Ok` arm with the
production-style `Err` sink, preserving symmetry: success-rail precision +
error-rail exhaustiveness.
