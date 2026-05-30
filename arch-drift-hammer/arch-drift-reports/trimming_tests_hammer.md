# Architectural Drift Report: `trimming/tests.rs`

**File**: `crates/vb_storage/src/trimming/tests.rs`
**Total Lines**: 1092
**Line Limit**: 300
**Drift Severity**: CRITICAL (3.6x over limit)

---

## 1. Line Count Violation

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total Lines | 1092 | 300 | 🔴 CRITICAL |
| Test Functions | 25 | — | — |
| Helper Functions | 9 | — | — |
| Lines per Test (avg) | ~40 | — | — |

The file is **3.6x over the 300-line architectural limit**. This violates the foundational `<300 line` rule.

---

## 2. Test Responsibility Map

| # | Test Function | Responsibility | Scenario Count |
|---|---------------|----------------|----------------|
| 1 | `trim_given_run_with_events_seq_0_to_9_and_snapshot_at_seq_5_trims_0_to_4` | Core trim behavior | 1 |
| 2 | `trim_given_run_already_trimmed_is_noop` | Idempotency | 1 |
| 3 | `trim_given_run_with_no_snapshot_returns_error` | Error handling | 1 |
| 4 | `trim_preserves_run_header_and_snapshot` | Boundary preservation | 1 |
| 5 | `trim_all_eligible_runs_skips_runs_without_snapshots` | Batch trim | 1 |
| 6 | `latest_durable_snapshot_seq_returns_highest_seq` | Snapshot query | 1 |
| 7 | `latest_durable_snapshot_seq_returns_none_for_no_snapshots` | Empty case | 1 |
| 8 | `latest_durable_snapshot_seq_rejects_payload_run_mismatch` | Validation | 1 |
| 9 | `latest_durable_snapshot_seq_rejects_payload_seq_mismatch` | Validation | 1 |
| 10 | `trim_preserves_events_at_or_after_snapshot` | Boundary edge case | 1 |
| 11 | `terminal_retention_blocks_recent_terminal_runs` | Retention policy | 1 |
| 12 | `terminal_retention_allows_older_terminal_runs` | Retention policy | 1 |
| 13 | `non_terminal_runs_ignore_retention_policy` | Terminal vs non-terminal | 1 |
| 14 | `replay_equivalence_after_trim` | Correctness verification | 1 |
| 15 | `trim_policy_default_includes_retention` | Default configuration | 1 |
| 16 | `no_durable_snapshot_error_has_correct_diagnostic_code` | Error codes | 1 |
| 17 | `retention_policy_blocks_error_has_correct_diagnostic_code` | Error codes | 1 |
| 18 | `diagnostic_returns_eligible_and_blocked_runs` | Diagnostic | 1 |
| 19 | `diagnostic_reports_correct_safe_point_and_trimmable_count` | Diagnostic | 1 |
| 20 | `diagnostic_blocks_run_without_durable_snapshot` | Diagnostic | 1 |
| 21 | `diagnostic_blocks_recent_terminal_run_under_retention` | Diagnostic | 1 |
| 22 | `diagnostic_allows_non_terminal_run_despite_retention` | Diagnostic | 1 |
| 23 | `diagnostic_does_not_delete_events` | Diagnostic immutability | 1 |
| 24 | `diagnostic_is_idempotent` | Diagnostic repeatability | 1 |
| 25 | `diagnostic_returns_empty_for_empty_journal` | Empty journal | 1 |

**25 distinct test scenarios** in a single file. These should be organized into **behavior groups** with shared scenario builders.

---

## 3. Primitive Obsession Violations

### 3.1 `RunSnapshot` Binary Blobs (lines 121-127, 172-178, 234-240, etc.)

```rust
let snapshot = RunSnapshot {
    run,
    seq: EventSeq::new(5),
    workflow: digest,
    slots: vec![0u8],      // 🔴 RAW: Vec<u8> without domain type
    taint: vec![],         // 🔴 RAW: Vec<u8> without domain type
};
```

**Violation**: `slots` and `taint` are `Vec<u8>` — raw byte collections with no type safety. These represent **slot values** and **taint markers** respectively but lack domain wrappers.

**Scott Wlaschin DDD Principle**: Replace primitives with Value Objects. `SlotValues` and `TaintMarkers` should wrap `Vec<u8>` with domain semantics.

### 3.2 `RunHeaderRecord.status: u8` (lines 54, 73)

```rust
let header = RunHeaderRecord {
    run,
    workflow_id: WorkflowId::new(0),
    compiled_digest: digest,
    status: 0,             // 🔴 RAW: u8 instead of RunHeaderStatus
    accepted_at_ms: 0,     // 🔴 RAW: u64 instead of TimestampMs
};
```

**Violation**: Despite `RunHeaderRecord` providing `run_header_status()` / `set_run_header_status()` helpers, the tests construct raw `u8` values. This bypasses the type safety the domain model provides.

### 3.3 `accepted_at_ms: u64` as Raw Timestamp (lines 55, 74)

```rust
accepted_at_ms: 0,  // 🔴 RAW: u64 instead of TypedTimestamp
```

**Violation**: `u64` milliseconds since epoch is a primitive. Should be `TimestampMs` or similar domain type.

### 3.4 Test Helper: Raw `u64` Event Sequence Construction

```rust
fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),  // Accepts raw u64
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}
```

**Violation**: `make_event` accepts `seq: u64` instead of `EventSeq`. Tests must remember to wrap it, creating an opportunity for raw primitive leakage.

### 3.5 Test Helper: Raw `u16` Step Index (line 31, 35)

```rust
fn make_step_started(run: RunId, seq: u64, step: u16) -> JournalEvent {
    JournalEvent::StepStarted {
        run,
        seq: EventSeq::new(seq),
        step: StepIdx::new(step),  // Accepts raw u16
        attempt: 1,
    }
}
```

**Violation**: `StepIdx::new(step)` accepts raw `u16`. Type system should prevent zero or out-of-range step indices.

### 3.6 `attempt: 1` as Raw Literal (lines 36, 45)

```rust
attempt: 1,  // 🔴 RAW: u32 literal instead of AttemptNumber
```

**Violation**: `attempt` should be a typed `AttemptNumber` or similar.

### 3.7 Raw Loop Counters (lines 108, 431, 481, etc.)

```rust
let events: Vec<JournalEvent> = (0..10u64)
    .map(|i| { ... })
    .collect();
```

**Violation**: Raw `u64` counter in iterator. Should use `EventSeq` range.

---

## 4. Privileged Escape Hatches

### 4.1 `insert_snapshot_payload_under_key` (lines 81-100)

```rust
fn insert_snapshot_payload_under_key(
    journal: &FjallJournal,
    key_run: RunId,
    key_seq: EventSeq,
    payload: &RunSnapshot,
) {
    let key = crate::keys::run_snapshot_key(key_run, key_seq).expect("snapshot key");
    let value = crate::codec::encode_record(
        crate::constants::MAGIC_SNAPSHOT,  // 🔴 Hardcoded magic number
        crate::records::RecordKind::Snapshot,
        payload.seq.get(),                  // 🔴 Extracts inner u64
        payload,
        crate::constants::MAX_SNAPSHOT_BYTES,  // 🔴 Magic constant
    )
    .expect("snapshot payload encode");
    journal.run_snapshot.insert(key.to_vec(), value).expect("...");
}
```

**Violation**: This function exercises **internal encoding machinery** that production code should never touch directly. It tests codec internals (`MAGIC_SNAPSHOT`, `RecordKind::Snapshot`, `MAX_SNAPSHOT_BYTES`) rather than public API behavior.

**This is a white-box test** that exposes implementation details instead of testing behavior.

### 4.2 Direct Key Construction (lines 671-677)

```rust
for seq in 0..3u64 {
    let key = crate::keys::run_event_key(run, EventSeq::new(seq)).expect("key ok");
    assert!(
        journal.events.get(key).expect("get ok").is_none(),
        "event seq {} should be deleted",
        seq
    );
}
```

**Violation**: Directly reads from `journal.events` internal Fjall store to verify deletion. Should use public API `events_for_run()` and verify absence there.

---

## 5. Duplication Patterns

### 5.1 Snapshot Construction Boilerplate (repeated ~18 times)

```rust
let snapshot = RunSnapshot {
    run,
    seq: EventSeq::new(X),
    workflow: digest,
    slots: vec![0u8],
    taint: vec![],
};
```

**Every test** that needs a snapshot repeats this exact pattern. A **scenario builder** or **test fixture** should abstract this.

### 5.2 Event Construction Boilerplate (repeated ~15 times)

```rust
let events: Vec<JournalEvent> = (0..Nu64)
    .map(|i| {
        if i == 0 {
            make_event(run, i)
        } else {
            make_step_started(run, i, i as u16 - 1)
        }
    })
    .collect();
```

**Every test** re-implements this pattern. A **scenario builder** should provide `with_events(run, count)` or similar.

### 5.3 Header Write Boilerplate (repeated ~10 times)

```rust
write_header(&journal, run, digest);
```

Better than repeating the full struct construction, but still involves raw `WorkflowId::new(0)`.

---

## 6. DDD Cohesion Violations

### 6.1 Test File as Aggregate Root

The `tests.rs` file acts as a ** monolithic test aggregate** instead of being decomposed into **scenario modules**:

```
trimming/
├── mod.rs           (156 lines - domain logic)
└── tests.rs         (1092 lines - ALL scenarios)
```

**Should be**:
```
trimming/
├── mod.rs
├── tests/
│   ├── basic_trim_scenarios.rs
│   ├── retention_policy_scenarios.rs
│   ├── diagnostic_scenarios.rs
│   └── snapshot_validation_scenarios.rs
```

### 6.2 Missing Scenario Builders

Tests should use **scenario builder functions** that create meaningful domain situations:

```rust
// SHOULD EXIST but doesn't:
fn terminal_run_scenario(journal: &FjallJournal, workflow_id: WorkflowId, run_id: u64) -> RunId;
fn non_terminal_run_scenario(journal: &FjallJournal, ...) -> RunId;
fn eligible_run_scenario(journal: &FjallJournal, ...) -> (RunId, EventSeq);
```

Instead, every test manually constructs scenarios from primitives.

---

## 7. Recommendations (Priority Order)

### P0 — Immediate (Line Count)

1. **Split file into scenario groups**:
   - `tests/basic_trim.rs` — scenarios 1-5 (basic trim behavior)
   - `tests/retention_policy.rs` — scenarios 11-13 (retention logic)
   - `tests/diagnostic.rs` — scenarios 18-25 (diagnostic queries)
   - `tests/snapshot_validation.rs` — scenarios 6-10, 14-17 (validation)
   - `tests/mod.rs` — re-exports all scenario modules

2. **Create shared scenario builders** in `tests/common/`:
   - `run_scenario.rs` — `terminal_run()`, `non_terminal_run()`, `eligible_run()`
   - `snapshot_helpers.rs` — `snapshot_at_seq()`, `latest_snapshot()`
   - `journal_helpers.rs` — `temp_journal()`, `append_run_events()`

### P1 — Primitive Obsession Fixes

3. **Add `SlotValues` and `TaintMarkers` value objects** in `recovery/types.rs`:
   ```rust
   pub struct SlotValues(Vec<u8>);
   pub struct TaintMarkers(Vec<u8>);
   ```

4. **Replace `slots: Vec<u8>` and `taint: Vec<u8>`** in `RunSnapshot` with domain wrappers.

5. **Use typed `AttemptNumber`** for the `attempt` field in events.

### P2 — Test Design

6. **Remove `insert_snapshot_payload_under_key`** — tests should not bypass public API.

7. **Remove direct `journal.events` access** — verify deletion through public API only.

8. **Replace raw `u64`/`u16` in helper functions** with proper domain types.

---

## 8. Summary

| Category | Count | Severity |
|----------|-------|----------|
| Line Count Violation | 1092 / 300 | 🔴 CRITICAL |
| Primitive Obsessions | 7 distinct | 🔴 HIGH |
| Privileged Escape Hatches | 2 | 🔴 HIGH |
| Duplication Patterns | 3 major | 🟡 MEDIUM |
| DDD Cohesion Violations | 2 | 🔴 HIGH |

**Total Architectural Violations**: 5 critical/high, 2 medium
**Estimated Refactor Effort**: 3-4 beads (split file, create builders, add domain types, remove escape hatches)

---

*Report generated by arch-drift-hammer*
*Drift Agent: architectural-drift skill*
