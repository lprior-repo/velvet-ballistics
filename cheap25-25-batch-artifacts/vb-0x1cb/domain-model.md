# Domain Model — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: production_file (transitions.rs) + gate_config (ignored-fallible-results.allow) + test_files (lifecycle_tests/chunk_005.rs, chunk_008.rs)
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

## Ubiquitous Language

| Term | Definition | Anti-pattern rejected |
|------|------------|------------------------|
| **Run lifecycle transition** | The act of moving a `RunId` between two `RuntimeState` slots in `Shard::runtime_states` (`runtime_state_insert`, `runtime_state_remove`). | Calling `runtime_states.insert(...)` from outside the `apply`/`keep_run`/`finish_run`/`fail_run_state`/`await_action`/`await_timer` family. |
| **Durable finalization** | A journal-append event that closes a run: `RunJournalEvent::RunFinished { run, result }` (happy close) or `RunJournalEvent::RunFailed { run }` (terminal close). The event MUST land in the journal before any in-memory state change that mirrors a closed run is observable. | Returning to the caller, incrementing counters, or releasing frame pool slots before the journal accept. |
| **Best-effort rollback** | When the durable finalization event is rejected by the journal, a compensating in-memory `run_state_insert` is issued so the run's terminal state remains consistent. It is "best effort" because the slot reservation inside `run_state_insert` itself may fail. | Permanently dropped `Result` from `run_state_insert`; that is the violation this bead repairs. |
| **Discarded fallible result (DISCARD-006)** | A `let _ = ...` or `match { Ok(_) \| Err(_) => {} }` pattern that destructures a `Result<_, E>` and discards `E`. The source-gate `scripts/check-ignored-fallible-results.sh` classifies this as DISCARD-006 when the receiving call returns `Result<_, E>`. | `let _ = self.run_state_insert(run, state);` followed by `return Err(error);` — both errors in flight, only one surfaced. |
| **Primary error** | The error from the durable finalization journal append. This is the error that the caller MUST see — it identifies the durability boundary that was crossed (or not crossed). | Returning a synthesized secondary error in place of the primary. |
| **Secondary error** | The error from the rollback `run_state_insert` after a primary error. This is the error that has been discarded under the current code and that this bead's contract binds into the runtime diagnostic path. | `let _ = ...` that drops it; this is the illegal-state-to-eradicate. |
| **Runtime diagnostic path** | The dual channel of typed errors (`RuntimeError`) and trace observability (`TraceEvent`). Either channel is sufficient evidence that the secondary error was bound; this contract picks **TraceEvent observability** for the secondary while preserving the primary's typed-error surface. | Conflating both channels into a single `Result::Err` and losing the primary; or bolting on a private `log::warn!` which would be invisible to the diagnostic surface that proof/test lanes exercise. |
| **`TraceEvent::RunRollbackFailed`** | A new `#[non_exhaustive]` `TraceEvent` variant that records the dual-failure shape (`run`, `site`, `primary: Arc<RuntimeError>`, `secondary: Arc<RuntimeError>`). | A `String` reason field — `RuntimeError` is already canonical, cheap to `Arc`-clone, and carries symbolic/diagnostic codes. |
| **`RollbackSite` enum** | Two `non_exhaustive` cases: `FinishRun` and `FailRunState`. Used as the `site` payload inside `RunRollbackFailed` to keep both rollback sites consistent without stringly branching on `crate::shard::transitions::*` paths. | Embedding the source line range or a `&'static str` function name; both can drift during refactors. |
| **Terminal fence** | The combination of `runtime_states` entry removal, frame release, and journal-sequence discard that commits a run to a terminal state. Once a run crosses the terminal fence under `finish_run` or `fail_run_state`, those auxiliary maps must NOT be observably divergent from the journal — frame invariants from `LegacyStepFailsJournal` (`chunk_004.rs:240-319`) apply here. | Inconsistency between the journal-rejected `RunFinished`/`RunFailed` event and the in-memory terminal slot; this is the invariant under test. |

## Entities & Value Objects

```
RunId (vb_core::ids::RunId)
RunState (vb_runtime::shard::types::RunState)
    .frame         : RunFrame
    .journal_seq   : EventSeq
RuntimeEvent (vb_runtime::shard::types::RuntimeEvent)
    Submit | Resume | ResumeRollback | DriveContinue | AwaitAction
    | AwaitTimer | Fail | TerminalRemove | DriveFinished
Shard (vb_runtime::shard::types::Shard)
    .runtime_states : BTreeMap<RunId, RunState>
    .journal        : SharedRuntimeJournal
    .terminal_runs  : BTreeSet<RunId>
    .trace_ring     : TraceRing
    .runs           : BTreeMap<RunId, RunState>     // capacity-bounded via reserve_run_state_slot
RuntimeError (vb_runtime::error::RuntimeError)
    QueueFull | RunNotFound | RunAlreadyExists | JournalFull { .. }
    | StorageJournalAppend { source: Arc<vb_storage::JournalError> }
    | Core { source: Box<vb_core::errors::CoreError> }     // wraps CoreError::InternalInvariantViolation
    | ... (42 other variants) ...
TraceEvent (vb_runtime::trace::event::TraceEvent)
    RunFinished { run } | RunFailed { run } |
    ... 11 other variants ... |
    // NEW: RunRollbackFailed { run, site, primary, secondary }
RollbackSite (vb_runtime::trace::event::RollbackSite)     // NEW
    FinishRun | FailRunState
```

## Aggregates

`Shard` is the single aggregate root for all run lifecycle transitions in this bead. The aggregate boundary is:

- **Inside**: `runtime_states`, `runs`, `terminal_runs`, `pending_timers`, `pending_actions`, `journal_sequences`, `trace_ring`, `counters`, frame pool (`release_frame`).
- **Outside**: `SharedRuntimeJournal` (FJALL-backed durable log), `SharedRuntimeConfig`, frame pool provider.

`finish_run` and `fail_run_state` are members of this aggregate that orchestrate a *two-phase* commit on the aggregate: durable journal append first, then aggregate field mutations.

## Commands

| Command | Caller | Result | Side effects |
|---------|--------|--------|--------------|
| `Shard::finish_run(run, state)` | `apply_drive_result`, `ShutdownCoordinator::finalize_active_runs` | `RuntimeResult<()>` returning the **primary** journal-append error if rejected; rollback best-effort. | Terminal fence mutation under terminal success; rollback `run_state_insert` under primary rejection; new `TraceEvent::RunRollbackFailed` if both fail. |
| `Shard::fail_run_state(run, state)` | `apply_drive_result` failure branch, `RetryClassifier`, `PoisonPill::drop` | Same. | Same shape; distinct `RollbackSite::FailRunState`. |
| `Shard::apply(run, event)` | dispatcher | `RuntimeResult<()>` | Single routing method for non-terminal events. Already clean. |

## Events

| Event | Channel | Producer | Trigger |
|-------|---------|----------|---------|
| `TraceEvent::RunFinished { run }` | Observability | `finish_run` happy path | After terminal fence mutation succeeds. |
| `TraceEvent::RunFailed { run }` | Observability | `fail_run_state` happy path | After terminal fence mutation succeeds. |
| `TraceEvent::RunRollbackFailed { run, site, primary, secondary }` (**NEW**) | Observability | `finish_run` / `fail_run_state` | Both journal append AND rollback `run_state_insert` fail in the same call. |

## Invariants

1. **Primary-error wins (I1).** When `append_journal_event(...)` returns `Err(primary)` the caller MUST see `Err(primary)` on `Result::Err`. The secondary error MUST NEVER replace the primary error in the function's return value.
2. **Secondary bound (I2).** When the rollback `run_state_insert(...)` returns `Err(secondary)` the secondary MUST be bound to a name (no `let _ = ...`; no `Ok(_)|Err(_)=>{}`) and MUST be visible on the runtime diagnostic path. Visibility under this contract = `trace_ring.push(TraceEvent::RunRollbackFailed { .. })`.
3. **No silent swallow (I3).** A `DISCARD-006` row in `scripts/ignored-fallible-results.allow` for `crates/vb_runtime/src/shard/transitions.rs` SHALL be removed once `transitions.rs:100` and `:202` are repaired. No justification marker for "best-effort rollback" survives.
4. **Terminal fence parity (I4).** When the rollback succeeds, the run's terminal-state slot MUST equal the pre-call state in `runtime_states` (the rollback restored it). When the rollback fails, the run is in a divergent state — `RunRollbackFailed` records this fact so an operator can rebuild the run before the next persistence boundary. This mirrors the `LegacyStepFailsJournal` invariant (`chunk_004.rs:240-319`) of "frame MUST remain unchanged after a rejected journal append".
5. **`Arc`-bounded allocation (I5).** `TraceEvent::RunRollbackFailed` carries `Arc<RuntimeError>` not `Box`. This keeps the size of `TraceEvent` bounded by `Arc` indirection (`size_of::<Arc<…>>() == size_of::<usize>`) and avoids implicit heap allocation per recorded event — the trace ring is capacity-bounded.
6. **No public-API churn on `RuntimeError` (I6).** This bead does not add a new `RuntimeError` variant. The secondary surface lives on the `TraceEvent` channel; the typed-error path is preserved verbatim for `StorageJournalAppend { source }`. (Choosing the `Core { InternalInvariantViolation { reason } }` wrap path would require extending `diagnostics.rs:61-64`, `130-133`, and 7 test files; that blast radius is rejected here in favor of the trace observability path.)

## Policies

| Policy | Statement | Reason |
|--------|-----------|--------|
| **P1 (Surface channel)** | The secondary error is surfaced on the `TraceEvent` observability channel via `TraceEvent::RunRollbackFailed`. | TraceEvent is already a non-error channel used for the success-path counterparts (`RunFinished`, `RunFailed`). Adopting it for the dual-failure case keeps `RuntimeError` enum stable and avoids `diagnostics.rs` `match` arm churn. |
| **P2 (No diagnostic-code mutation)** | `RuntimeError::diagnostic_code()` and `RuntimeError::symbolic_code()` are NOT touched. | The primary `StorageJournalAppend { .. }` already maps to `STORAGE_JOURNAL_APPEND_FAILED_CODE` (0x2008). Touching the match arms would re-trigger the `runtime_error_diagnostic_codes_are_unique` test in `tests_diagnostics.rs:64`. |
| **P3 (No `unsafe`)** | Per AGENTS.md, the runtime is zero-panic / zero-`unsafe`. This contract does not introduce `unsafe`. | All implementations are pure safe Rust. |
| **P4 (No `Arc<Mutex<…>>` indirection for `TraceEvent`)** | The trace ring's existing push mechanism is reused. | `TraceRing::push` is already thread-safe per its consumer contract; we do not mutate the ring's interface. |
| **P5 (Lane profile)** | Rust-local + concurrency-empty. Lane decisions: `kani`, `verus`, `flux-rs`, `proptest`. Lanes omitted: `loom` (single-shard, sequential rollback paths), `cargo-fuzz` (no parser/codec surface; this is type-level error plumbing). | Bead scope per `delivery-scope.jsonl` row 1. |

## Forbidden / Illegal States

The bead actively makes the following states unrepresentable under the repaired code:

1. **Forbidden S1 — DISCARD-006 in transitions.rs.** Under the script's gate, the line `let _ = self.run_state_insert(run, state);` at `transitions.rs:100` and `:202` is a `JustifiedException` row in the allow file. After the repair, `scripts/check-ignored-fallible-results.sh` SHALL exit 0 for `transitions.rs` without reading any allow row.
2. **Forbidden S2 — Falling into `Ok(_)|Err(_)=>{}`.** No arm of any `match` in `transitions.rs` may destructure both `Ok` and `Err` and discard. (The original bead description at the time of filing referenced `Ok(_)|Err(_)=>{}` at line 146; today that pattern does not exist in the file, but the contract forbids its future occurrence on the rollback sites.)
3. **Forbidden S3 — Primary masked by secondary.** `finish_run` / `fail_run_state` SHALL NOT return `Err(secondary)` if a `primary` was available. The function's `Result::Err` carries primary only.
4. **Forbidden S4 — Secondary observable only via `eprintln!`.** The secondary error MUST land on `TraceEvent`, not on stderr. `eprintln!` in error paths is forbidden by AGENTS.md.

## Open Domain Questions (forwarded to proof/test lanes)

1. **Q1 — `TraceEvent` variant vs Core wrap**: this contract chose TraceEvent over `RuntimeError::Core { source: CoreError::InternalInvariantViolation { .. } }`. The proof-planner may decide to add a secondary Core-wrap for the case where `Arc::clone` is unavailable. Resolve before any harness commit.
2. **Q2 — Should the secondary RollbackFailed event still be emitted when the primary error is `StorageJournalAppend(QueueFull)` (slot exhaustion overlap)?** Yes under this contract: the disk and in-memory slot can both be full at once, and the secondary error type is `RuntimeError::ActiveRunCapacityExceeded { .. }` which is independent. Recording both keeps the diagnostic honest.
3. **Q3 — `size_of::<TraceEvent>`**: `Arc<RuntimeError>` ×2 + `RollbackSite` (1 byte) + `RunId` (8 bytes typical). Acceptable; still well under any cache-line budget. The proof-writer should bound `size_of::<TraceEvent::RunRollbackFailed>` in a Flux refinement if surface area is added.
4. **Q4 — Lifecycle inclusion for the new test chunk**: `lifecycle_tests/chunk_008.rs` is referenced in `delivery-scope.jsonl` but does not yet exist on disk. The test-writer lane MUST create it under `lifecycle_tests/` (skip-list at `check-ignored-fallible-results.sh:62-72`), include it from `crates/vb_runtime/src/shard/lifecycle.rs`, and mirror the `LegacyStepFailsJournal` pattern.

## Cross-references

- `error-taxonomy.md` — the `RuntimeError` and `TraceEvent` variants used by this contract.
- `workflow-model.md` — the dual-failure state machine this contract imposes.
- `boundary-map.md` — where the contract enters (`Shard` boundary) vs where it leaves (`trace_ring`, `RuntimeError::Err`).
- `hazard-analysis.md` — temporal/concurrency/observability hazards tied to the dual failure.
- `contract.md` — the single canonical surface statement this bead commits to.
