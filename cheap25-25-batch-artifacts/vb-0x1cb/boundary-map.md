# Boundary Map — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: boundary_map
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This document pins the boundaries of the contract: where the secondary-error surface enters, exits, is contained, and where the **runtime diagnostic path** runs. Every arrow below is mapped to a concrete source location.

## 1. Boundary diagram

```
                                external caller
                                       │
                                       │  Shard::finish_run(run, state) -> RuntimeResult<()>
                                       │  Shard::fail_run_state(run, state) -> RuntimeResult<()>
                                       ▼
       ┌──────────────────────────────────────────────────────────────────┐
       │  PURE CORE: shard/transitions.rs (lines 87–214)                  │
       │  - apply, keep_run, finish_run, await_action, await_timer,        │
       │    fail_run_state                                                 │
       │  - new helper: observe_run_state_rollback(run, state, site, …)    │
       └─────────────┬────────────────────────────────────────────────────┘
                     │
                     │  call
                     ▼
       ┌──────────────────────────────────────────────────────────────────┐
       │  STORAGE / DURABILITY: append_journal_event(                     │
       │      RuntimeJournalEvent::RunFinished | RunFailed)               │
       │  - vb_storage Fjall-backed mutable log                           │
       │  - SharedRuntimeJournal (Arc<dyn RuntimeJournal>)                │
       └─────────────┬────────────────────────────────────────────────────┘
                     │  Err(primary)
                     │  rollback path enters
                     ▼
       ┌──────────────────────────────────────────────────────────────────┐
       │  ROLLBACK CHOKEPOINT: observe_run_state_rollback                │
       │  - run_state_insert(run, state)                                  │
       │    └─ reserve_run_state_slot(run)                                │
       │      - bounded BTreeMap<RunId, RunState>                         │
       │      - errors: ActiveRunCapacityExceeded, …                      │
       └─────────────┬────────────────────────────────────────────────────┘
                     │
                     │  Err(secondary)
                     ▼
       ┌──────────────────────────────────────────────────────────────────┐
       │  TRACE OBSERVABILITY CHANNEL:                                    │
       │    self.trace_ring.push(TraceEvent::RunRollbackFailed {          │
       │      run, site, primary: Arc<…>, secondary: Arc<…>               │
       │    })                                                             │
       │  - bounded ring (capacity set at config)                         │
       │  - push is allocation-fallible at saturation (silent drop)       │
       └──────────────────────────────────────────────────────────────────┘


       RESULT CHANNEL (typed): return Err(primary)  ─────────► caller
       OBSERVABILITY CHANNEL (typed): TraceEvent::RunRollbackFailed  ─────────► operator / proof harness
```

## 2. Boundary classification

### 2.1 Pure core (functional)

| Symbol | File | Notes |
|--------|------|-------|
| `Shard::apply` | `crates/vb_runtime/src/shard/transitions.rs:50-76` | Already clean. |
| `Shard::keep_run` | `transitions.rs:79-83` | Already clean. |
| `Shard::finish_run` | `transitions.rs:87-112` | **Repair target** — line 100 replaced. |
| `Shard::fail_run_state` | `transitions.rs:200-214` | **Repair target** — line 202 replaced. |
| `Shard::observe_run_state_rollback` (NEW) | `transitions.rs` (added in this bead) | Pure chokepoint that binds `Result` and dispatches to trace ring. |
| `ObservedRollbackOutcome` | `crates/vb_runtime/src/shard/transitions.rs` or `shard/types.rs` (where it lives in the impl_parts) | Pure data, `Debug + PartialEq + Eq`. |

### 2.2 Imperative shell

The two helpers that touch mutable in-memory maps but are still synchronous, fully under `&mut Shard` ownership:

| Symbol | File | Notes |
|--------|------|-------|
| `Shard::run_state_insert` | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:323-330` | Returns `RuntimeResult<Option<RunState>>`. Reads `Option` from `BTreeMap::insert`. Reads/writes `runs`. |
| `Shard::reserve_run_state_slot` | inside the above | Bounded capacity check. |

### 2.3 Storage boundary (durable I/O)

| Symbol | File | Notes |
|--------|------|-------|
| `Shard::append_journal_event` | `crates/vb_runtime/src/shard/impl_parts/chunk_001.rs:206-211` (production) | Reads/writes durable journal. Fjall-backed mutable log. |
| `vb_storage::JournalError` | `crates/vb_storage/src/...` | Root cause for `StorageJournalAppend { source }`. |

### 2.4 Trace observability boundary

| Symbol | File | Notes |
|--------|------|-------|
| `TraceRing::push` | `crates/vb_runtime/src/trace/...` | Synchronous push under `&mut self`. Drops on saturation (existing behavior; this bead does not change it). |
| `TraceEvent::RunRollbackFailed` (new variant) | `crates/vb_runtime/src/trace/event.rs` | `#[non_exhaustive]`, lives inside the existing enum derive. |

### 2.5 Async shell

This bead touches NO async paths. The runtime core is single-threaded, single-shard, fully synchronous (`Shard::tick` runs on a sync `VecDeque` command queue). Concurrency is on the **boundary plane only**, controlled by `Arc<SharedRuntimeJournal>`.

### 2.6 Time / FFI / Unsafe / Parser

This bead introduces **none** of these. No `Instant`, no `extern "C"`, no `unsafe`, no string parser.

## 3. Boundary direction

| From → To | Direction | Notes |
|------------|-----------|-------|
| `apply_drive_result` → `finish_run` | external → pure core | single owner, no race. |
| `apply_drive_result` → `fail_run_state` | external → pure core | same. |
| `finish_run` → `append_journal_event` | pure core → storage | `&mut self` lock for the duration. |
| `finish_run` → `observe_run_state_rollback` → `run_state_insert` | pure core → imperative shell | single-threaded, owned. |
| `observe_run_state_rollback` → `trace_ring.push` | pure core → trace observability | bounded ring. |
| `finish_run` → `runtime_states` (in success) | pure core → aggregate field | single-threaded. |

## 4. Boundary invariants

| ID | Invariant |
|----|-----------|
| **B-1** | Every public-facing entry (`finish_run`, `fail_run_state`) accepts an owned `RunState` and an integer-like `RunId`. No `&` references cross the public boundary for the parameters in this bead. |
| **B-2** | All `&mut self` borrows in `transitions.rs` are scoped to individual statements; the rollback chokepoint `observe_run_state_rollback` does NOT re-borrow `self` outside its match arm — it returns an `ObservedRollbackOutcome` value that the caller can branch on. |
| **B-3** | The trace ring is reached only through the `push` method; no direct `tracing::error!` or `eprintln!` writes are introduced. (Forbids H-OBS-2 in `hazard-analysis.md`.) |
| **B-4** | `Arc::new(secondary)` allocation happens exactly once per dual-failure event. Subsequent reads (`Arc::clone`) are pointer-sized. |
| **B-5** | No `unsafe`, no FFI, no `Box<[u8]>` parsing. |
| **B-6** | The repair does NOT cross crate boundaries (`vb_core`, `vb_storage`) for type changes. Only the `vb_runtime` crate is modified. |

## 5. Existing boundaries preserved

| Boundary | Status | Comment |
|----------|--------|---------|
| `scripts/check-ignored-fallible-results.sh` | exercises `transitions.rs` for DISCARD-006 | After the repair, the script MUST exit 0 for `transitions.rs` without reading the allow row. |
| `scripts/check-test-integrity.sh` | wraps `test-integrity` | unaffected by this bead (per `codebase-map.md` §Moon v2). |
| `.moon/tasks/all.yml` | `ignored-fallible-results` task depends on allow file | Removing the allow row invalidates the cache for `transitions.rs` but no yaml change required. |
| `lifecycle_tests::chunk_004.rs::LegacyStepFailsJournal` | mirrors `finish_run`/`fail_run_state` rollback contract on the typed-error side | preserves frame invariants — used as the test pattern reference for `chunk_005.rs` and `chunk_008.rs`. |

## 6. Cross-references

- `domain-model.md` §Forbidden S4 — no `eprintln!` in error paths.
- `type-contracts.md` §2.2 — `observe_run_state_rollback` signature with `&mut self` borrow scope.
- `workflow-model.md` §1, §2 — rollback branch states S2 → S3/S4.
- `hazard-analysis.md` H-OBS-1, H-OBS-2, H-MEM-1.
- `contract.md` clauses C-2, C-3, C-7.
