# Journal Tests V4 - Corrected API Bindings

## Defects fixed from V3

- **EventSeq import**: V3 imported from `vb_core::ids`; corrected to `vb_storage::EventSeq` per actual type location.
- **Queue adapter type**: V3 used `QueuedStorageRuntimeJournal::shared_journaled()` returning `Arc<dyn RuntimeJournal>` where `drain_all` is not in scope; corrected to `QueuedStorageRuntimeJournal::journaled()` returning the concrete adapter so its inherent `drain_all` method is callable.
- **StorageLimits construction**: V3 hand-built a `StorageLimits` struct literal guessing the single field name; corrected to use `StorageLimits::DEFAULT` per the actual constant.
- **Redundant Resumed test**: V3 appended an extra `RuntimeJournalEvent::Resumed` after drain with no assertion; removed this dead check since the earlier later-append and drain already exercises acceptance.
- **Panic in noop/volatile test**: V3 used `match` with `panic!` on non-matching outcomes; corrected to exact `matches!` assertion on `Err(RuntimeError::UnsupportedOperation { operation: "append_policy_decision_strict" })` with no panic fallback.
- **Queue ordering verification**: V3 only verified events after the final `drain_all`, which would not detect if the strict policy decision failed to flush preceding queued prefix. Corrected to assert full `run_a` events (RunAccepted + PolicyDecisionEvent) immediately after `append_policy_decision_strict` and before any later RunFinished append, proving the flush-before-strict property.
- **Fixture simplification**: Decision digests changed from sequential [1..32]/[32..1] sequences to uniform [17;32]/[29;32]/[43;32] arrays; ticket fields retained with 9 explicit fields and 8-field event records for full structural coverage without unnecessary byte diversity.

## Test coverage

- `storage_adapter_persists_full_policy_decision_event`: StorageRuntimeJournal roundtrip of PolicyDecisionEvent through Fjall.
- `queued_adapter_drains_prefix_before_strict_decision`: QueuedStorageRuntimeJournal flush-before-strict property; run_a prefix + policy decision verified before run_b later-append; run_b verified after final drain.
- `noop_and_volatile_adapters_fail_closed`: Exact UnsupportedOperation variant matching for both non-storage adapters.

## Not yet verified

Source-only file in isolated workspace. Root will compile actual tests after Runtime compiler repair adds `RuntimeJournal::append_policy_decision_strict`. No shell, no test runner, no VCS operations performed.