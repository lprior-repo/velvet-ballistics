# Error Taxonomy — vb-0x1cb

- bead_id: vb-0x1cb
- phase: 3 (contract)
- attempt: 1-of-1
- captured_at: 2026-07-01T15:55:00Z
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-0x1cb
- source_checkout: /home/lewis/src/velvet-ballistics
- controller: femdation
- scope_kind: error_taxonomy
- lane_profile: rust_local_concurrency_empty
- status: contract drafted

This document classifies every error that `Shard::finish_run`, `Shard::fail_run_state`, and the new `Shard::observe_run_state_rollback` chokepoint can produce, plus the secondary-error surface channel (`TraceEvent`). It is the single point of truth for downstream proof/test lanes that need to enumerate the failure modes.

## 1. Primary-error ladder (unchanged)

The call sites `Shard::finish_run` / `Shard::fail_run_state` produce the same `RuntimeError` set they did before this bead. The primary-error ladder is:

| Failure origin | Variant | Diagnostic code | Symbolic code | Notes |
|----------------|---------|-----------------|---------------|-------|
| Journal rejects `RunFinished { run, result }` | `RuntimeError::StorageJournalAppend { source: Arc<vb_storage::JournalError> }` | `0x2008 STORAGE_JOURNAL_APPEND_FAILED_CODE` | `SymbolicCode::STORAGE_JOURNAL_APPEND` | Returned to caller as `Err(_)` from `finish_run`. |
| Journal rejects `RunFailed { run }` | same variant | same | same | Returned to caller as `Err(_)` from `fail_run_state`. |
| `terminal_runs_insert(run)` rejects | `RuntimeError::ActiveRunCapacityExceeded { capacity }` (via `reserve_terminal_run_slot`) | `0x2003 ACTIVE_RUN_CAPACITY_EXCEEDED_CODE` | (registry) | Only reachable on the **happy** path after the journal append — pre-existing. |
| `count_to_two` overflow | (no path today; counter overflow guard is at `counters::inc_completed`/`inc_failed` calls — out of scope) | n/a | n/a | n/a |

These are the ONLY variants the `Result::Err` of `finish_run` / `fail_run_state` is permitted to return. The repair does not add or remove any of them.

## 2. Secondary-error ladder (new)

When the primary-error branch fires (`append_journal_event` returned `Err(primary)`) and the rollback `run_state_insert` also returns `Err(secondary)`, the secondary error lands on the **observability channel**, not on the typed-error channel. The taxonomy of `secondary` is therefore the same `RuntimeError` variant space, narrowed to what `run_state_insert` can produce:

| Failure origin | Variant | Diagnostic code | Symbolic code | Channel |
|----------------|---------|-----------------|---------------|---------|
| `reserve_run_state_slot(run)` rejects (capacity exhausted) | `RuntimeError::ActiveRunCapacityExceeded { capacity }` | `0x2003` | (registry) | `RuntimeError::StorageJournalAppend { source }`-like — primary already typed. |
| `reserve_run_state_slot` rejects with slot exhaustion upper-bound | propagated via `From<vb_core::errors::CoreError>` as `RuntimeError::Core { source: Box<CoreError::…> }` | depends on inner `CoreError::diagnostic_code` | depends | see `error/conversions.rs`. |
| Frame-pool invariant violated when reconstructing state in `run_state_insert` | `RuntimeError::FramePoolUnavailable` | `0x200A` | (registry) | surfaced by helper internals. |

The contract does NOT re-classify these — they are wrapped by `Arc<RuntimeError>` and pushed onto `TraceEvent::RunRollbackFailed`. `diagnostic_code()` and `symbolic_code()` continue to identify them.

## 3. New `TraceEvent` variant

### 3.1 `TraceEvent::RunRollbackFailed { run, site, primary, secondary }`

```rust
RunRollbackFailed {
    run: RunId,
    site: RollbackSite,
    primary: std::sync::Arc<RuntimeError>,
    secondary: std::sync::Arc<RuntimeError>,
}
```

| Field | Cardinality | Encoding |
|-------|-------------|----------|
| `run` | exactly one per shard call | `RunId: Copy` |
| `site` | exactly one | `RollbackSite: Copy` (FinishRun or FailRunState) |
| `primary` | exactly one | `Arc<RuntimeError>` — wraps the `Err(primary)` from `append_journal_event` |
| `secondary` | exactly one | `Arc<RuntimeError>` — wraps the `Err(secondary)` from `run_state_insert` |

### 3.2 Diagnostic accessor (extension point)

The `TraceEvent::run_id()` method (`crates/vb_runtime/src/trace/event.rs:95-110`) MUST be extended:

```rust
pub const fn run_id(&self) -> RunId {
    match self {
        // ... existing arms unchanged ...
        Self::RunRollbackFailed { run, .. } => *run,
    }
}
```

### 3.3 `is_terminal_for_run`

The new variant IS NOT a terminal-event for the run lifecycle (it records a rollback failure, not a clean terminal state). The `is_terminal_for_run` method (`event.rs:114-129`) MUST NOT add `RunRollbackFailed` to its `match` arms:

```rust
Self::RunRollbackFailed { run, .. } if *run == target => false,  // explicit
```

(or simply leave the wildcard `Self::RunRollbackFailed { .. } => false` arm). This is an explicit non-claim: a dual-failure event is NOT terminal evidence.

## 4. Forbidden variants (NOT introduced)

The contract explicitly forbids these additions:

| Forbidden addition | Why |
|---------------------|-----|
| `RuntimeError::RollbackFailed { primary: Box<…>, secondary: Box<…> }` | Would force `diagnostics.rs:47-105` and `runtime_code` match arms to be extended for a code path that is observability-only. Forbids layering at the typed-error surface. |
| `RuntimeError::Core { source: CoreError::InternalInvariantViolation { reason: "finish_run_rollback_failed" } }` | Would change the primary error visible to the caller (from `StorageJournalAppend` to invariant), violating I1 (primary error wins). Forbids the secondary surface as typed-error. |
| `TraceEvent::RollbackRecovered { run, site }` | The recovered case carries no diagnostic; emitting it would needlessly inflate the trace ring and force log-noise analysis. Forbidden. |

## 5. Compact error-tree (one-page reference)

```
Result::Err from Shard::finish_run           │
  ├─ RuntimeError::StorageJournalAppend      │  <-- primary (must reach caller)
  └─ (no other variant; surface invariant)   │

Result::Err from Shard::fail_run_state       │
  └─ RuntimeError::StorageJournalAppend      │  <-- primary

TraceEvent variants emitted from any rollback:
  ├─ RunRollbackFailed { run, FinishRun | FailRunState,
  │                     primary: Arc<RuntimeError>,
  │                     secondary: Arc<RuntimeError> }   <-- dual failure
  ├─ RunFinished { run }                     │  <-- happy path of finish_run (pre-existing)
  └─ RunFailed   { run }                     │  <-- happy path of fail_run_state (pre-existing)
```

## 6. `From<…>` ladder (no change)

The `From<…>` ladder at `crates/vb_runtime/src/error/conversions.rs` is unchanged by this bead. No new `From` impl is needed because the secondary `RuntimeError` is already an `RuntimeError` and needs only an `Arc` wrap.

`Arc<RuntimeError>` is `Clone + PartialEq + Eq` for the `TraceEvent` derive to compile; both impls already exist via `std::sync::Arc<T>: PartialEq` and `: Eq` when `T: PartialEq + Eq`.

## 7. Cross-references

- `type-contracts.md` §1.2 — variant definition.
- `domain-model.md` §Invariants I1–I6.
- `workflow-model.md` — `S0–S4`/`S0–S4'` states.
- `contract.md` clauses C-1, C-2, C-3.
- `hazard-analysis.md` H-OBS-1.
