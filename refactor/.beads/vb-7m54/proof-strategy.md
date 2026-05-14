# Proof Strategy: vb-7m54 — Concurrency Loom Models

## Bead: vb-7m54
## Status: PLANNED

## Scope

- **vb_runtime**: `journal/writer.rs`, `engine/action.rs`, `shard/timer_wheel.rs`, `shard/lifecycle.rs`, `frame_pool.rs`
- **xtask**: `src/loom.rs` (new), `src/cli.rs` (modify), `src/proof.rs` (modify)

## Discovery Evidence

```bash
# loom crate availability
cargo search loom  # Returns loom = "0.7.2" — AVAILABLE

# No loom usage in workspace (CONFIRMS GAP)
rg "loom" --include=Cargo.toml  # 0 results

# All concurrency files use #![forbid(unsafe_code)] — GOOD
rg "#!\[\]\"forbid(unsafe_code)\]" vb_runtime/src/journal/writer.rs vb_runtime/src/engine/action.rs vb_runtime/src/shard/timer_wheel.rs vb_runtime/src/shard/lifecycle.rs vb_runtime/src/frame_pool.rs
```

## Risk Classification

| Obligation | Risk | Property Type | Verifier | Rationale |
|---|---|---|---|---|
| VB-CONC-001 | HIGH | Ordering (concurrent writes) | loom | Shared mutable state, lock-free queue |
| VB-CONC-002 | HIGH | Ordering (completion vs cancel) | loom | Shared mutable state, race condition |
| VB-CONC-003 | HIGH | Ordering + UB (timer vs cancel) | loom | Shared mutable state, use-after-free risk |
| VB-CONC-004 | HIGH | Ordering (shutdown drain) | loom | Shared mutable state, orphaned work risk |
| VB-CONC-005 | HIGH | Invariants (bounded queue) | loom | Shared mutable state, overflow/underflow |
| VB-CONC-XTASK | HIGH | Implementation | command | Prerequisite for running models |

## Verification Strategy

### Layer 1: Loom Model Checking (Primary)

Each of the 5 models is a Rust test module annotated with `#[cfg(loom)]` that:
1. Models the concurrency seam as a set of threads/interleavings
2. Checks ordering invariants using loom's permutation exploration
3. Reports any violations

#### VB-CONC-001: journal_writer_queue

**Model**: `crates/vb_runtime/src/models/loom/journal_writer_queue.rs`
**Command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue`
**Model approach**:
- Model journal writer queue as a series of ordered writes
- Spawn N writers competing for the queue
- Verify: all writes appear in the queue in program order, flush preserves order
- Invariant: `forall i, j. write_i happens-before write_j => position(write_i) < position(write_j)`

#### VB-CONC-002: action_completion_cancel

**Model**: `crates/vb_runtime/src/models/loom/action_completion_cancel.rs`
**Command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime action_completion_cancel`
**Model approach**:
- Model action completion and cancellation as racing operations
- Verify: exactly one of completion/cancel succeeds, state is consistent
- Invariant: `running_state == Completed || running_state == Cancelled` (never both, never neither after race)

#### VB-CONC-003: timer_fired_cancel

**Model**: `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs`
**Command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel`
**Model approach**:
- Model timer fired and cancel operations on the same timer
- Verify: no use-after-free, exactly one handler fires
- Invariant: `timer.fired == true => timer.handler ptr is valid`

#### VB-CONC-004: shutdown_drain

**Model**: `crates/vb_runtime/src/models/loom/shutdown_drain.rs`
**Command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain`
**Model approach**:
- Model shutdown initiating drain while work is in flight
- Verify: all pending work is drained before shutdown completes
- Invariant: `pending_work.is_empty()` after shutdown returns

#### VB-CONC-005: bounded_queue

**Model**: `crates/vb_runtime/src/models/loom/bounded_queue.rs`
**Command**: `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime bounded_queue`
**Model approach**:
- Model enqueue/dequeue operations on bounded queue
- Verify: no overflow (enqueue when full returns error), no underflow (dequeue when empty returns error)
- Invariant: `queue.len() <= queue.capacity()`, `queue.len() >= 0`

### Layer 2: xtask Loom Command

**Artifact**: `xtask/src/loom.rs` + CLI modifications
**Command**: `cargo xtask loom --model <name>`
**Approach**:
- Create `xtask/src/loom.rs` module with `run_loom_model(model_name: &str)` function
- Add `Loom` variant to `Commands` enum in `cli.rs`
- Dispatch from `main()` to `loom::run_loom_model`
- Each model name maps to a specific test function via match statement
- Returns exit code 0 on success, 1 on failure

## Artifact Plan

| Artifact | Type | Command |
|---|---|---|
| `crates/vb_runtime/src/models/loom/mod.rs` | New | Module entry |
| `crates/vb_runtime/src/models/loom/journal_writer_queue.rs` | New | loom model |
| `crates/vb_runtime/src/models/loom/action_completion_cancel.rs` | New | loom model |
| `crates/vb_runtime/src/models/loom/timer_fired_cancel.rs` | New | loom model |
| `crates/vb_runtime/src/models/loom/shutdown_drain.rs` | New | loom model |
| `crates/vb_runtime/src/models/loom/bounded_queue.rs` | New | loom model |
| `crates/vb_runtime/src/models/loom.rs` | New | Module re-export |
| `xtask/src/loom.rs` | New | xtask command |
| `xtask/src/cli.rs` | Modify | Add Loom variant |
| `xtask/src/main.rs` | Modify | Dispatch loom |
| `vb_runtime/Cargo.toml` | Modify | Add loom dependency |
| `crates/vb_runtime/src/shard/lifecycle.rs` | Modify | Add loom cfg guards |
| `crates/vb_runtime/src/shard/timer_wheel.rs` | Modify | Add loom cfg guards |
| `crates/vb_runtime/src/engine/action.rs` | Modify | Add loom cfg guards |
| `crates/vb_runtime/src/journal/writer.rs` | Modify | Add loom cfg guards |
| `crates/vb_runtime/src/frame_pool.rs` | Modify | Add loom cfg guards |

## Owner States

| Obligation | Owner | State |
|---|---|---|
| VB-CONC-001 | Lewis | open |
| VB-CONC-002 | Lewis | open |
| VB-CONC-003 | Lewis | open |
| VB-CONC-004 | Lewis | open |
| VB-CONC-005 | Lewis | open |
| VB-CONC-XTASK | Lewis | open |
