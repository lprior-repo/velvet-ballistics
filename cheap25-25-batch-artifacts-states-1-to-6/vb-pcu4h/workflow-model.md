# Workflow Model — vb-pcu4h

- bead_id: vb-pcu4h
- artifact_owner: rust-contract
- workflow_kind: test assertion-state machine (no production workflow change).

## Why a workflow model for a test-edit bead

The bead's fix changes how the recovery reducer's *terminal state* is asserted. The test workflow is therefore a small state machine with the following phases per primary target:

```
Setup → Construct events → Recover seed → Assert (Vec-equal) → Pass / Fail
```

Within each phase there is one workflow hazard that this model captures explicitly: the silent-pass mode of `matches!(Ok(recovered) if <bool>)` (Test A).

## Phase table

| Phase | Test A (line 437) | Test B (line 621) | Test C (line 743) |
|-------|--------------------|--------------------|--------------------|
| Setup | `RunId::new(61)` | `RunId::new(70)`; build ticket; `sample_digest(0xA1)` | `RunId::new(72)`; build ticket; `digest(72)`; `sample_digest(0xA1)` |
| Construct events | `[ActionScheduled { step: 3, action: 9, attempt: 1 }]` | `[ActionScheduledTicket { ticket.step=5, action=11, output=SlotIdx::new(9) }]` | `[RunAccepted, StepStarted { step: 6 }, ActionScheduledTicket { ticket.step=6, action=17, output=SlotIdx::new(8) }]` |
| Recover seed | `recover_runtime_frame_seed_from_events(&events)` | `.expect("schedule-only event must produce a seed")` (already present) | `.expect("post-schedule crash must produce a recoverable seed")` (already present) |
| Assert (legacy) | `matches!(seed, Ok(recovered) if .iter().any(...) && unsupported.pending_actions)` | `.iter().any(...)` | `.iter().any(...)` |
| Assert (replacement) | `expect("…")` THEN `assert_eq!(pending_actions, vec![…])` AND `assert!(unsupported.pending_actions)` | `assert_eq!(pending_actions, vec![…])` (existing `.expect` retained) | `assert_eq!(pending_actions, vec![…])` (existing `.expect` retained) |
| Pass / Fail | Test fails on Err(_) with context; test fails on length drift; test fails on field drift; test fails on unsupported-flag drift | Test fails on length drift; test fails on field drift | Test fails on length drift; test fails on field drift |
| Auxiliary asserts (preserved) | (none) | `slot_count == 10`, `step_count == 6`, `seed.steps.iter().any(|e| e.step == 5 && e.state == Running)`, `summary.actions_scheduled == 1` | `slot_count == 9`, `step_count == 7`; second recovery call (`let _ = frame_recovery;`) is preserved verbatim |

## Guards

- G-1 — `seed` MUST be `Ok(_)`. Replacement: `let recovered = seed.expect("…")` so an `Err(_)` panics with context and is surfaced as a test failure.
- G-2 — `recovered.pending_actions` MUST equal exactly the constructed vec. Replacement: `assert_eq!(recovered.pending_actions, vec![RecoveredPendingAction { step, action }])`. The constructed vec is the literal expected output, sorted by `(step, action)` (single-element vec; sort order is trivial but canonical).
- G-3 — (Test A only) `recovered.unsupported.pending_actions` MUST be `true`. The accumulator flips this flag because `pending_actions` is non-empty after recovery; the boolean is a separate derivation from G-2 and must be exercised.

## Transitions (recovery reducer's effect on `pending_actions`)

The bead does not modify the reducer. For documentation:

```
RecoveryFrameSeedAccumulator::new()  --(ActionScheduled)-->  pending_actions = {(action, step)}
RecoveryFrameSeedAccumulator::new()  --(ActionScheduledTicket)-->  pending_actions = {(action, step)}
                                                                ↓
recovered_pending_actions(set)       --sort by (step, action)-->  Vec<RecoveredPendingAction>
```

After recovery of a single schedule-only event, the vec MUST have exactly one entry. This is the workflow's terminal state for the fixture; the test asserts it directly.

## Outcomes

- OK — `seed = Ok(recovered)` AND `recovered.pending_actions == vec![RecoveredPendingAction { step, action }]` AND (Test A only) `recovered.unsupported.pending_actions == true`. All three PRIMARY targets must reach this state for the test to pass.
- FAIL-1 (silent-pass mode, pre-fix Test A) — `seed = Err(_)` causes `matches!` to return `false`; the outer `assert!` evaluates `assert!(false)` which panics. Wait — actually `matches!(Err(_), Ok(_) if …)` returns `false`; the outer `assert!(false)` panics, so the test does fail. The audit's "silent-pass" wording therefore does NOT apply to `assert!(matches!(seed, Ok(_) if …))` because the outer `assert!` still panics on `false`. **Correction**: the silent-pass reading in the codebase-map applies only if the assertion were `let _ = matches!(...)` or `if let Ok(_) = seed { … }` without an outer check; Test A uses `assert!(matches!(…))` which DOES fail on `Err`. The contract nonetheless RECOMMENDS the `.expect()` rewrite for clarity and so the panic message names the failure mode.
- FAIL-2 (drop-all) — `recovered.pending_actions == vec![]`; `assert_eq!` against the literal vec with length 1 panics with the Vec diff. Caught.
- FAIL-3 (phantom-duplicate) — `recovered.pending_actions == vec![RecoveredPendingAction { step, action }, RecoveredPendingAction { step, action }]`; `assert_eq!` against length-1 vec panics with the Vec diff. Caught.
- FAIL-4 (field-drift) — `recovered.pending_actions[0].step != S` OR `.action != A`; `assert_eq!` panics with the struct diff (per-field, since both sides are `RecoveredPendingAction`). Caught.
- FAIL-5 (unsupported-flag drift, Test A only) — `recovered.unsupported.pending_actions == false`; the boolean assertion panics. Caught.

## Idempotence

The reducer is idempotent: replaying the same event sequence produces the same `RecoveryFrameSeed`. The test is therefore idempotent under rerun. No temporal hazard.

## Cancellation / retry

Not applicable — the bead is a synchronous unit test.

## Terminal states

- PASS — fixture constructed, recovery `Ok`, Vec-equality holds (and boolean holds for Test A), all auxiliary assertions pass.
- FAIL — any of the FAIL-1..5 outcomes above, or any compilation failure introduced by the test edit.

## Cross-references

- Reducer (read-only): `crates/vb_storage/src/recovery/replay/summary/derive.rs:69-83` (entry points) and `:287-296` (Vec assembly).
- Accumulator (read-only): `crates/vb_storage/src/recovery/replay/summary/accumulator.rs:35,68` (HashSet field and init).
- Verus mirror (read-only): `verification/verus/production_inner/replay_invariants_production.rs:253-256`.