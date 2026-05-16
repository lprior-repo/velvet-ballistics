# Baseline Report — GAP-12 vb-yvlb

bead_id: vb-yvlb
bead_title: GAP-12 feat: Implement ShardOwnership.tla in Rust
updated_at: 2026-05-11T00:00:00Z

## Pre-Edit Repo State

### Known failures (repo-wide, pre-bead)
- **DEFERRED_GLOBAL**: `crates/vb_runtime/src/shard/lifecycle.rs:142` — `admit_artifact_run` call missing 6th argument `reservation: AggregateResourceBudget`. Fixed by this bead.
- **DEFERRED_GLOBAL**: `crates/vb_runtime/src/journal.rs:366-388` — `action_storage_event` helper missing `attempt` field when constructing `JournalEvent`. Fixed by this bead.
- **DEFERRED_GLOBAL**: `crates/vb_runtime/src/journal.rs:879,885` — test assertions missing `attempt` field. Fixed by this bead.
- **DEFERRED_GLOBAL**: `crates/vb_core/src/policy.rs:50,60` — `JournalBeforeDispatch` naming and `DispatchSafety` constant naming lint errors. Pre-existing.
- **DEFERRED_GLOBAL**: `crates/vb_ui/src/replay/controller.rs:497,505,513` — missing `attempt` field in `JournalEvent` initializers. Pre-existing.
- **DEFERRED_GLOBAL**: `crates/vb_runtime/src/lib.rs` — `NonIdempotentActionReplayed` missing from `runtime_error_static_message`. Fixed by this bead.

### Scope of changes
This bead adds cross-shard run ownership tracking to vb_runtime and fixes the pre-existing `admit_artifact_run` call site.

### Touched crates
- `crates/vb_runtime/src/runtime.rs` — add ownership maps and transfer actions
- `crates/vb_runtime/src/lib.rs` — add RuntimeError variants
- `crates/vb_runtime/src/shard/lifecycle.rs` — fix pre-existing `admit_artifact_run` call
- `crates/vb_runtime/src/journal.rs` — fix pre-existing JournalEvent initializers (DEFERRED_GLOBAL fixes)

### Risk tags
- tla-contract
- multi-shard-consistency
- ownership-invariants
- pre-existing-build-error
