# vb-z7ky4 Journal Tests v3 — Test Map

**File:** `crates/vb_runtime/src/journal/tests/strict_policy_port.rs`
**Wiring:** `crates/vb_runtime/src/journal.rs` — separate `mod strict_policy_port_tests` with `include!`

## Test 1: `storage_adapter_persists_full_policy_decision_event`
- **Adapter:** `StorageRuntimeJournal::shared_journaled` (Journaled profile)
- **Action:** `append_policy_decision_strict` with full 9-field `DecisionRecord`
- **Verification:** Drop adapter + original Arc<FjallJournal>; reopen FjallJournal; `events_for_run` returns exact full `Vec<JournalEvent>` equal to expected (single `PolicyDecisionEvent`)
- **Key:** Proves actual Fjall persistence despite Journaled profile; durable reopen

## Test 2: `queued_adapter_drains_prefix_before_strict_decision`
- **Adapter:** `QueuedStorageRuntimeJournal::shared_journaled`
- **Action:** Enqueue `RunSubmitted` (seq 0); call `append_policy_decision_strict` (seq 1); enqueue `RunFinished` (seq 2); call `drain_all()`
- **Verification:** `events_for_run(run_a)` returns exact `RunAccepted` + `PolicyDecisionEvent`; `events_for_run(run_b)` returns exact `RunFinished`; queue still accepts after strict decision (append `Resumed`)
- **Key:** Proves prefix drain, strict decision interleaving, queue remains accepting

## Test 3: `noop_and_volatile_adapters_fail_closed`
- **Adapters:** `NoopRuntimeJournal::shared()` and `VolatileRuntimeJournal::shared()`
- **Action:** `append_policy_decision_strict` on each
- **Verification:** Exact match on `RuntimeError::UnsupportedOperation { operation: "append_policy_decision_strict" }` — no broad `is_err()`
- **Key:** Proves default impl fails closed (never silent success)

## Constraints Satisfied
- No `.unwrap`, `.expect`, `panic!`, `assert!(false)`, unchecked indexing, unchecked casts
- `Result<(), Box<dyn std::error::Error>>` with `?` for all fallible setup
- `Ok(JournalWriterQueue::new(...)?)` — no `as Box` casts
- Full `Vec` equality checks — no subset/prefix checks
- Distinct nonzero policy/input digests (1..32 vs 32..1 vs 128..159)
- All 9 `DecisionRecord` fields and all 8 `ActionTicket` fields retained
- Fixture ticket explicit for codec/trait test (not Kani)
- No Shard tests, no shell scripts, no deps/gate/source changes

## Not Run
No compile or test commands executed. Coordinator runs `moon run :build` and `cargo test -p velvet-ballistics --test journal-strict-policy-port` (or equivalent).
