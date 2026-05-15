# Codebase Map — vb-core-strict-ack-ordering

## Bead Goal
Prove strict persistence-before-acknowledgement ordering for all mutation types: submit, action, wait, ask, retry, cancel, terminal. Fail-closed on persistence injection; restart evidence matches acknowledged state.

---

## 1. Core Crates and Files

### vb_runtime (orchestration, lifecycle, journal adapters)
| File | Role |
|------|------|
| `crates/vb_runtime/src/durability_matrix.rs` | **PRIMARY ARTIFACT**: `DURABILITY_MATRIX` maps each YAML primitive to `AckPoint::AfterJournalAppend`. `verify_ack_after_persist()` ensures no row claims `BeforeJournalAppend`. `AckPoint` enum with `AfterJournalAppend` / `BeforeJournalAppend`. |
| `crates/vb_runtime/src/error/mod.rs` | `RuntimeError` enum: `StorageJournalAppend`, `AdmissionHeaderPersistenceFailed`, `UnsupportedAsyncStrictAck`, `EncodeFailed` |
| `crates/vb_runtime/src/error/display.rs` | Display impl: "queued strict journal ack is unsupported without persisted-before-ack proof" |
| `crates/vb_runtime/src/admission.rs` | `RunAdmission`, `ArtifactEnvelopeError`, `REQUIRED_GATE_COUNT = 15`. Artifact envelope validation at admission. |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | **PRIMARY**: `handle_submit` → `handle_submit_with_inputs_contracts_and_header_mode` → journal appends `RunSubmitted` + `RunAdmission` → `RunState::insert` → `drive_run`. Error paths call `discard_journal_sequence`. `handle_action_completion`, `handle_action_failure` each `append_journal_event` before returning. |
| `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs` | `handle_ask_completion`, `handle_timer_fire`, `handle_cancel` — each appends before returning. |
| `crates/vb_runtime/src/shard/transitions.rs` | `finish_run`, `await_action`, `await_timer`, `fail_run_state` — all append journal events synchronously before returning. |
| `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs` | `enqueue` probes journal health before accepting submit variants. `append_journal_event` sequences events. `flush_evidence` drains `EvidenceCollector`. |
| `crates/vb_runtime/src/journal.rs` | `RuntimeJournalEvent` enum (16 variants). `RuntimeJournal` trait: `append`, `append_sequenced`, `probe`, `drain_for_shutdown`. |
| `crates/vb_runtime/src/journal/chunk_001.rs` | `NoopRuntimeJournal`, `VolatileRuntimeJournal`, `StorageRuntimeJournal`, `QueuedStorageRuntimeJournal`. `RuntimeJournalConfig` selects profile. |
| `crates/vb_runtime/src/journal/chunk_002.rs` | `StorageRuntimeJournal::append_storage_event` calls `append_strict` (strict barrier) or `append_journaled` (no barrier) based on `DurabilityProfile`. `run_storage_event` / `action_storage_event` / `boundary_storage_event` map `RuntimeJournalEvent` → `JournalEvent`. |
| `crates/vb_runtime/src/shard/impl_tests/` | Integration tests for persistence-before-ack per handler. |
| `crates/vb_runtime/src/runtime.rs` | `Runtime::submit_direct` calls `persist_run_header_before_ack` THEN `shard.enqueue(SubmitPrePersisted)`. |

### vb_storage (Fjall-backed persistence)
| File | Role |
|------|------|
| `crates/vb_storage/src/journal/mod.rs` | `FjallJournal` public re-export. |
| `crates/vb_storage/src/journal/core.rs` | `FjallJournal` struct: 8 keyspaces (`workflow_source`, `compiled_ir`, `run_header`, `events`, `run_snapshot`, `blob`, `index_status`, `index_workflow`, `index_action`). `Database` field is `fjall::Database`. |
| `crates/vb_storage/src/journal/append.rs` | **PRIMARY**: `append_journaled` (no barrier), `append_strict` (calls `persist_strict` after append), `append_strict_batch`, `persist_strict` (calls `database.persist(fjall::PersistMode::SyncAll)`). |
| `crates/vb_storage/src/journal/injection.rs` | `inject_raw_event`, `inject_seq_gap` — expert recovery tools. |
| `crates/vb_storage/src/journal/admission.rs` | `verify_content_digest` — blake3 digest verification at admission. |
| `crates/vb_storage/src/types.rs` | `DurabilityProfile` enum: `Volatile` / `Journaled` / `Strict`. `EventSeq` monotonic per-run sequencer. `StorageKey` variants. |
| `crates/vb_storage/src/records.rs` | `RecordKind` enum (u16 IDs 1-50). Maps all journal event types. |
| `crates/vb_storage/src/events.rs` | `JournalEvent` enum — storage-facing events with `EventSeq`. |
| `crates/vb_storage/src/recovery/mod.rs` | Recovery module re-exports. |
| `crates/vb_storage/src/recovery/replay/core.rs` | `replay_events` — event replay with `ReplayDivergence` detection, `compute_max_attempt`. |
| `crates/vb_storage/src/recovery/types.rs` | `RecoveryError`, `RecoveredRunAdmission`, `RecoveryFrameSeed`, `RecoveryTerminalState`. |
| `crates/vb_storage/src/recovery/hydrate.rs` | `hydrate_run_frame`, `hydrate_run_frame_from_events`. |
| `crates/vb_storage/src/error/mod.rs` | `JournalError` enum. |

### vb_core (IR, engine)
| File | Role |
|------|------|
| `crates/vb_core/src/action.rs` | `ActionTicket`, `ActionFailure`, `ActionOutputReady` — action boundary types. |
| `crates/vb_core/src/replay/` | Replay logic for step execution. |
| `crates/vb_core/src/engine/run_loop.rs` | Engine run loop. |

---

## 2. Key APIs and Ordering Contracts

### Persistence Ordering Mechanism
1. **Strict path**: `append_strict` → `persist_strict` → Fjall `persist(SyncAll)` → return Ok
2. **Journaled path**: `append_journaled` → return Ok (no barrier)
3. **Runtime adapter** (`StorageRuntimeJournal`): `append_storage_event` selects strict vs journaled based on `DurabilityProfile::Strict`
4. **Runtime error propagation**: `StorageJournalAppend` / `AdmissionHeaderPersistenceFailed` / `UnsupportedAsyncStrictAck`

### Acknowledgement Ordering (from DURABILITY_MATRIX)
Every primitive row has `ack_point: AckPoint::AfterJournalAppend`. No row may use `BeforeJournalAppend`.

| Primitive | CompiledNodeKind | Journal Events | Storage Partition |
|-----------|-----------------|----------------|------------------|
| set | SetConst | StepStarted, SlotWritten | RuntimeJournal |
| do | Do | StepStarted, ActionScheduled, ActionCompleted, SlotWritten | ActionJournal |
| choose | Choose | StepStarted, SlotWritten | RuntimeJournal |
| for_each | ForEach | StepStarted, SlotWritten | RuntimeJournal |
| together | Together | StepStarted, SlotWritten | RuntimeJournal |
| collect | Collect | StepStarted, SlotWritten, SlotWritten | RuntimeJournal |
| reduce | Reduce | StepStarted, SlotWritten, SlotWritten | RuntimeJournal |
| repeat | Repeat | StepStarted, SlotWritten | RuntimeJournal |
| wait | WaitUntil | StepStarted, WaitScheduled, SlotWritten | TimerJournal |
| ask | Ask | StepStarted, AskScheduled, AskAnswered, SlotWritten, SlotWritten | TimerJournal |
| finish | Finish | StepStarted, RunFinished | RuntimeJournal |

### Fail-Closed Error Types
- `RuntimeError::StorageJournalAppend { source: Arc<JournalError> }`
- `RuntimeError::AdmissionHeaderPersistenceFailed { source: Arc<JournalError> }`
- `RuntimeError::UnsupportedAsyncStrictAck`
- `RuntimeError::EncodeFailed`
- `RuntimeError::QueueFull` (journal probe failure)

---

## 3. Verifier Modes Required

| Mode | Coverage Required |
|------|-----------------|
| **Kani** | Bounded model checking for `append_strict` / `append_journaled` selection. Proof harnesses for `verify_ack_after_persist`. Codec roundtrip for all `RecordKind` IDs. |
| **Loom** | `journal_writer_queue` concurrency tests. `action_completion_cancel`, `shutdown_drain`, `timer_fired_cancel`. |
| **Miri** | `cfg(miri)` codec roundtrip tests in `vb_storage`. |
| **Proptest** | Journal event encoding/decoding. EventSeq ordering invariants. |
| **Integration tests** | `submit_direct_returns_durability_error_before_ack_when_header_cannot_persist`. `storage_failure_before_header_prevents_ack`. `restart_lookup_finds_persisted_header`. |

---

## 4. Known Dependencies and Blockers

- **Blocks**: `vb-core-atomic-admission`, `vb-core-yaml-e2e-chain`, `vb-engine-yaml`, `vb-qi37.12`
- **Required by**: `vb-2yb8` (per-primitive durability matrix)
- **Related**: `vb-qi37.4.3` (header persistence before ack), `vb-qi37.4.4` (storage admission)

---

## 5. Missing/Unverified Items

| Item | Status |
|------|--------|
| Formal Kani proof that `append_strict` barrier is reached before `Ok(())` return | MISSING |
| Loom test for strict journal flush ordering under concurrent submit | MISSING |
| Integration test for fail-closed on `persist_strict` injection for all mutation types | PARTIAL — submit covered, others not |
| Proof that restart recovery produces identical acknowledged state from journal | MISSING |
| `BeforeJournalAppend` variant removal from `AckPoint` | CANDIDATE — variant is unreachable per `verify_ack_after_persist` |

---

## 6. Open Questions

1. Is there a test verifying `QueuedStorageRuntimeJournal::flush_batch` respects strict ordering?
2. Does the queued strict path (`Strict` profile via queue) maintain the same barrier guarantee as `append_strict`?
3. Are there existing Kani harnesses for the `DurabilityProfile` selection logic in `append_storage_event`?
