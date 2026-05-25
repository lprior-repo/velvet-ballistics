# Codebase map: vb-qi37.1

State 2 exploration timestamp: 2026-05-15T19:47:44Z  
Isolated workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1`  
Source checkout for bd reads only: `/home/lewis/src/velvet-ballistics`

## Bead scope from bd

Bead: `vb-qi37.1` - `runtime/storage: Complete full live-frame recovery hydration`.

Acceptance requires crash recovery to reconstruct:

- program counter, slot handles, slot taints, step state, journal sequence, action tickets, waits, asks, and terminal result;
- non-empty live frame state from journal/snapshot data, or a typed fail-closed recovery error;
- digest mismatch and corrupt/incomplete journal detection;
- test evidence for crash-before-ack, crash-after-ack, corrupt journal, and snapshot+journal replay.

Important dependency context from `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.1 --json`:

- Closed child work already covers deterministic step lifecycle (`vb-qi37.1.1`), slot writes with taint (`vb-qi37.1.2`), frame hydration (`vb-qi37.1.3`), incomplete recovery fail-closed paths (`vb-qi37.1.4`), and digest mismatch proof (`vb-qi37.1.5`).
- Still-open blockers shape final scope: crash restart integration evidence (`vb-qi37.1.6`) and silent discard elimination (`vb-qi37.12`).
- Downstream acceptance roots include `vb-core-yaml-e2e-chain` and `vb-engine-yaml`, so this bead must not regress no-YAML recovery and typed durable replay behavior.

## Relevant crate clusters

### Storage recovery core: `crates/vb_storage/src/recovery`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/types.rs`
  - Lines 16-96 define typed `RecoveryError` variants for journal errors, workflow source digest mismatch, compiled IR digest mismatch, replay divergence, missing recovery data, corrupt snapshots, terminal mismatch, and frame dimension overflow.
  - Lines 115-163 define `RecoveryRuntimeSummary` and `RecoveryHydration::{Summary, FrameSeed}`.
  - Lines 176-218 define recovered step and slot entries with durable `SlotValue` plus `Taint`.
  - Lines 221-302 define `UnsupportedRecoveryState` and `RecoveryFrameSeed`; unsupported flags are the runtime fail-closed guard for missing slot values, taint, action payloads, and pending action resumability.
  - Lines 304-317 define `RunSnapshot` with run id, event sequence, workflow digest, compact slot bytes, and compact taint bytes.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/recover.rs`
  - `check_workflow_source_digest` lines 21-39 rejects absent or mismatched `RunAccepted` digest data.
  - `check_compiled_ir_digest` lines 42-50 returns `CompiledIrDigestMismatch` for artifact drift.
  - `verify_digests` lines 56-74 combines workflow and IR checks; action ABI and policy digest checks are explicitly deferred in comments.
  - `recover_runtime_summary` lines 77-86 and `recover_runtime_frame_seed` lines 89-98 load durable events from `FjallJournal` and reject empty recovery data.
  - `recover_all_incomplete_runs` lines 115-134 scans run headers and fails closed if a header has no events.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/replay/summary.rs`
  - `apply_summary_event` lines 22-66 records steps, actions, suspensions, slot writes, and terminal status into the recovery summary.
  - `summarize_recovery_events` lines 88-119 rejects empty input and mixed-run event streams.
  - `recover_runtime_frame_seed_from_events` lines 166-170 and `recover_runtime_frame_seed_from_events_with_workflow` lines 172-180 are the frame seed entrypoints.
  - `reject_workflow_digest_mismatch` lines 182-199 raises `CompiledIrDigestMismatch` before workflow-backed replay.
  - `build_recovery_frame_seed` lines 224-248 constructs dimensions, pc, steps, slots, pending actions, and unsupported-state flags.

### Snapshot and event hydration

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/hydrate.rs`
  - `hydrate_run_frame` lines 32-126 hydrates `RunFrame` from `RunSnapshot` plus tail journal events; it validates run id, tail event run id, tail sequence ordering, non-empty data, decodes slot/taint bytes, derives dimensions, applies snapshot slots, applies tail events, and increments executed count.
  - `hydrate_run_frame_from_events` lines 135-229 hydrates from full event history using `recover_runtime_frame_seed_from_events`, then applies steps, slots, pc, executed count, and parallel in-flight count.
  - Local risk: line 208 currently uses `unwrap_or(u64::MAX)` on a fallible count conversion. It is not an edit target in this State 2 repair, but downstream implementation/review should decide whether that violates repository zero-unwrap governance.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/hydrate_support.rs`
  - Supporting functions named by `hydrate.rs`: `decode_snapshot_slots`, `derive_dimensions_from_snapshot_and_tail`, `apply_tail_events`, and `compute_parallel_in_flight`.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/events.rs`
  - `JournalEvent` lines 13-180 carries durable records needed by this bead: `RunAccepted`, `RunAdmission`, `StepStarted`, `StepSucceeded`, `ActionScheduled`, `ActionCompletedEvent`, `ActionFailedEvent`, `SlotWrittenEvent`, `WaitScheduledEvent`, `AskScheduledEvent`, `AskAnsweredEvent`, `RetryScheduledEvent`, `RunCancelled`, `RunFinished`, and `RunFailedEvent`.
  - `SlotWrittenEvent` lines 97-112 stores postcard `SlotValue` bytes but no explicit taint field in this enum variant; existing recovery tests infer or require taint behavior, so downstream work must prove exact taint semantics remain durable rather than defaulted.

### Runtime recovery boundary: `crates/vb_runtime`

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_runtime/src/recovery.rs`
  - `hydrate_run_admission_from_events` lines 12-27 maps durable `RunAdmission` events into runtime admission metadata.
  - `RuntimeRecoveryBoundary` lines 29-36 defines summary plus live-frame hydration behavior.
  - `DurableFrameRecoveryBoundary::hydrate_run_frame` lines 63-70 rejects unsupported state, builds an empty recovered frame, applies recovered steps, slots, and pc.
  - `reject_unsupported_live_frame_state` lines 73-83 returns `RuntimeError::InvalidRecoveryHydration` for unsupported slot values, taint, action payloads, or pending actions.
  - `recovery_boundary_from_hydration` lines 120-129 selects summary-only versus full-frame recovery.
  - `SummaryRecoveryBoundary::hydrate_run_frame` lines 145-152 explicitly fails with `UnsupportedFullRecoveryHydration`, preventing silent empty-frame success.

## Existing tests and evidence surfaces

- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/workspace_tests/tests/vb_qi37_1_1_red_recovery_contract_test.rs`
  - Lines 19-52 build complete event streams with `RunAccepted`, `StepStarted`, `SlotWrittenEvent`, and `StepSucceeded`.
  - Lines 76-97 exercise `recover_runtime_frame_seed_from_events` and runtime `hydrate_run_frame`.
  - Lines 154-178 cover recovered slot values and expected taints across i64, bool, action completion, ask answer, and null cases.
  - Lines 180-223 assert supported recovery when value bytes are valid and prevent missing taint from defaulting to clean.
  - Lines 232-252 prove no-output steps do not fabricate slot zero state.
  - Lines 254-294 cover corrupt and missing slot values as unsupported recovery state.
  - Lines 353-388 add proptest coverage for taint preservation, no-output slot dimensions, and hydrateable valid slot events.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/src/recovery/tests.rs`
  - Broad storage recovery test module; use for digest mismatch, snapshot/tail, corrupt journal, no recovery data, and terminal-state cases.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/vb_storage/tests/replay_resume.rs`
  - Integration-oriented replay/resume surface relevant to crash-before-ack and crash-after-ack evidence.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/velvet_ballistics/tests/cli_integration.rs`
  - CLI end-to-end surface for durable run, inspect/events, and recovery-adjacent operator proof.
- `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-1/crates/velvet_ballistics/tests/admission_evidence_integration/**/*.rs`
  - Admission and durable artifact evidence; useful because recovery must remain bound to accepted persisted artifacts.

## Public APIs to preserve/prove

- `vb_storage::recovery::{verify_digests, recover_runtime_summary, recover_runtime_frame_seed, recover_run_admission, recover_all_incomplete_runs}`.
- `vb_storage::recovery::replay::summary::{recover_runtime_frame_seed_from_events, recover_runtime_frame_seed_from_events_with_workflow, recover_run_admission_from_events, summarize_recovery_events}`.
- `vb_storage::recovery::{hydrate_run_frame, hydrate_run_frame_from_events}`.
- `vb_runtime::recovery::{RuntimeRecoveryBoundary, DurableFrameRecoveryBoundary, SummaryRecoveryBoundary, recovery_boundary_from_hydration, hydrate_run_admission_from_events}`.
- `vb_storage::{FjallJournal, JournalEvent, EventSeq}` event append/read paths.
- `vb_core::{RunFrame, RunId, StepIdx, SlotIdx, SlotValue, Taint, WorkflowDigest}` frame and typed value contracts.

## Open questions and risks for downstream states

- Acceptance asks for action tickets, waits, asks, retries, and collect pagination; `RecoveryFrameSeed` currently exposes `pending_actions` and step states, but runtime boundary rejects pending actions as unsupported. Downstream states must either prove the requested state is represented and resumable or fail closed with explicit acceptance disposition.
- `SlotWrittenEvent` has `value` and `extra` but no visible `slot_taint` field in `events.rs`; downstream work must verify whether taint is encoded in `extra`, inferred from workflow replay, or still incomplete.
- `verify_digests` documents deferred Action ABI and policy digest verification. If this bead is treated as full replay/recovery digest proof, the deferral must be resolved or explicitly scoped to another bead.
- Summary-only recovery is intentionally fail-closed for live-frame hydration. Tests must assert no code path converts summary-only recovery into an empty successful frame.
- Crash-before-ack and crash-after-ack should exercise durable write ordering against `FjallJournal` and runtime acknowledgement boundaries, not only pure event-vector helpers.

## Recommended verifier/test modes

- Unit: recovery event folding, snapshot+tail hydration, unsupported-state fail-closed behavior, digest mismatch variants.
- Integration: Fjall-backed crash-before-ack, crash-after-ack, corrupt journal, and snapshot+journal replay using persisted run headers/events.
- BDD/e2e: Given accepted artifact and durable journal, When restart/recover runs, Then full frame or typed fail-closed diagnostic is produced without YAML reparsing.
- Proptest: corrupted event sequence, slot bytes, digest drift, attempt ordering, and mixed-run event streams.
- Kani/Verus optional: event-sequence invariants for no silent empty-frame success and monotonic journal sequence assumptions if existing harness patterns are available.
