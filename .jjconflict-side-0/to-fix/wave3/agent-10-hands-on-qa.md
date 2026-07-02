# Wave 3 Hands-On QA Report — Agent 10

**Scope:** 9 bug IDs (vb-igldl, vb-jd99y, vb-jnome, vb-joxhb, vb-jut5w, vb-k8eif, vb-keji6, vb-khvqm, vb-kz475)
**Date:** 2026-06-24
**Working dir:** `/home/lewis/src/velvet-ballistics`
**Method:** Targeted cargo test + source code review

---

## Summary Table

| bug-id | pri | affected-crate | targeted-cmd | exit-code | result | verdict | log-path |
|--------|-----|----------------|--------------|-----------|--------|---------|----------|
| vb-igldl | P1 | velvet-ballistics-workspace-tests | `cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery recovery_detects_unsupported_slot_taint` | 0 | 1 passed; 0 failed | **PATCHED** | /tmp/qa-vb-igldl.log |
| vb-jd99y | P1 | vb_runtime | `cargo test -p vb_runtime --lib recovery` (+ source review of `shard/lifecycle/chunk_001.rs`) | 0 | 12 lib tests passed; 615 shard tests passed | **NOT-PATCHED** (source still bugged) | /tmp/qa-vb-jd99y-lib-recovery.log |
| vb-jnome | P1 | vb_storage | `cargo check -p vb_storage --all-targets --all-features` + `cargo test -p vb_storage --test recovery_property_tests proptest_seed_dimensions` | 0 | check OK; 1 passed; 0 failed | **PATCHED** | /tmp/qa-vb-jnome.log |
| vb-joxhb | P2 | vb_storage | `cargo test -p vb_storage --lib queue::tests` + source review of `queue/writer.rs:130-213` | 0 | 46 queue tests passed | **NOT-PATCHED** (source still bugged) | /tmp/qa-vb-joxhb-queue.log |
| vb-jut5w | P0 | velvet-ballistics-workspace-tests | `cargo test -p velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery --test integration_storage_runtime_validate_pipeline --test recovery_watermark_tests` | 0 | 13+15+21 = 49 passed; 0 failed | **PATCHED** | /tmp/qa-vb-jut5w-final.log |
| vb-k8eif | P3 | vb_storage | `cargo test -p vb_storage --lib journal_wrapped_error_delegates_to_inner_diagnostic_code` | 0 | 1 passed; 0 failed | **PATCHED** | /tmp/qa-vb-k8eif.log |
| vb-keji6 | P2 | vb_storage | `cargo test -p vb_storage --lib batch_append_event_allows_duplicate_key_insertion` + source review of `batch.rs:243-251` | 0 | 1 passed (test confirms buggy behavior) | **NOT-PATCHED** (source still bugged) | /tmp/qa-vb-keji6.log |
| vb-khvqm | P2 | vb_runtime | `cargo test -p vb_runtime --lib storage_runtime_journal_probe` | 0 | 2 passed; 0 failed | **PATCHED** (duplicate of vb-odiyq) | /tmp/qa-vb-khvqm.log |
| vb-kz475 | P1 | vb_storage | `cargo test -p vb_storage --lib apply_tail_events` + `cargo test -p vb_storage --lib recovery` + source review of `recovery/hydrate_support.rs:395` | 0 | 1 + 213 passed | **NOT-PATCHED** (source still bugged) | /tmp/qa-vb-kz475-recovery.log |

---

## Counts

- bugs-checked: **9**
- PATCHED: **5** (vb-igldl, vb-jnome, vb-jut5w, vb-k8eif, vb-khvqm)
- NOT-PATCHED: **4** (vb-jd99y, vb-joxhb, vb-keji6, vb-kz475)
- PARTIAL: **0**
- UNKNOWN: **0**

---

## Test Regressions Detected

**None.** All broader crate-level test suites that were exercised passed cleanly:
- `vb_storage --lib`: 1270 passed; 0 failed
- `vb_runtime --lib`: 1734 passed; 0 failed
- `vb_runtime --lib shard`: 615 passed; 0 failed
- `vb_runtime --lib recovery`: 12 passed; 0 failed
- `vb_runtime --test recovery_integration`: 16 passed; 0 failed
- `vb_runtime --test recovery_hydration_tests`: 41 passed; 0 failed
- `vb_runtime --test durable_resume_red_phase` (filtered resume_inv001): 2 passed; 0 failed
- `velvet-ballistics-workspace-tests --test integration_storage_runtime_recovery`: 13 passed; 0 failed
- `velvet-ballistics-workspace-tests --test integration_storage_runtime_validate_pipeline`: 15 passed; 0 failed
- `velvet-ballistics-workspace-tests --test recovery_watermark_tests`: 21 passed; 0 failed

However: the absence of regressions masks 4 still-buggy implementations, because
the existing test corpus was authored either **before** the fix was attempted
or **explicitly codifies the buggy behavior** as the spec.

---

## Top-3 NOT-PATCHED with evidence

### 1. vb-joxhb (SA-005 — JournalWriterQueue::flush_batch holds mutex across Fjall writes)

- **Targeted exit-code:** 0
- **Targeted test:** `cargo test -p vb_storage --lib queue::tests` (46 passed)
- **Source evidence:** `crates/vb_storage/src/queue/writer.rs:131-213` still acquires `self.state.lock()` at line 135 and holds it through every `journal.append_queued_unfsynced(&item.event)` (line 165, 191) and `journal.persist_strict()` (line 168). The bead's close-reason claims "flush_batch now releases self.state mutex before journal IO and persist_strict fsync (queue/writer.rs:113-209)" — this fix is **not present** in the file.
- **Last error line:** N/A (test passes; bug is observable only under concurrent producer load, which is not exercised by the current queue tests).

### 2. vb-keji6 (SA-003 — append_event intra-batch dedup)

- **Targeted exit-code:** 0
- **Targeted test:** `cargo test -p vb_storage --lib batch_append_event_allows_duplicate_key_insertion` — **PASSES** (this is the smoking gun: the test asserts intra-batch duplicates ARE allowed, exactly the buggy behavior).
- **Source evidence:** `crates/vb_storage/src/batch.rs:243-251` — `append_event` checks only `self.journal.events.contains_key(key)?` (committed-state lookup); the `staged_event_keys: HashSet` field (line 47) is **declared and constructed but never consulted** in the duplicate guard. The I20 doc-comment at line 217-220 even states: *"Same-batch idempotent inserts are allowed (duplicates within the same batch are collapsed at commit time)."* — this codifies the bug as the spec. The bead's close-reason claims the fix is at `batch/write_event.rs:27-33`; no such file exists (batch code lives at `batch.rs:243`), and the check is still on committed state only.
- **Last error line:** N/A (test passes by design; bug is real per source review).

### 3. vb-kz475 (SR-003 — apply_tail_events ignores SlotWrittenEvent.extra)

- **Targeted exit-code:** 0
- **Targeted test:** `cargo test -p vb_storage --lib apply_tail_events` (1 passed) + `cargo test -p vb_storage --lib recovery` (213 passed)
- **Source evidence:** `crates/vb_storage/src/recovery/hydrate_support.rs:395` — match arm is `JournalEvent::SlotWrittenEvent { slot, value, .. }` (the `..` does not bind `extra`). Taint is derived from `frame.read_taint(*slot)` (line 404), exactly the stale-frame-taint path described in the bug-hunt SR-003 finding. The suggested fix (use `recovered_slot_taint(slot, slot_value, extra)` from `summary.rs:704`) is **not applied**.
- **Last error line:** N/A (test passes; bug only manifests when freshly-hydrated frames see SlotWrittenEvents with non-Clean encoded taint, which no existing test exercises).

---

## Additional NOT-PATCHED detail

### vb-jd99y (RQ-W0-20 — RuntimeState::Resuming has no recovery path)

- **Targeted exit-code:** 0
- **Targeted test:** `cargo test -p vb_runtime --lib recovery` (12 passed) + `cargo test -p vb_runtime --lib shard` (615 passed)
- **Source evidence:** `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs:307-331` — `handle_resume` explicitly returns `NotResumable` at line 318 if `current_state != RuntimeState::Resumable`. The `Resuming` state is therefore unrecoverable: after a process crash mid-resume (state left as `Resuming` per `transitions.rs:56`), every subsequent `handle_resume` call fails with `NotResumable`. No recovery-specific arm handles `Resuming`. The kani harnesses at `kani_resume_state_machine.rs` only test `Resuming → Resumable` via the explicit `ResumeRollback` event, not the post-crash recovery path.
- **Last error line:** N/A (no existing test reaches this code with state = Resuming).

---

## File-path written

`/home/lewis/src/velvet-ballistics/to-fix/wave3/agent-10-hands-on-qa.md`