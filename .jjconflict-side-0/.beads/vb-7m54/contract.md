# Contract: Concurrency Loom Models for VB-CONC-001..005

## Bead: vb-7m54

## 1. Overview

This contract covers the implementation of loom concurrency models for 5 runtime concurrency seams. The `cargo xtask loom --model <name>` command is documented in the master build contract (line 4724) and required by proof_obligations.yaml (VB-CONC-001..005, Section 49), but the command does not currently exist in xtask/src/. This contract bridges that gap.

## 2. Requirements

### VB-CONC-001: Journal Writer Queue
- **Statement**: Journal writer queue: ordered write before flush
- **File**: `crates/vb_runtime/src/journal/writer.rs`
- **Command**: `cargo xtask loom --model journal_writer_queue`
- **Model**: Verify that journal events are written in order before flush, and that concurrent writes do not reorder.

### VB-CONC-002: Action Completion vs Cancel Race
- **Statement**: Action completion vs cancel race: proper ordering
- **File**: `crates/vb_runtime/src/engine/action.rs`
- **Command**: `cargo xtask loom --model action_completion_cancel`
- **Model**: Verify that when an action completion and a cancellation race, the ordering is correct and no state corruption occurs.

### VB-CONC-003: Timer Fired vs Cancel
- **Statement**: Timer fired vs cancel: proper ordering
- **File**: `crates/vb_runtime/src/shard/timer_wheel.rs`
- **Command**: `cargo xtask loom --model timer_fired_cancel`
- **Model**: Verify that timer fired and cancel operations are properly ordered with no use-after-free.

### VB-CONC-004: Shutdown Drain
- **Statement**: Shutdown drain: graceful shutdown ordering
- **File**: `crates/vb_runtime/src/shard/lifecycle.rs`
- **Command**: `cargo xtask loom --model shutdown_drain`
- **Model**: Verify that shutdown drains all pending work in correct order without orphaned state.

### VB-CONC-005: Bounded Queue Wrapper
- **Statement**: Bounded queue wrapper: enqueue/dequeue invariants
- **File**: `crates/vb_runtime/src/frame_pool.rs`
- **Command**: `cargo xtask loom --model bounded_queue`
- **Model**: Verify that the frame pool bounded queue maintains enqueue/dequeue invariants under concurrent access.

## 3. Non-Goals

- Loom models for non-concurrent primitives (rtrb SPSC ring, single-threaded frame pool)
- Formal verification of Fjall disk persistence
- Network distribution correctness

## 4. Constraints

- All loom models must use the `loom` crate (crablang/loom)
- Models must be placed in `models/loom/` directory within vb_runtime
- The xtask loom subcommand must dispatch to the correct model
- All 5 models must execute under `cargo xtask loom --model <name>` without additional setup

## 5. Assumptions

- loom 0.4+ is available as a dev-dependency
- rustc supports the loom cfg flag
- The 5 concurrency seams identified are the only L3-required seams in the current codebase

## 6. Verification Criteria

Each model must:
1. Compile under `RUSTFLAGS="--cfg loom"` without errors
2. Run under `cargo xtask loom --model <name>` and complete without deadlocks
3. Check all ordering invariants defined in the model
4. Be deterministic (same seed produces same result)
