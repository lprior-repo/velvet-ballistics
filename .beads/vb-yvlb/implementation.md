# Implementation Report — GAP-12 vb-yvlb

bead_id: vb-yvlb
bead_title: GAP-12 feat: Implement ShardOwnership.tla in Rust
phase: 6-implemented
updated_at: 2026-05-11T00:00:00Z

## What Was Implemented

### 1. New Types (runtime.rs)

- `ShardIndex(pub usize)` — 0-based shard index, implements `Eq`, `Hash`, `Copy`, `Clone`, `Debug`, `PartialEq`
- `Transfer { run: RunId, target_shard: ShardIndex }` — pending transfer record, implements `Eq`, `Hash`, `Clone`, `Debug`, `PartialEq`

### 2. Runtime State Extensions (runtime.rs)

Added to `Runtime` struct:
- `run_owner: HashMap<RunId, ShardIndex>` — global ownership map
- `shard_runs: HashMap<ShardIndex, HashSet<RunId>>` — reverse index
- `pending_transfers: HashSet<Transfer>` — in-flight migrations

Initialized in `Runtime::new_with_journal`:
- `shard_runs` pre-populated with empty sets for each shard index 0..count

### 3. New RuntimeError Variants (lib.rs)

- `RunAlreadyOwned { run, owner }` — run already has an owner
- `TransferAlreadyPending { run, target_shard }` — transfer already in flight
- `NoPendingTransfer { run }` — no pending transfer for run
- `DuplicateTransfer { run, shard }` — run already transferring to shard
- `TransferSameShard { run, shard }` — source and target are identical

### 4. Ownership Methods (runtime.rs)

- `assign_shard(run, shard)` — claims ownership of unowned run; returns `RunAlreadyOwned` if already owned
- `initiate_transfer(run, target_shard)` — adds transfer to pending set; enforces preconditions
- `complete_transfer(run, target_shard)` — executes transfer: removes from old shard, adds to new, updates owner, clears pending
- `release_run_ownership(run)` — called on cancel: removes from all ownership maps and pending transfers
- `get_run_owner(run)` — returns `Option<ShardIndex>`
- `get_shard_runs(shard)` — returns `Option<&HashSet<RunId>>`
- `get_pending_transfers()` — returns `&HashSet<Transfer>`
- `is_run_owned_by(run, shard)` — returns `bool`

### 5. Updated shard_index (runtime.rs)

Now checks `run_owner` first for actual ownership; falls back to hash-based routing for unowned runs (consistent placement).

### 6. Updated Submit Path (runtime.rs)

- `submit_direct`, `submit_compiled`, `submit_compiled_with_inputs` changed from `&self` to `&mut self`
- Each calls `assign_shard` before enqueueing the Submit command — enforces `SingleOwner` invariant
- `cancel_run` changed from `&self` to `&mut self`; calls `release_run_ownership` after enqueueing Cancel

### 7. Pre-Existing Build Fix (lifecycle.rs)

Fixed `admit_artifact_run` call site:
- Added `AggregateResourceBudget::from_workflow(&workflow)` call to compute budget
- Propagates `RuntimeError::UnsupportedOperation` on budget computation failure
- Updated `build_admission` to accept and forward the `reservation` parameter

### Pre-Existing Test Errors (journal.rs) — DEFERRED_GLOBAL

`crates/vb_runtime/src/journal.rs:882,888,981` — `JournalEvent` test assertions use stale field names (`seq`, `attempt`, variant names like `ActionCompletedEvent`). These are pre-existing test code bugs unrelated to GAP-12 changes. Not fixed in this bead.

## Constraints Compliance

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in production code
- All HashMaps use `std::collections::HashMap` and `std::collections::HashSet`
- `ShardIndex` and `Transfer` implement all required traits
- All fallible operations return typed `RuntimeResult`/`Result`

## Verification

- `cargo build -p vb_runtime --lib` → 0 errors
- `cargo clippy -p vb_runtime --lib` → 0 errors
- `cargo test -p vb_runtime` → blocked by pre-existing journal.rs test compilation errors (DEFERRED_GLOBAL)
