# Workflow Model — vb-vzo9b

> **Scope.** Workflow of the post-fix `fuzz_recovery_decode` test body. Inputs
> are a fuzz payload; outputs are panic-on-divergence, panic-on-untype-error,
> or silent pass. No state lives across calls; the procedure is straight-line.

## Legal States

There is no persistent state. Each call to `fuzz_recovery_decode` traverses
exactly one of two **input-derived branches**:

| State ID | Predicate on `data` | Resulting `events` |
|---|---|---|
| `S-Even` | `data.len().is_multiple_of(2)` | `vec![JournalEvent::RunAccepted { run, seq, workflow: digest }]` |
| `S-Odd` | `!data.len().is_multiple_of(2)` | `Vec::new()` |

Inside either branch, `match summarize_recovery_events(&events)` partitions
the call path into an `Ok(...)` arm and an `Err(error)` arm.

| Ok arm sub-state | Predicate | Outcome |
|---|---|---|
| `Ok.S-Even.NonEmpty` | `Ok(hydration)` ∧ `!events.is_empty()` (i.e. branch `S-Even`) | **Assert `assert_eq!(hydration.summary(), expected)`**. |
| `Ok.S-Odd.NoEvents` | `Ok(hydration)` ∧ `events.is_empty()` (unreachable for `S-Odd` because the empty slice returns `Err`) | impossible — empty `&[]` returns `Err(NoRecoveryData)`. The post-fix body keeps the `if !events.is_empty()` guard as a static analyzer aid, mirroring the pre-fix structure. |

| Err arm sub-state | Source of error | Outcome |
|---|---|---|
| `Err.NoRecoveryData` | empty `events` slice (`S-Odd`) | `RecoveryError::NoRecoveryData { run: RunId::new(0) }` — `assert_typed_recovery_error` sinks it (`errors.rs:60-72`). |
| `Err.ReplayDivergence.MultiRun` | (not reachable from the current fuzz body — would require ≥ 2 events with mismatched `run_id`) | `RecoveryError::ReplayDivergence { detail: "recovery summary received events for multiple runs" }`. The current body cannot trigger it. |
| `Err.ReplayDivergence.SeqOverflow` | (not reachable — `seq = EventSeq::new(1)`) | `RecoveryError::ReplayDivergence { detail: "overflow sentinel sequence N is not valid" }`. The current body cannot trigger it. |

The second production call (`recover_runtime_frame_seed_from_events(&events)`)
runs unconditionally after the first and routes any `Err` through
`assert_typed_recovery_error`.

## Transition Diagram

```
                ┌──────────────────────────────┐
                │  entry: fuzz_recovery_decode │
                │       data: &[u8]            │
                └──────────────┬───────────────┘
                               │
                       derive inputs
                  (digest, run, seq, events)
                               │
              ┌────────────────┴─────────────────┐
              │ events.is_empty() ?               │
              └──┬─────────────────────────┬──────┘
                 │                         │
       Yes  (S-Odd)                  No  (S-Even)
                 │                         │
   summarize_recovery_events  summarize_recovery_events
                 │                         │
       Err(NoRecoveryData)        ┌────────┴────────┐
                 │                │                 │
   assert_typed_recovery_error    Ok               Err
                                 │                 │
                          ┌──────┴──────┐    assert_typed_
                          │ events      │    recovery_error
                          │ non-empty ? │
                          └──┬──────┬───┘
                          Yes │      │ No
                             │      └──> (impossible)
                  assert_eq!(hydration.summary(), expected)
                             │
                  panic if diverehgence, pass otherwise
                             │
                             ▼
       recover_runtime_frame_seed_from_events(&events)
                             │
                  ┌──────────┴───────────┐
                  │ Err                  │ Ok
                  ▼                      ▼
          assert_typed_recovery_error    pass
```

## Guards

| Guard ID | Predicate | When fires |
|---|---|---|
| `G-Empty` | `events.is_empty()` | Selects `S-Odd`; suppresses the `assert_eq!`. |
| `G-NonEmpty` | `!events.is_empty()` | Selects `S-Even`; enables `assert_eq!(hydration.summary(), expected)`. |
| `G-Ok` | `matches!(result, Ok(_))` on summary call | Routes to `assert_eq!`. |
| `G-Err` | `matches!(result, Err(_))` on summary call | Routes to `assert_typed_recovery_error`. |
| `G-FrameSeed-Err` | `matches!(result, Err(_))` on frame-seed call | Routes to `assert_typed_recovery_error`. |

## Outcomes

| Outcome | Trigger | Effect |
|---|---|---|
| `OUT-Pass` | All guards pass and `assert_eq!(hydration.summary(), expected)` holds. | No panic. The fuzz target continues to the second production call. |
| `OUT-Divergence-Panic` | Any field of `hydration.summary()` differs from `expected`. | `assert_eq!` panics with `Debug`-formatted diff. |
| `OUT-Untype-Error-Panic` | `RecoveryError` variant not enumerated in `assert_typed_recovery_error`. | The `_ => {}` arm in `errors.rs:57-72` is **non-panicking**, so this is unreachable today; if a future variant is added without updating the sink, the panic surfaces at the call site. (Documented for future maintenance.) |
| `OUT-MultiRun-Reachable` | Fuzz payload constructed with ≥ 2 `run_id`-mismatched events. (Not in scope of vb-vzo9b; future strengthening.) | Production returns `Err(ReplayDivergence)`; sink absorbs it. |
| `OUT-SeqOverflow-Reachable` | Fuzz payload constructed with `seq == EventSeq::MAX`. (Not in scope of vb-vzo9b; future strengthening.) | Production returns `Err(ReplayDivergence)`; sink absorbs it. |

## Temporal Hazards (in-scope)

None for this bead. The fuzz body is straight-line; there is no async, no
cancellation, no scheduler, no timer. Loom/proptest temporal coverage is not
required (see `proof-planner` post-contract disposition: `loom`-profile seeds
are intentionally absent).

## Reference Strong Pattern

`crates/vb_storage/src/recovery/replay/summary/tests.rs:285-302` already
asserts `RecoveryRuntimeSummary`-level invariants exactly via `matches!(...)`
with field-level guard predicates. The post-fix `readback.rs` body follows the
same principle but uses `assert_eq!(value, expected)` instead of `matches!`
because `RecoveryRuntimeSummary` is `PartialEq + Eq + Copy + Debug` — making a
plain `assert_eq!` strictly stronger.

## Diff to a Workflow Hazard Template

The standard workflow-hazard template asks for liveness, fairness, deadlock,
and timer hazards. **This bead has none of those** — the procedure is
synchronous, deterministic, and bounded by the length of `events` (which is
either 0 or 1 in the current fuzz driver). All workflow hazards are
`N/A` with the note "test-only repair of a non-temporal contract".
