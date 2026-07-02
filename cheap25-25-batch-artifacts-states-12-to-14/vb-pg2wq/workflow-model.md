# Workflow Model — vb-pg2wq

**Bead:** vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Lane:** Rust-local + test-only assertion repair

## Workflow: Proptest Cross-Batch Duplicate Detection

This is the **single workflow** in scope. It describes the runtime sequence the 5 weak proptest functions exercise, and the contract assertion that lands at the terminal state.

### Legal States

| State | Invariant |
|-------|-----------|
| `S0_JournalEmpty` | Fresh `tempfile::tempdir()`; `FjallJournal` opened; no events in keyspace. |
| `S1_BatchOpen` | `b1 = JournalWriteBatch::new(&journal)`; `b1.len() == 0`, `b1.is_aborted() == false`. |
| `S2_EventStaged` | `b1.append_event(&event) == Ok(())`; `b1.len() == 1`, `b1.staged_event_keys.contains(key) == true`. |
| `S3_BatchCommitted` | `b1.commit() == Ok(())`; durable keyspace `journal.events` now contains `key`; `b1.is_aborted() == false`. |
| `S4_SecondBatchOpen` | `b2 = JournalWriteBatch::new(&journal)`; `b2.len() == 0`, `b2.is_aborted() == false` (for PS_004 first assertion only — `b2` is opened AFTER `S3_BatchCommitted`). |
| `S5_DuplicateRejected` | `b2.append_event(&event) == Err(JournalError::DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(seq) })`. For PS_004: `b2.is_aborted() == true` as a side effect. |

### Terminal States

| Terminal State | Outcome | Test Assertion |
|----------------|---------|----------------|
| `T_Pass` | All assertions succeed; proptest accepts the shrunk input. | `prop_assert!` and `prop_assert_eq!` all true. |
| `T_Fail_Variant` | `b2.append_event` returns a non-`DuplicateEvent` variant. | `let-else` panic or `matches!` guard returns `false`. |
| `T_Fail_Tuple` | `b2.append_event` returns `DuplicateEvent` with `run`/`seq` differing from inputs. | `assert_eq!` or `matches!` guard returns `false`. |
| `T_Fail_Ok` | `b2.append_event` returns `Ok(())` (silent overwrite regression). | `let-else` panic or `matches!` guard returns `false`. |

### Transitions

```
S0_JournalEmpty
  │ FjallJournal::open(tempdir, None)
  ▼
S1_BatchOpen
  │ b1.append_event(&event)
  ▼
S2_EventStaged
  │ b1.commit()
  ▼
S3_BatchCommitted
  │ JournalWriteBatch::new(&journal)  [second batch]
  ▼
S4_SecondBatchOpen
  │ b2.append_event(&event)
  ▼
S5_DuplicateRejected
  │ proptest assertion
  ▼
T_Pass  (or T_Fail_* on regression)
```

### Guards (per transition)

| Transition | Guard |
|------------|-------|
| `S0 → S1` | `tempfile::tempdir()` succeeds; `FjallJournal::open` succeeds. |
| `S1 → S2` | `event.is_valid()` is true; `key = run_event_key(event.run_id(), event.seq())` is `Ok`; `b1.staged_event_keys.contains(key) == false`; `journal.events.contains_key(key) == false`; `b1.inner.len() < MAX_BATCH_COUNT`; encoding succeeds; byte admission passes. |
| `S2 → S3` | `commit` succeeds; durable keyspace now contains `key`. |
| `S3 → S4` | `JournalWriteBatch::new(&journal)` succeeds; `b2.len() == 0`; `b2.is_aborted() == false` initially. |
| `S4 → S5` | `event.is_valid()` is true; `key = run_event_key(event.run_id(), event.seq())` is `Ok`; `b2.staged_event_keys.contains(key) == false`; **`journal.events.contains_key(key) == true`** (this is the cross-batch duplicate guard); production then sets `b2.aborted = true` and returns `Err(DuplicateEvent { run: event.run_id(), seq: event.seq() })`. |
| `S5 → T_*` | `prop_assert!` macro evaluates the field-bound `matches!` guard; for PS_004, also `b2.is_aborted()`, `b2.commit()`, `journal.events_for_run(...).len()`. |

### The Cross-Batch Duplicate Branch (production code path)

The transition `S4 → S5` is the one this bead pins. From `crates/vb_storage/src/batch/append_event.rs:42-67`:

```rust
pub fn append_event(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
    let key = run_event_key(event.run_id(), event.seq())?;
    if !event.is_valid() { return Err(JournalError::InvalidEvent); }
    if self.staged_event_keys.contains(&key) {
        return Err(JournalError::DuplicateStagedKey { run: event.run_id(), seq: event.seq() });
    }
    if self.journal.events.contains_key(key)? {                          // ← THE BRANCH
        self.aborted = true;                                            // ← SIDE EFFECT
        return Err(JournalError::DuplicateEvent {                       // ← THE TUPLE
            run: event.run_id(),                                        // ← exact (e.run_id(), e.seq())
            seq: event.seq(),
        });
    }
    ...
}
```

The contract: `run == event.run_id()` and `seq == event.seq()` (typed equality). The test assertion MUST pin this tuple.

### Commands (Test-Side)

| Command | Type | Effect |
|---------|------|--------|
| `make_event(run: u64, seq: u64) -> JournalEvent` | Helper | Constructs `JournalEvent::RunAccepted` with `WorkflowDigest::from_bytes([0u8; 32])`. |
| `temp_journal() -> (TempDir, FjallJournal)` | Helper | Creates a fresh temp dir and opens a `FjallJournal` rooted at it. |
| `b1.append_event(&event).expect("first")` | Imperative | Stages the event in batch `b1`. |
| `b1.commit().expect("commit")` | Imperative | Persists the staged event to the journal keyspace. |
| `b2.append_event(&event)` | Observed | Returns the value under test. |

### Events (Test-Side)

| Event | Source | Payload |
|-------|--------|---------|
| `event` (the `JournalEvent::RunAccepted`) | `make_event(run, seq)` | `{ run: RunId::new(run), seq: EventSeq::new(seq), workflow: WorkflowDigest::from_bytes([0u8; 32]) }` |
| `result` (the `Result<(), JournalError>`) | `b2.append_event(&event)` | In the happy-path of this workflow: `Err(DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(seq) })` |

### Per-File Workflow Variations

| File | Test Function | Input | Extra Post-S5 Assertions |
|------|--------------|-------|---------------------------|
| `proptest_vb_vzcuf_PS_001.rs` | `ps001_duplicate_rejected` | `run in 1u64..1000u64, seq in 0u64..100u64` | (none — just the strong `DuplicateEvent` assertion) |
| `proptest_vb_vzcuf_PS_003.rs` | `ps003_dup_fields` | `run in 1u64..1000u64, seq in 0u64..100u64` | (none — the function name lies; fix pins the "fields" the name promises) |
| `proptest_vb_vzcuf_PS_004.rs` | `ps004_no_persist` | `run in 1u64..1000u64` (seq is fixed at 0) | `prop_assert!(b2.is_aborted())`; `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))`; `prop_assert_eq!(events.len(), 1)` |
| `proptest_vb_vzcuf_PS_004.rs` | `ps004_empty_commit_after_rej` | `run in 1u64..1000u64, seq in 0u64..100u64` | `prop_assert!(b2.is_aborted())`; `prop_assert!(matches!(commit_result, Err(JournalError::BatchAborted)))` |
| `proptest_vb_vzcuf_PS_008.rs` | `ps008_dup_before_queue` | `run in 1u64..1000u64, seq in 0u64..100u64` | (none) |
| `proptest_vb_vzcuf_PS_009.rs` | `ps009_dup_rejected` | `run in 1u64..1000u64, seq in 0u64..100u64` | (none) |

### Idempotence

The proptest is idempotent in the sense that each input `(run, seq)` produces a fresh `tempfile::tempdir()` and a fresh `FjallJournal`; repeated runs of the same shrunk input do not leak state across iterations. (This is already true and is preserved.)

### Cancellation Paths

None. The workflow is synchronous, single-threaded, no `tokio` runtime, no cancellation tokens.

### Retry Semantics

None. The workflow is deterministic on `tempfile::tempdir()` + `FjallJournal::open`. No transient failures.

### Concurrency

None. All 5 proptest functions are sequential; `JournalWriteBatch::append_event` is `&mut self`. No shared state across proptest iterations.

### Temporal Hazards

| Hazard | Mitigation |
|--------|------------|
| Wall-clock dependence | None — `tempfile::tempdir()` uses system clock for directory naming, but the test does not read time. |
| Process scheduling | None — single-threaded. |
| File-system fsync timing | `b1.commit()` is synchronous and fsynced at the journal layer; the test does not assert fsync timing. |

### Hazards Specific to This Workflow

1. **Audit-regression-resistance (THE hazard):** The 6 weak `..` matches accept ANY `DuplicateEvent` tuple. A regression that mutates the production code to return wrong fields passes silently. **Fix: pin the tuple.**

2. **Variant confusion:** A regression that returns `DuplicateStagedKey { .. }` instead of `DuplicateEvent { .. }` would pass under the weak `..` pattern IF the `matches!` were rewritten as `Err(JournalError::DuplicateStagedKey { .. })`. The strong pattern binds the variant name explicitly, so the variant confusion regression fails.

3. **Setup mutation:** A regression that mutates the setup so `b1.commit()` fails would cause `expect("commit")` to panic. That panic is a setup failure, not an assertion failure; proptest reports it correctly. No change needed.

4. **Helper drift:** If `make_event` were changed to construct a different event variant, the strong assertion would still bind `run`/`seq` to the input `u64` values; this is intentional. No change to helpers is in scope.

### Outcome Lattice

```
           ┌── T_Pass  (proptest accepts; CI continues)
S5_DuplicateRejected ──┤
           └── T_Fail_Variant  (panic from let-else or prop_assert! false → proptest shrinks and reports)
           └── T_Fail_Tuple    (prop_assert! false → proptest shrinks and reports)
           └── T_Fail_Ok       (panic from let-else → proptest shrinks and reports)
```

The terminal states are exhaustive over `Result<(), JournalError>`. There is no "ambiguous" terminal state.

### Verification Lane Mapping (Workflow → Verifier)

| Workflow aspect | Lane |
|-----------------|------|
| Cross-batch duplicate detection (production) | **out of scope** — already proven by `kani_vb_vzcuf_ps004.rs` and existing strong tests; no production change. |
| Cross-batch duplicate detection (test-side contract) | **Rust-local + proptest** — `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_00X` is the per-file lane. |
| `let-else` exhaustiveness | **Rust-local + proptest** — same per-file test lane. |
| Field-bound `matches!` guard | **Rust-local + proptest** — same per-file test lane. |

No new Kani, Verus, Flux, Loom, or fuzz lanes are required by this bead. The test fix strengthens the runtime↔proof alignment without requiring new harnesses (Kani harness `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` already models `DuplicateEvent { run: r, seq: s }` with `r == run && s == seq` guards).