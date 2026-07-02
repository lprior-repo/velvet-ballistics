# State 13 Unblock Plan for `vb-qi37.4.3`

STATUS: PLAN_ONLY

Workspace investigated: `/home/lewis/src/Velvet-ballistics-vb-qi37-4-3-go` only. Forbidden source checkout `/home/lewis/src/Velvet-ballistics` was not touched.

## Inputs read

- `.beads/vb-qi37.4.3/delivery-scope.jsonl`
- `.beads/vb-qi37.4.3/architectural-drift-review.md`
- `jj status && jj diff --stat`
- Scoped blocker line count over the six known oversized files.

## Current blockers

Line-count gate rejects every `.rs` file over 300 lines. The bead-local diff is small, but it touches oversized scoped files:

- `crates/vb_runtime/src/journal.rs`: 1191 lines; bead diff only adjusts one shutdown-drain test assertion.
- `crates/vb_runtime/src/runtime.rs`: 2240 lines; bead diff adds pre-ack header persistence and one unit test.
- `crates/vb_runtime/src/shard/impl_.rs`: 799 lines; bead diff adds `SubmitPrePersisted` dispatch.
- `crates/vb_runtime/src/shard/lifecycle.rs`: 2106 lines; bead diff adds pre-persisted submit path and tests.
- `crates/vb_runtime/src/shard/tests.rs`: 7005 lines; bead diff adds duplicate-submit coverage.
- `crates/velvet_ballistics/tests/admission_evidence_integration.rs`: 877 lines; bead diff adds two admission/header integration tests.

## Smallest safe unblock movement

Do not rewrite behavior. Do mechanical module extraction only: move existing functions/tests unchanged, keep existing public API re-exports, and add one type-safe enum to replace the new boolean header flag.

### 1. Runtime source split

Goal: `runtime.rs` becomes a façade under 300 lines and keeps `vb_runtime::runtime::Runtime::*` public methods unchanged.

Files:

- Keep `crates/vb_runtime/src/runtime.rs` with:
  - `ActiveRunSummary`
  - `Runtime` struct fields
  - `mod constructors; mod submission; mod commands; mod metrics; mod queries; mod shutdown;`
  - `#[cfg(test)] mod tests;`
- Add `crates/vb_runtime/src/runtime/constructors.rs`
  - `impl Runtime::{new,new_with_journal}`
- Add `crates/vb_runtime/src/runtime/submission.rs`
  - `impl Runtime::{submit_direct, submit_compiled, submit_compiled_with_inputs}`
  - private `persist_run_header_before_ack`
  - import `RuntimePolicy`, `CapabilitySet`, `CompiledWorkflow`, `ShardCommand`
- Add `crates/vb_runtime/src/runtime/commands.rs`
  - `cancel_run`, `resume_run`, `inspect_run`, `tick_all`, `complete_action`, `complete_action_with_output`, `fail_action`, `answer_ask`, `timer_fired`, private `shard_index`, private `shard_for`
- Add `crates/vb_runtime/src/runtime/queries.rs`
  - `snapshot_run`, `list_events`, `take_inspect_response`, `drain_trace`, `list_active_runs`
- Add `crates/vb_runtime/src/runtime/metrics.rs`
  - `collect_metrics`, `counters_snapshot`
- Add `crates/vb_runtime/src/runtime/shutdown.rs`
  - `shutdown_graceful`

Visibility/API notes:

- Keep `Runtime` fields private but visible to child modules by using normal Rust child privacy.
- Keep all existing `pub fn` signatures exactly unchanged.
- `shard_index` and `shard_for` can be `pub(super)` in `commands.rs` if needed by sibling modules; otherwise duplicate only routing helper is not recommended.

Tests:

- Replace inline `mod tests` with `#[cfg(test)] mod tests;`.
- Create `crates/vb_runtime/src/runtime/tests/mod.rs` as a thin module under 300 lines with shared helpers and submodules.
- Split existing runtime tests by outline groups:
  - `runtime/tests/support.rs`: `suspended_workflow`, `action_then_finish_workflow`, `runtime_config`, `finished_workflow`, `wait_then_finish_workflow`, `assert_suspended_run_is_found`, helper journals.
  - `runtime/tests/submission.rs`: `submit_direct_returns_durability_error_before_ack_when_header_cannot_persist`, submit/compiled/queue-full tests.
  - `runtime/tests/routing.rs`: cancel/complete/fail/answer/timer routing tests.
  - `runtime/tests/inspection.rs`: snapshot/list/take/drain trace tests.
  - `runtime/tests/metrics.rs`: counters/collect metrics tests.
  - `runtime/tests/shutdown.rs`: shutdown/tick-after-shutdown tests.
  - `runtime/tests/scheduler.rs`: scheduler-prefixed tests.

### 2. Journal source split

Goal: keep `crate::journal::{RuntimeJournalEvent, RuntimeJournal, SharedRuntimeJournal, NoopRuntimeJournal, VolatileRuntimeJournal, StorageRuntimeJournal, QueuedStorageRuntimeJournal, RuntimeJournalConfig}` usable exactly as before.

Files:

- Keep `crates/vb_runtime/src/journal.rs` as façade under 300 lines:
  - `mod event; mod port; mod volatile; mod storage; mod queued; mod seq;`
  - `pub use event::RuntimeJournalEvent;`
  - `pub use port::{RuntimeJournal, SharedRuntimeJournal};`
  - `pub use volatile::{NoopRuntimeJournal, VolatileRuntimeJournal, RuntimeJournalConfig};`
  - `pub use storage::StorageRuntimeJournal;`
  - `pub use queued::QueuedStorageRuntimeJournal;`
  - `#[cfg(test)] mod tests;`
- Add `journal/event.rs`: event enum and `run_id` impl.
- Add `journal/port.rs`: `RuntimeJournal` trait and `SharedRuntimeJournal` alias.
- Add `journal/volatile.rs`: `NoopRuntimeJournal`, `VolatileRuntimeJournal`, `RuntimeJournalConfig`.
- Add `journal/storage.rs`: `StorageRuntimeJournal` and mapping helpers `run_storage_event`, `action_storage_event`, `boundary_storage_event`, `storage_event`, `encoded_slot_taint_extra`.
- Add `journal/queued.rs`: `QueuedStorageRuntimeJournal`, `flush_batch`, `drain_all`, trait impl.
- Add `journal/seq.rs`: `current_seq`, `next_seq`.

Tests:

- Move inline journal tests into `journal/tests/mod.rs` plus submodules:
  - `support.rs`: `single_finish_workflow`, `temp_journal`, `journal_queue`, `require_ok`.
  - `storage_mapping.rs`: storage mapping tests including RunAdmission.
  - `queued.rs`: queued flush/drain/queue-full tests and the adjusted shutdown-drain assertion.
  - `config.rs`: config profile tests.

### 3. Shard implementation split

Goal: shrink `shard/impl_.rs` under 300 without changing `Shard` public methods.

Files:

- Keep `crates/vb_runtime/src/shard/impl_.rs` with constructor/enqueue/tick/evidence-drain core only.
- Add `crates/vb_runtime/src/shard/accessors.rs`:
  - `command_queue_len`, `remaining_capacity`, `is_queue_full`, `command_queue_capacity`, `active_run_count`, `pending_timer_count`, `frame_pool_metrics`, `trace_ring_mut`, `snapshot_run`, `take_inspect_response`, `status`.
- Add `crates/vb_runtime/src/shard/frame_pool.rs`:
  - `take_frame_for`, `release_frame`.
- Add `crates/vb_runtime/src/shard/shutdown.rs`:
  - `drain_for_shutdown`.
- Add `crates/vb_runtime/src/shard/config.rs`:
  - `impl ShardConfig::new`.
- Update `crates/vb_runtime/src/shard/mod.rs` to include the new modules.

Tests:

- Move `impl_.rs` inline tests to `shard/impl_tests/` or merge into split `shard/tests/` groups below; no test file may exceed 300 lines.

### 4. Shard lifecycle split with typed header mode

Goal: shrink `shard/lifecycle.rs` under 300 and remove the new boolean control flag.

Files:

- Add `crates/vb_runtime/src/shard/submission.rs`:
  - `enum HeaderPersistence { PersistBeforeRunState, AlreadyPersisted }`
  - `impl Shard::{handle_submit, handle_submit_pre_persisted, handle_submit_with_inputs}`
  - private `handle_submit_with_inputs_and_header_mode(..., header: HeaderPersistence)`
  - private `build_admission`
  - This replaces `persist_header: bool` and makes the pre-persisted workflow explicit.
- Add `crates/vb_runtime/src/shard/action_failure.rs`:
  - `ActionFailureOutcome`, `retry_is_available`, `apply_error_handler`, `write_failure_slot`, `ticket_with_retry_capacity`, `apply_action_failure_to_state`, `handle_action_failure`.
- Add `crates/vb_runtime/src/shard/action_completion.rs`:
  - `handle_action_completion`, `handle_legacy_action_completion`.
- Add `crates/vb_runtime/src/shard/wait_ask_timer.rs`:
  - `handle_ask_answer`, `handle_timer`.
- Add `crates/vb_runtime/src/shard/control.rs`:
  - `handle_resume`, `handle_cancel`, `handle_inspect`.
- Keep `crates/vb_runtime/src/shard/lifecycle.rs` with shared drive helpers only:
  - `drive_run`, `take_run_state`, `drive_state`, `apply_drive_result`.
- Update `shard/mod.rs` with new modules. Keep `handle_*` methods `pub(crate)` exactly as currently consumed by `impl_.rs`.

### 5. Shard mega-test split

Goal: replace `crates/vb_runtime/src/shard/tests.rs` with a small dispatcher and files under 300 lines.

Files:

- Replace `crates/vb_runtime/src/shard/tests.rs` with:
  - `mod support;`
  - `mod retry_failure; mod submit_capacity; mod inspect_trace; mod timers; mod config_types; mod command_eq; mod action_completion; mod action_failure; mod ask_answer; mod frame_pool; mod shutdown; mod black_hat; mod bdd_shutdown; mod drain_shutdown;`
- Move helper workflows to `shard/tests/support.rs`; if it exceeds 300, split into `support/workflows.rs`, `support/assertions.rs`, and `support/builders.rs` with `support.rs` re-exporting.
- Put the new bead-local duplicate-submit test `submit_rejects_duplicate_run_id` in `shard/tests/submit_capacity.rs` beside existing duplicate/capacity tests.
- Keep tests using `use super::support::*; use crate::shard::{Shard, ShardCommand, ShardConfig};` rather than exporting new production APIs.

### 6. Admission integration test split

Goal: keep one integration test crate name while every file stays below 300 lines.

Files:

- Keep `crates/velvet_ballistics/tests/admission_evidence_integration.rs` under 300 lines with only module declarations:
  - `mod admission_evidence_support;`
  - `mod admission_storage;`
  - `mod admission_execution;`
  - `mod admission_policy;`
  - `mod admission_taint;`
- Add directory `crates/velvet_ballistics/tests/admission_evidence_integration/` with:
  - `admission_evidence_support.rs`: `fail_assert`, workflow builders, `test_config`, `temp_journal`, `FailingBeforeHeaderJournal`.
  - `admission_storage.rs`: `storage_failure_before_header_prevents_ack`, `restart_lookup_finds_persisted_header`, `submit_artifact_then_run_succeeds`.
  - `admission_execution.rs`: `evidence_chain_after_execution`.
  - `admission_policy.rs`: `run_without_artifact_under_relaxed_policy`, `capability_check_rejects_unauthorized_action`, `budget_validation_rejects_oversized_workflow`.
  - `admission_taint.rs`: `taint_propagates_through_expression_eval`.

### 7. Commands for implementation agent

Run from `/home/lewis/src/Velvet-ballistics-vb-qi37-4-3-go` only:

```bash
jj status
jj diff --stat

# After mechanical moves:
cargo fmt --all
cargo test -p vb_runtime runtime::tests::submission::submit_direct_returns_durability_error_before_ack_when_header_cannot_persist
cargo test -p vb_runtime shard::tests::submit_capacity::submit_rejects_duplicate_run_id
cargo test -p velvet-ballistics --test admission_evidence_integration storage_failure_before_header_prevents_ack
cargo test -p velvet-ballistics --test admission_evidence_integration restart_lookup_finds_persisted_header

# Line-count gate for touched runtime/test files:
python3 - <<'PY'
from pathlib import Path
roots = [Path('crates/vb_runtime/src'), Path('crates/velvet_ballistics/tests')]
bad = []
for root in roots:
    for path in root.rglob('*.rs'):
        lines = sum(1 for _ in path.open())
        if lines > 300:
            bad.append((lines, path))
for lines, path in sorted(bad, reverse=True):
    print(f'{lines}: {path}')
raise SystemExit(1 if bad else 0)
PY

# If focused tests and line-count pass, rerun canonical gate for changed code:
moon ci
```

## Risk controls

- Use `git mv`/`mv -f` only inside the isolated workspace.
- Do not alter storage semantics, journal event ordering, queue behavior, or public method signatures.
- Prefer `pub(super)`/`pub(crate)` over `pub` for extracted helpers.
- If any moved test needs extra visibility, first move the test closer to the code under test; widen production visibility only as a last resort.
- After code movement, State 13 must be rerun from State 8 per the existing architectural-drift review note.

## Recommendation

Plan-only stop. A safe implementation is mechanical but broad: it must split four runtime source modules plus two mega-test files. Another agent can execute this plan without inventing behavior.
