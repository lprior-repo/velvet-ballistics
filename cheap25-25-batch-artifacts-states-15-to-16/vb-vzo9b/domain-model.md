# Domain Model — vb-vzo9b

> **Scope.** Test-only repair at `fuzz/src/journal_target/readback.rs:196` inside
> `fuzz_recovery_decode`. Replace the disjunctive `run == local || run == RunId(0)`
> check with an exact `assert_eq!(run_summary, expected_recovery_runtime_summary)`
> over every observable field of `RecoveryRuntimeSummary`.
> **No production code is touched** (multi-run rejection already lives in
> `crates/vb_storage/src/recovery/replay/summary/apply.rs:108-114` and
> `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:78-90`).

## Ubiquitous Language

| Term | Meaning in this bead |
|---|---|
| **Fuzz payload** | Arbitrary `&[u8]` byte slice supplied to `fuzz_recovery_decode`. First byte seeds `RunId`, length parity selects single-`RunAccepted` vs empty `events`. |
| **Single-RunAccepted event** | The fuzz-constructed `JournalEvent::RunAccepted { run, seq = EventSeq::new(1), workflow: digest }` produced when `data.len().is_multiple_of(2)`. |
| **Empty events** | The fuzz-constructed `Vec::new()` produced when `data.len()` is odd. Triggers the `NoRecoveryData { run: RunId::new(0) }` error branch. |
| **`RecoveryRuntimeSummary`** | The 11-field `Copy + PartialEq + Eq + Debug` value object at `crates/vb_storage/src/recovery/types.rs:547-570` returned by `RecoveryHydration::summary()`. |
| **Expected summary** | The locally-built `RecoveryRuntimeSummary` whose every field is deterministically determined by the constructed inputs (see `type-contracts.md`). |
| **Exact pin** | A `assert_eq!(actual, expected)` assertion that simultaneously checks all 11 fields and therefore catches any divergence in `summarize_recovery_events` or `recover_runtime_frame_seed_from_events`. |
| **Disjunction defect** | The pre-fix `assert!(summary.run == run || summary.run == RunId::new(0))`. Two distinct `RunId` values are accepted where exactly one is correct for the non-empty-events branch. The `RunId::new(0)` branch is the production sentinel returned by `RecoveryError::NoRecoveryData { run: RunId::new(0) }` (see `crates/vb_storage/src/recovery/replay/summary/apply.rs:90`); it is never a valid summary `run` for non-empty events. |
| **`assert_typed_recovery_error`** | Fuzz-only typed error sink at `fuzz/src/journal_target/errors.rs:57-72`; enumerates every legal `RecoveryError` variant the fuzz target may observe. |

## Value Objects

### `RecoveryRuntimeSummary` (already defined in production, unchanged)

11 pin-able fields (per `types.rs:547-570`):

1. `run: RunId`
2. `first_seq: EventSeq`
3. `last_seq: EventSeq`
4. `workflow: Option<WorkflowDigest>`
5. `steps_started: u64`
6. `steps_succeeded: u64`
7. `actions_scheduled: u64`
8. `actions_resolved: u64`
9. `suspensions: u64`
10. `slots_written: u64`
11. `terminal: Option<RecoveryTerminalState>`

The derive set (`Debug, Clone, Copy, PartialEq, Eq`, see `types.rs:546`) makes a
single `assert_eq!` over the whole struct a sound, exhaustive field check.

### `RecoveryHydration` (already defined in production, unchanged)

`#[non_exhaustive] enum` with `Summary(RecoveryRuntimeSummary)` and
`FrameSeed(RecoveryFrameSeed)` variants (`types.rs:587-594`). Its
`fn summary(&self) -> RecoveryRuntimeSummary` accessor (`types.rs:596-605`)
returns the inner summary from either variant, so the post-fix
`assert_eq!(hydration.summary(), expected)` is exhaustive over both variants.

### `ExpectedSummary` (introduced by this contract, lives in `fuzz/src/journal_target/readback.rs`)

A locally-constructed `RecoveryRuntimeSummary` whose **every** field is fixed
by the fuzz-driver inputs:

| Field | Exact expected value |
|---|---|
| `run` | `run` (the fuzz-constructed `RunId`) |
| `first_seq` | `seq` (= `EventSeq::new(1)`) |
| `last_seq` | `seq` (= `EventSeq::new(1)`) |
| `workflow` | `Some(digest)` |
| `steps_started` | `0` |
| `steps_succeeded` | `0` |
| `actions_scheduled` | `0` |
| `actions_resolved` | `0` |
| `suspensions` | `0` |
| `slots_written` | `0` |
| `terminal` | `None` |

`ExpectedSummary` does **not** need to be exported, named at the type level, or
wrapped in a smart constructor — it is a `RecoveryRuntimeSummary` literal whose
field values are pinned by the surrounding fuzz body. The post-fix fuzz driver
must compute `expected` before the `summarize_recovery_events` call and assert
the post-call `hydration.summary()` equals it.

## Aggregates

`fuzz_recovery_decode` is the only aggregate under change. Its post-fix
shape:

```
inputs  : fuzz payload (data: &[u8])
state   : derived (digest, run, seq, events)
action  : call summarize_recovery_events(&events)
guard   : if non-empty events -> assert_eq!(hydration.summary(), expected)
          if empty events      -> (no assertion; the Ok path is unreachable)
          on Err              -> assert_typed_recovery_error(error)
action  : call recover_runtime_frame_seed_from_events(&events)
guard   : on Err -> assert_typed_recovery_error(error)
outcomes: ok (assertion holds), bug (assertion panics), error (typed-error sink)
```

## Commands

| Command (logical) | Purpose in the post-fix harness |
|---|---|
| Construct `digest` from `blake3::hash(data)` | Stable identifier for `RunAccepted.workflow`. |
| Construct `run` from `data[0]` | The unique identifier of the synthesized run. |
| Construct `events` (single-element or empty) | Branch selector (non-empty / empty). |
| Compute `expected: RecoveryRuntimeSummary` | Exhaustive field map documented above. |
| Call `summarize_recovery_events(&events)` | Production decoder under test. |
| `assert_eq!(hydration.summary(), expected)` | Single equivalence check over all 11 fields. |
| `assert_typed_recovery_error` (twice) | Typed-error sinks already in `errors.rs:57-72`. |

## Events

No new events are introduced. `JournalEvent::RunAccepted { run, seq, workflow }`
is the only event the fuzz driver constructs.

## Policies

1. **Exact-pin policy.** The non-empty branch of `fuzz_recovery_decode` must
   assert structural equality over all 11 `RecoveryRuntimeSummary` fields.
   Single-field disjunctions, sentinel-equality short-circuits, and
   `let _summary = …` patterns are forbidden.
2. **No-sentinel policy.** `RunId::new(0)` is **never** an acceptable value of
   `RecoveryRuntimeSummary.run` when the underlying `events` slice is
   non-empty; the empty-events branch returns it via
   `RecoveryError::NoRecoveryData`, not via a summary struct.
3. **No-production-change policy.** Only `fuzz/src/journal_target/readback.rs`
   is editable in this bead. `apply.rs`, `derive.rs`, `accumulator.rs`,
   `types.rs`, `tests.rs`, `Cargo.toml`, and `fuzz/src/bin/recovery_decode.rs`
   are read-only context.

## Invariants

| ID | Invariant | Where it lives |
|---|---|---|
| INV-1 | `RecoveryRuntimeSummary: PartialEq + Eq + Copy + Debug` (derive set). | `types.rs:546` |
| INV-2 | All 11 fields are structurally fixed by the fuzz body's inputs. | `readback.rs:184-191` (post-fix adds `expected`). |
| INV-3 | `RecoveryError::NoRecoveryData { run: RunId::new(0) }` is the *only* `run` value paired with the empty-events path. | `apply.rs:90`, `accumulator.rs`, `tests.rs:285-302` |
| INV-4 | Multi-run divergence in `events` produces `RecoveryError::ReplayDivergence { detail: "recovery summary received events for multiple runs" }` for the summary path and `"frame seed recovery received events for multiple runs"` for the frame-seed path; the fuzz body never constructs multi-run inputs. | `apply.rs:108-114`, `accumulator.rs:86` |
| INV-5 | `EventSeq::MAX` in any event produces `RecoveryError::ReplayDivergence { detail: "overflow sentinel sequence N is not valid" }`; the fuzz body never constructs such a sequence (`seq = EventSeq::new(1)`). | `apply.rs:115-122` |

## Forbidden States

| State | Why forbidden |
|---|---|
| `assert!` over only `summary.run` in the non-empty branch. | Hides divergence in any of the other 10 fields (counts, workflow, terminal, seq domain). |
| `summary.run == RunId::new(0)` accepted in the non-empty branch. | Sentinel collision with `NoRecoveryData` masks single-run divergence when `data[0] == 0x00`. |
| `summary.run != run && summary.run != RunId::new(0)` accepted silently. | OR-disjunction bugs along with every free-form fuzz divergence would go uncaught. |
| `summary.last_seq != seq` for single-event input. | Production invariant guarantees `last_seq == seq` for one event; any other value is a decoder bug. |
| `summary.workflow != Some(digest)` for `RunAccepted`. | The first event supplies `workflow`; production wires it in (`apply.rs:93-105`, `apply_summary_event_checked`). |
| `summary.steps_started > 0` etc. for a single `RunAccepted` event. | Counters only advance inside `apply_summary_event_checked` for `StepStarted`/`StepFinished`/etc.; the fuzz body emits none. |
| `summary.terminal.is_some()` for a single `RunAccepted` event. | Terminal requires a `RunFinished`/`RunFailed`/`RunCancelled` event. |

## Open Domain Questions (no decisions made; left for downstream owners)

1. Whether the fuzz payload should be extended to also cover multi-run
   divergence and `EventSeq::MAX` overflow. This bead only pins the existing
   single-`RunAccepted` and empty branches.
2. Whether a deterministic `#[test]` wrapper should be added (in
   `fuzz/src/journal_target/readback.rs` under `#[cfg(test)]`) with a known
   payload to lock the exact field assertions against future regressions.
