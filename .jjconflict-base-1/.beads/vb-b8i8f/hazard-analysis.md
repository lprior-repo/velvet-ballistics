# Hazard Analysis: vb-b8i8f Cancel/Kill Lattice Recovery

## Temporal Hazards

| Hazard | Consequence | Contract control |
|---|---|---|
| Cancel and kill both queued for same live run | Duplicate terminal events or counter increments. | Single terminal winner; second command returns typed rejection. |
| Finish/fail races with cancel/kill in queue ordering | Terminal regression. | Terminal state is absorbing; transition from terminal to another terminal is illegal. |
| Timer fires after cancel/kill | Resurrects or mutates terminal run. | Pending timer removed; stale fire rejected. |
| Action completion after kill | Slot write or action journal event after terminal. | Action authority invalidated by live state removal; stale completion rejected. |
| Storage append fails after in-memory cleanup | Durable ledger lacks terminal but memory says terminal. | Terminal commit must be ordered with append success or rollback-safe. |

## Rust-Core Invariant Hazards

- `runs.contains_key` followed by separate removal can drift from journal append if append fails.
- `terminal_runs` as a set loses terminal kind unless journal evidence bridges it.
- Silent `Ok(())` for missing/already-terminal hides incorrect caller behavior and weakens tests.
- `discard_journal_sequence(run)` after failed append could corrupt retry/replay assumptions if not ordered carefully.

## Storage / Codec Hazards

- `RecordKind::RunKilled=28` exists but validation range `10..=27` rejects it.
- Encode-side fix without decode-side fix yields unreadable ledgers.
- Test fixture ranges that assert `10..=27` journal kinds will preserve the bug unless updated.
- Master storage table currently omits ID 28, creating documentation drift against code and bead scope.

## Bounded-State Hazards

- Cancel/kill must not drain unbounded command queues.
- Cleanup must not allocate unbounded diagnostic strings in hot paths.
- Journal append must respect bounded writer queue/backpressure and surface typed errors.

## Concurrency / Scheduling Hazards

- Even single-threaded shard processing has schedule interleavings through command order: cancel vs action completion, kill vs timer fired, finish vs cancel.
- Public API may enqueue commands before processing; live validation must happen at processing time, not only at enqueue time.
- Snapshot/inspect must not create time-of-check/time-of-use assumptions for cancel/kill correctness.

## Hostile / Invalid Input Hazards

- Unknown run IDs sent repeatedly through public API must not produce terminal journal spam.
- Malformed persisted envelopes with kind 28 under the wrong magic must still fail family validation.
- Attempt zero for killed event must remain invalid.

## Release/API Hazards

- Adding public `Runtime::kill_run` changes public API and must have acceptance coverage.
- Reclassifying cancel missing/already-terminal from `Ok(())` to `Err` may require updating existing tests that assumed no-op semantics.
- Storage kind 28 admission affects wire compatibility; regression tests must show old valid kinds remain accepted and invalid kinds remain rejected.

## Remaining Illegal-State Risks

- Existing storage of terminal markers as a set may still make terminal kind unrepresentable without journal lookup.
- Existing public API cannot synchronously know shard processing result if it only enqueues; downstream design must decide whether API errors are enqueue-time only or processing-time observable through tick/result surfaces.
- If storage append occurs before cleanup but cleanup fails, implementation needs an explicit recovery plan for partially terminalized state.
