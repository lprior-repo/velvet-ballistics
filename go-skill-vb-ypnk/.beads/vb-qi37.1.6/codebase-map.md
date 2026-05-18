# vb-qi37.1.6 codebase map

State 2 explore artifact for `runtime/recovery: Crash restart integration evidence`.

## Inputs read

- Bead DB command from isolated workspace: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.1.6 --json` returned the in-progress task and dependencies.
- State/baseline: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/.beads/vb-qi37.1.6/STATE.md`, `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/.beads/vb-qi37.1.6/baseline-report.md`.
- Acceptance target: end-to-end crash/restart recovery evidence across persisted headers, journal events, snapshots, live-frame hydration, waits/asks/actions, and collect pagination; failures must be typed.

## Primary production scope

### Storage recovery

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/mod.rs`
  - Re-exports the recovery API: `recover_full_journal`, `recover_snapshot_plus_tail`, `recover_runtime_summary`, `recover_runtime_frame_seed`, `hydrate_run_frame`, `hydrate_run_frame_from_events`, `RecoveryError`, `RecoveryHydration`, `RecoveryFrameSeed`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/types.rs`
  - Key types: `RecoveryError`, `RecoveryRuntimeSummary`, `RecoveryHydration`, `RecoveryFrameSeed`, `RunSnapshot`, `ActionReplayTracker`, `UnsupportedRecoveryState`.
  - Typed failure variants already include `NoRecoveryData`, `CorruptSnapshot`, `ReplayDivergence`, digest mismatch variants, `NonIdempotentActionBlocked`, and `FrameDimensionOverflow`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/recover.rs`
  - High-level entrypoints: `recover_runtime_summary`, `recover_runtime_frame_seed`, `recover_all_incomplete_runs`, `verify_digests`, `check_workflow_source_digest`, `check_compiled_ir_digest`.
  - `recover_all_incomplete_runs` scans persisted run headers via `journal.run_headers()` and rejects headers with no events as `NoRecoveryData`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/replay/core.rs`
  - `replay_events` filters live state to the latest attempt while preserving diagnostic output; detects step-order divergence and non-idempotent action replay.
  - `recover_full_journal`, `load_snapshot`, `recover_snapshot_plus_tail`, `extract_terminal` are the restart replay core.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/replay/summary.rs`
  - `summarize_recovery_events` and `recover_runtime_frame_seed_from_events*` construct summaries and live-frame seeds from ordered events.
  - Tracks steps, slots, pending actions, terminal state, suspensions, unsupported state, digest mismatch with compiled workflow.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/hydrate.rs`
  - Snapshot + tail and event-only `RunFrame` hydration.
  - Validates snapshot run id, tail run ids, tail seq after snapshot, non-empty evidence, dimensions, slot writes, pc, and parallel in-flight counters.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/recovery/hydrate_support.rs`
  - Snapshot slot/taint decode, dimension derivation, tail-event application, parallel in-flight reconstruction.
  - Current tail application preserves existing snapshot taint for slot writes and defaults new-slot taint to clean.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/events.rs`
  - Durable event vocabulary: `RunAccepted`, `RunAdmission`, `StepStarted`, `StepSucceeded`, action lifecycle, `SlotWrittenEvent { value, extra }`, waits, asks, retry, terminal events, and lifecycle `RunResumed`/`RunRetried`/`RunAnswered`.
  - Important detail: lifecycle events do not carry ordered `seq`; recovery summaries ignore them.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/journal/mod.rs` and submodules under `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/journal/`
  - Fjall journal open/append/replay/header/snapshot surface. Relevant for crash simulation by drop/reopen and persisted header checks.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/src/snapshots.rs`
  - Snapshot persistence surface; pair with `load_snapshot` and `hydrate_run_frame` for snapshot+tail evidence.

### Runtime recovery and primitives

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/recovery.rs`
  - Runtime boundary: `RuntimeRecoveryBoundary`, `DurableFrameRecoveryBoundary`, `SummaryRecoveryBoundary`, `recovery_boundary_from_hydration`.
  - Rejects unsupported live-frame state as `RuntimeError::InvalidRecoveryHydration`; summary-only hydration returns `UnsupportedFullRecoveryHydration`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/primitives/wait_ask.rs`
  - Wait/ask/answer primitive handlers. `wait_until`, `wait_event`, `ask`, `ask_resume` mutate `RunFrame` but host/journal integration must prove durable wait/ask restart continuity.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/primitives/collect.rs`
  - `CollectPaginationState`, `CollectStates`, `capture_extra`, `hydrate_extra`, `hydrate_journal_events` are the collect pagination recovery surface.
  - `SlotWrittenEvent.extra` is used to rehydrate collect state; corrupt/empty/wrong identity extras map to `EngineError::CollectExtraHydrationFailed` variants.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/action.rs`
  - `ActionRegistry` and `ActionTicket`-producing generic dispatch. Restart evidence must cover pending and resolved action tickets without duplicate re-execution.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/journal.rs` and chunks under `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/journal/`
  - Runtime-to-storage event emission surface. Needed if downstream tests drive runtime instead of storage-level synthetic events.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/engine/*`
  - Engine signal and execution path that turns primitives/actions into suspended/continued runtime state.

### Core state

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_core/src/frame.rs`
  - `RunFrame`, `StepState`, slot/taint/pc/executed/parallel counters. Hydration applies recovered steps, slots, pc, and counters here.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_core/src/action.rs`
  - `ActionTicket`, action contract, idempotency/retry policy types.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_core/src/value.rs`
  - `SlotValue`, `Taint`, and postcard-encoded durable slot values.

### CLI/integration surface

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/src/run.rs`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/src/storage.rs`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/src/lifecycle.rs`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/src/commands_journal.rs`
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/src/commands_status.rs`
  - These are likely needed only if acceptance evidence must be command-level instead of crate-level integration.

## Existing test/evidence anchors

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/tests/recovery_integration.rs`
  - Existing crash/reopen storage integration tests for full round-trip, partial writes, summary reconstruction, action tracker, strict/journaled durability.
  - Many current helper events use `SlotWrittenEvent { value: None, extra: None }`, so they prove event presence but not full slot-value/taint hydration.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/tests/replay_resume.rs`
  - Reopen replay for wait/retry tail, deterministic double replay, and sequence-gap rejection.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/tests/vb_h6ix_integration.rs`
  - Latest-attempt replay integration, stale terminal handling, wait/ask attempt cases, determinism.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`
  - Contract tests for event-only frame seed recovery, slot values/taint, unsupported flags, queue drain to journal.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_runtime/src/collect_tests.rs`
  - Extensive collect pagination unit/contract tests, including `capture_extra` and journal-event hydration helpers.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/tests/lifecycle_integration.rs`
  - CLI/library lifecycle journal tests for cancel/resume/retry/answer event append and replay.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/velvet_ballastics/tests/mode_activation_integration_tests.rs`
  - CLI journal open behavior and durability mode assertions.

## Required downstream scope

1. Add or strengthen integration evidence, not production behavior unless contract/test phases discover a real gap.
2. Test crash simulation via actual `FjallJournal` drop/reopen, not in-memory-only replay.
3. Cover at least these restart cuts:
   - after persisted run header/admission but before any step terminal state;
   - after slot write with postcard value and durable taint/extra;
   - while waiting;
   - while asking and after answer event/write;
   - with pending action ticket and with resolved action ticket;
   - mid-collect pagination with durable `extra` and current page validation;
   - snapshot base plus tail events.
4. Assert no lost run id, seq ordering, pc, step states, slot values, taint, pending/resolved action identity, wait/ask state, collect cursor/page, terminal/result, or digest binding.
5. Assert typed failures for missing journal data, corrupt snapshot, sequence gap/tail-before-snapshot, digest mismatch, corrupt slot value, corrupt collect extra, and unsupported pending action/live-frame hydration.

## Risks and open questions

- `SlotWrittenEvent` has `value` and `extra` but no explicit taint field; `summary.rs` derives taint from `recovered_slot_taint(slot_value, extra)`. Downstream must verify this is intentional and enough for exact taint recovery.
- `hydrate_support.rs` defaults new tail slot taint to `Clean` when no snapshot taint exists. This is a taint-loss risk for snapshot+tail hydration unless tail value/extra proves taint.
- `RunResumed`, `RunRetried`, and `RunAnswered` lack sequence numbers and are ignored by summary recovery. If acceptance requires durable ordered recovery of lifecycle events, this is a likely gap.
- `RecoveryFrameSeed.unsupported.pending_actions` intentionally blocks live-frame hydration with pending actions. Evidence may need to show typed fail-closed, not successful pending-action frame hydration.
- Collect pagination state is runtime side-table state, not part of `RecoveryFrameSeed`; evidence must explicitly call `CollectStates::hydrate_journal_events` or a higher-level restart path.
- Existing storage tests prove many storage replay properties but not one cohesive e2e restart path across headers + journal + snapshot + runtime boundary + collect state.
- Public CLI-level restart command surface is UNKNOWN from this explore; crate-level integration may be the smallest reliable acceptance proof.

## Recommended downstream owners

- Contract/proof planner: focus on temporal/durability invariants for persisted-before-ack, latest-attempt replay, snapshot-tail ordering, and fail-closed unsupported state.
- Test writer: add crate-level integration tests under `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/vb_storage/tests/` and/or `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1-6/crates/workspace_tests/tests/`; use actual Fjall reopen.
- Implementation owner: only touch recovery/runtime storage paths if tests expose missing behavior.

## Excluded scope

- YAML parsing/compile recovery chain is covered by dependent `vb-core-yaml-e2e-chain`; do not expand this bead into YAML reparsing.
- UI, doc reconciliation, naming scan, and Makepad code are unrelated.
- No production code, tests, or proof files were modified during this explore state.
