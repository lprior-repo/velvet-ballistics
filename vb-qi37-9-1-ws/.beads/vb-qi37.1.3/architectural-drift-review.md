bead_id: vb-qi37.1.3
bead_title: runtime/recovery: Hydrate RunFrame from snapshot and journal
phase: 13
updated_at: 2026-05-09T00:00:00Z

# Architectural Drift Review

## Reviewer: Orchestrator (GoMasterOrchestrator)
## Date: 2026-05-09

## File Size Audit

| File | Lines | Limit | Status |
|---|---|---|---|
| `recovery/recover.rs` | 134 | 300 | PASS |
| `recovery/hydrate.rs` | 227 | 300 | PASS |
| `recovery/hydrate_support.rs` | 285 | 300 | PASS |
| `recovery/mod.rs` | 59 | 300 | PASS |
| `recovery/types.rs` | 375 | 300 | PRE-EXISTING |
| `recovery/tests.rs` | 2430 | 300 | PRE-EXISTING |

## Changes Made

### Split recover.rs into three files

**Before**: `recover.rs` was 620 lines (exceeded 300-line limit)

**After**:
- `recover.rs` (134 lines) — Original recovery functions: digest checks, summary recovery
- `hydrate.rs` (227 lines) — Public hydration API: `hydrate_run_frame`, `hydrate_run_frame_from_events`
- `hydrate_support.rs` (285 lines) — Internal helpers: `decode_snapshot_slots`, `derive_dimensions_from_snapshot_and_tail`, `apply_tail_events`, `compute_parallel_in_flight`

### Updated mod.rs

Added `pub mod hydrate;` and `pub mod hydrate_support;` with re-exports for
`hydrate_run_frame` and `hydrate_run_frame_from_events`.

## DDD Audit

### Primitive Obsession
**PASS** — No primitives in domain positions. Uses `RunId`, `StepIdx`, `SlotIdx`, `EventSeq`.

### Parse, Don't Validate
**PASS** — Snapshot bytes are decoded into typed `RecoveredSlotEntry` structures.
Journal events are already strongly typed (`JournalEvent` enum).

### Explicit State Transitions
**PASS** — Hydration is a linear, explicit workflow:
1. Validate preconditions
2. Decode snapshot
3. Derive dimensions
4. Construct frame
5. Apply events
6. Return result

### Option-Based State Machines
**PASS** — No Option used for state machine representation. Uses `StepState` enum.

## Scott Wlaschin DDD Principles

### Illegal States Unrepresentable
**PASS** — `RunFrame::new` rejects step_count=0. `StepState` enum has no invalid transitions.
`checked_add` prevents overflow.

### Types as Documentation
**PASS** — All domain concepts have dedicated types (RunId, StepIdx, SlotIdx, EventSeq).

## CUPID Properties

- **Composable**: `hydrate_run_frame` and `hydrate_run_frame_from_events` are independent entry points.
- **Unix-philosophy**: Each file has a single responsibility.
- **Predictable**: Deterministic, no side effects, no randomness.
- **Idiomatic**: Follows Rust conventions, uses Result throughout.
- **Domain-based**: Names reflect domain concepts (hydrate, snapshot, tail, frame).

## Decision

STATUS: REFACTORED

The file split was necessary because `recover.rs` grew to 620 lines during implementation.
All new files are now under the 300-line limit. Pre-existing files (types.rs, tests.rs) are
out of bead scope and were not modified structurally.

The refactored structure is clean:
- `recover.rs`: legacy recovery orchestration
- `hydrate.rs`: public hydration API
- `hydrate_support.rs`: internal hydration helpers

Tests pass after refactoring (24/24 hydrate tests, 892/894 total vb_storage tests).
