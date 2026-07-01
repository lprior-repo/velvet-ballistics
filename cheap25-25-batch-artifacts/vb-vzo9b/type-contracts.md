# Type Contracts — vb-vzo9b

> **Scope.** Type-level contract for the post-fix `fuzz_recovery_decode` body.
> The fuzz body must construct an `expected: RecoveryRuntimeSummary` from its
> own inputs and assert `hydrate.summary() == expected` exactly, replacing the
> pre-fix disjunctive `summary.run == run || summary.run == RunId::new(0)`.

## TC-1 — `RecoveryRuntimeSummary` derive set (production, unchanged)

**Source.** `crates/vb_storage/src/recovery/types.rs:546`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryRuntimeSummary { /* 11 fields */ }
```

| Field | Type | `Copy` | `PartialEq` | `Eq` | `Debug` |
|---|---|---|---|---|---|
| `run` | `RunId` | ✅ | ✅ | ✅ | ✅ |
| `first_seq` | `EventSeq` | ✅ | ✅ | ✅ | ✅ |
| `last_seq` | `EventSeq` | ✅ | ✅ | ✅ | ✅ |
| `workflow` | `Option<WorkflowDigest>` | ✅ | ✅ | ✅ | ✅ |
| `steps_started` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `steps_succeeded` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `actions_scheduled` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `actions_resolved` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `suspensions` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `slots_written` | `u64` | ✅ | ✅ | ✅ | ✅ |
| `terminal` | `Option<RecoveryTerminalState>` | ✅ | ✅ | ✅ | ✅ |

**Consequence.** A single `assert_eq!` over the whole struct is exhaustive
over all 11 fields and emits a `Debug` print on failure. There is no need to
manually compare field-by-field.

## TC-2 — Exact expected values (post-fix fuzz body)

**Source.** `fuzz/src/journal_target/readback.rs:183-204` (post-fix).

For every constructed input `(data: &[u8])`:

```
let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
let run    = vb_core::RunId::new(u64::from(data.first().copied().unwrap_or(0)));
let seq    = vb_storage::EventSeq::new(1);
```

The post-fix fuzz body **must** build:

```rust
let expected = vb_storage::recovery::RecoveryRuntimeSummary {
    run,
    first_seq:       seq,
    last_seq:        seq,
    workflow:        Some(digest),
    steps_started:   0,
    steps_succeeded: 0,
    actions_scheduled: 0,
    actions_resolved:  0,
    suspensions:     0,
    slots_written:   0,
    terminal:        None,
};
```

Every field is pinned by derivation:

| Field | Pinned by | Why this exact value |
|---|---|---|
| `run` | fuzz input byte 0 → `RunId::new(u64::from(...))` | The locally constructed `run`. |
| `first_seq` | fuzz body hardcodes `EventSeq::new(1)` | First (and only) event is the one constructed. |
| `last_seq` | same | Fuzz body only constructs one event in the non-empty branch. |
| `workflow` | `blake3::hash(data)` | The single `RunAccepted` event carries `workflow: digest`. |
| `steps_started` | fuzz body emits no `StepStarted` | The single `RunAccepted` does not bump this counter. |
| `steps_succeeded` | fuzz body emits no `StepFinished`/`StepOk` | ditto |
| `actions_scheduled` | fuzz body emits no `ActionScheduled` | ditto |
| `actions_resolved` | fuzz body emits no `ActionResolved` | ditto |
| `suspensions` | fuzz body emits no `BoundarySuspension` | ditto |
| `slots_written` | fuzz body emits no `SlotWritten` | ditto |
| `terminal` | fuzz body emits no `RunFinished`/`RunFailed`/`RunCancelled` | ditto |

## TC-3 — Single equivalence assertion (post-fix fuzz body)

The post-fix non-empty branch **must** be exactly:

```rust
match vb_storage::recovery::summarize_recovery_events(&events) {
    Ok(hydration) => {
        if !events.is_empty() {
            let run_summary = hydration.summary();
            assert_eq!(run_summary, expected);
        }
    }
    Err(error) => assert_typed_recovery_error(error),
}
```

| Forbidden construction | Reason |
|---|---|
| `assert!(run_summary.run == run)` | Hides divergence in any of `first_seq/last_seq/workflow/.../terminal`. |
| `assert!(run_summary.run == run \|\| run_summary.run == RunId::new(0))` | Pre-fix defect (P1 bug). Two distinct `RunId` values accepted; `RunId(0)` is the empty-events sentinel. |
| `assert!(run_summary.first_seq == seq && run_summary.last_seq == seq && ...)` chain | Brittle, easy to drop a field silently. The struct already derives `PartialEq`. |
| `assert!(matches!(run_summary, RecoveryRuntimeSummary { run, .. }))` | Does not check the non-`run` fields. |
| `let _summary = ...;` | No check at all. |
| `dbg!`/`println!` instead of `assert_eq!` | No panic on failure. |

## TC-4 — Empty-events path (unchanged contract)

The `events.is_empty()` branch is unchanged: `summarize_recovery_events(&[])`
returns `Err(RecoveryError::NoRecoveryData { run: RunId::new(0) })` (see
`apply.rs:89-91` and `tests.rs:285-302`). The post-fix fuzz body still relies
on `assert_typed_recovery_error` to consume that error and **must not**
attempt to dereference `hydration` in this branch.

## TC-5 — `assert_typed_recovery_error` (unchanged contract)

Two `assert_typed_recovery_error` calls remain in the post-fix body:

1. `Err(error) => assert_typed_recovery_error(error)` after the
   `summarize_recovery_events` call (`readback.rs:199`).
2. `assert_typed_recovery_error(error)` after the
   `recover_runtime_frame_seed_from_events` call (`readback.rs:202`).

Both already enumerate every legal `RecoveryError` variant (see
`fuzz/src/journal_target/errors.rs:57-72`). The post-fix body must not bypass
either sink.

## TC-6 — Sentinel-binding invariant

`RunId::new(0)` is bound to `RecoveryError::NoRecoveryData` (production
invariant, see `INV-3` in `domain-model.md`). The post-fix `expected` value
**must not** be `RunId::new(0)` when `data` is empty, because the empty branch
returns `Err(...)`, not `Ok(...)` — there is no `expected` literal to build
in that branch.

## TC-7 — Pin pair (assertion + Debug print)

If the post-fix `assert_eq!` panics, the cargo test runner prints:

```
assertion `left == right` failed
  left:  RecoveryRuntimeSummary { ... }
  right: RecoveryRuntimeSummary { ... }
```

The `Debug` derive on the struct is what makes this output actionable for
downstream observers. Downstream test authors must therefore refrain from
clobbering the `Debug` derive.

## Type contract checklist (per `references/type-contract-checklist.md`)

- [x] No new primitive-obsession leak (no `bool` flags, no `String` IDs).
- [x] No `Option` lifecycle state — both `workflow` and `terminal` are
      domain-meaningful optional fields, not lifecycle flags.
- [x] No new smart constructor; `RecoveryRuntimeSummary` already has the
      required derive set.
- [x] No new error variant; production errors already enumerated in
      `assert_typed_recovery_error`.
- [x] No typestate needed — the fuzz body is a straight-line procedure.
- [x] Production-binding: `RecoveryRuntimeSummary`, `RecoveryHydration`,
      `summarize_recovery_events`, `recover_runtime_frame_seed_from_events`
      are all referenced by production path and unchanged.
- [x] No `unsafe`, no `unwrap`/`expect`/`panic` in the contract.
