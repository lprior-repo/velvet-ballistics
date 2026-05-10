# QA Report — vb-fb52

**Date:** Sat May 09 2026  
**State:** 9 (QA Enforcer)  
**Bead ID:** vb-fb52  
**Title:** storage: Atomic journal and index write batches  
**Scope:** `JournalWriteBatch<'j>` — Fjall-backed atomic write batch

---

## 1. Discovery

### 1.1 Bead Database Status
```
$ bd show vb-fb52 --json
{"error":"no issues found matching the provided IDs"}
```

**CRITICAL:** Bead `vb-fb52` does NOT exist in the beads database. Local STATE.md
indicates "Status: OPEN, Claimed: true" but the issue was never persisted to the
beads Dolt database.

### 1.2 Local Bead Directory
```
$ ls .beads/vb-fb52/
contract.md        test-plan.md      moon-report.md
test-plan-review.md  moon-report-test.md  STATE.md
```

**OBSERVATION:** Local files exist but are not synced to the beads database.

---

## 2. Test Plan Verification

### 2.1 Test Plan Exists
```
$ test -f .beads/vb-fb52/test-plan.md && echo "EXISTS"
EXISTS
```

- 300 lines
- Covers Unit (18), Integration (24), Property-based (10), BDD scenarios (14)
- Total planned tests: 52

### 2.2 Test Plan Scope Coverage

| Layer | Planned | Actual Batch Tests |
|-------|---------|-------------------|
| Unit | 18 | Included in 30 batch tests |
| Integration | 24 | Included in 30 batch tests |
| Property-based | 10 | Included in 30 batch tests |
| BDD scenarios | 14 | Covered by integration tests |

---

## 3. Moon :test Gate

### 3.1 Moon :test Exit Code: 101 (FAILED)

**Compilation Errors (not test failures):**

| Category | Count | Affected Files |
|----------|-------|----------------|
| Missing `serde_yaml` crate | Multiple | `xtask/src/evidence.rs` |
| Missing `cmd_ai_*` functions | 3 | `xtask/src/main.rs` |
| `EventSeq` private constructor | 1 | `crates/vb_storage/src/trimming.rs` |
| Missing `attempt` field on `JournalEvent` | 40+ | `recovery/replay/summary.rs`, `recovery/tests.rs` |

**Root Cause:** Pre-existing API/schema drift in `recovery` and `xtask` modules,
unrelated to `JournalWriteBatch` scope.

### 3.2 Moon :quick Gate
Not run explicitly in this session. Previous evidence in `.beads/vb-fb52/moon-report.md`.

---

## 4. Test Execution (batch module only)

```
$ cargo test -p vb_storage --lib -- batch::
cargo test: 30 passed, 892 filtered out (1 suite, 0.04s)
```

**PASS** — All 30 batch-specific tests pass.

### 4.1 Full vb_storage Lib Tests
```
$ cargo test -p vb_storage --lib
909 passed; 13 failed; 0 ignored
```

**13 Pre-existing Failures (NOT in batch scope):**

| Test | Module | Assertion |
|------|--------|-----------|
| `gate_values_outside_range_fail_is_valid` | admission | gate 14 should be invalid |
| `is_valid_rejects_gate_fourteen` | admission | `!w.is_valid()` |
| `submit_artifact_strict_is_durable` | admission | gate_count == 2 (got 15) |
| `submit_artifact_journaled_runs_both_gates` | admission | gate_count == 2 (got 15) |
| `frame_seed_slot_dimension_overflow_reports_exact_variant` | recovery | matches variant |
| `event_slot_values_cover_valid_corrupt_and_missing_frame_paths` | recovery | slot recovery |
| `recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states` | recovery | state rebuild |
| `bdd_strict_policy_enforces_gates_and_syncall` | vb_2bok | gate_count == 2 (got 15) |
| `bdd_journaled_policy_enforces_both_gates` | vb_2bok | gate_count == 2 (got 15) |
| `gate_count_two_for_journaled` | vb_2bok | Journaled gate_count == 2 |
| `gate_count_two_for_strict` | vb_2bok | Strict gate_count == 2 |
| `submit_artifact_journaled_enforces_both_gates` | vb_2bok | exactly 2 gates |
| `submit_artifact_strict_enforces_gates_plus_syncall` | vb_2bok | exactly 2 gates |

**All 13 failures are in `admission` and `recovery` modules — NOT in `batch` module.**

---

## 5. Findings

### CRITICAL (Block Merge)
1. **Bead Not in Database:** `vb-fb52` does not exist in the beads Dolt database.
   - Local `.beads/vb-fb52/` exists with contract and test plan
   - `bd show vb-fb52` returns "no issue found matching vb-fb52"
   - **Cannot proceed to State 10 without bead registration**

### MAJOR (Fix Before Merge)
2. **Moon :test Compilation Errors:** Pre-existing API drift causes compilation failure in:
   - `xtask/src/evidence.rs` — missing `serde_yaml` crate
   - `xtask/src/main.rs` — missing `cmd_ai_*` functions
   - `crates/vb_storage/src/trimming.rs` — `EventSeq` private constructor
   - `recovery/replay/summary.rs` and `recovery/tests.rs` — missing `attempt` fields

### MINOR
3. **Dead Code Warning:** `staged_event_keys` field never read in `JournalWriteBatch`
   - `crates/vb_storage/src/batch.rs:41`

---

## 6. Batch Module Test Evidence

```
$ cargo test -p vb_storage --lib -- batch:: 2>&1 | grep -E "test result"
cargo test: 30 passed, 892 filtered out (1 suite, 0.04s)
```

All `JournalWriteBatch` tests pass. The scope of vb-fb52 (atomic journal and index
write batches) is implemented and tested.

---

## 7. Auto-fixes Applied

None — this is a QA review session, not an implementation session.

---

## 8. Beads Filed

| ID | Title | Severity |
|----|-------|----------|
| (pending) | Bead vb-fb52 not registered in Dolt database | CRITICAL |
| (pending) | Moon :test compilation errors in xtask and recovery | MAJOR |

---

## 9. VERDICT

```
┌─────────────────────────────────────────────────────────────┐
│  QA DECISION:  ❌ REJECTED                                 │
│                                                             │
│  REASON: Bead vb-fb52 does not exist in beads database.    │
│          Moon :test has pre-existing compilation errors.    │
│                                                             │
│  CAN PROCEED TO STATE 10: NO                               │
│                                                             │
│  BLOCKING ISSUES:                                           │
│  1. CRITICAL: Bead not in beads database                  │
│  2. MAJOR: Moon :test compilation failures                │
│                                                             │
│  NON-BLOCKING:                                              │
│  - 30/30 batch tests pass (vb-fb52 scope)                 │
│  - 13 failures are pre-existing and out-of-scope           │
└─────────────────────────────────────────────────────────────┘
```

**Next Steps:**
1. Register bead `vb-fb52` in the beads Dolt database via `bd create` or sync
2. Fix pre-existing compilation errors in `xtask` and `recovery` modules
3. Re-run State 9 QA after bead is properly registered

---

*QA Enforcer — State 9*
