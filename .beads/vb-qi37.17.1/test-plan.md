# Test Plan — vb-qi37.17.1: cli: Add incident command

## 1. Overview

This bead adds a CLI `incident` command to `vb_cli` that reads events from a `FjallJournal`,
produces a structured `IncidentReport`, and outputs JSON, JSONL, or Text with no stack traces.

Three layers of work:
- **Compile fixes** (56 E0061 errors on `recover_full_journal` and `replay_events` call sites)
- **Zero-unwrap fixes** (4 violations in `cmd_incident`)
- **Tests** (13 unit tests + 3 integration tests + static scans + manual QA)

**Why**: The contract requires every error path to return structured failure evidence without
stack traces (POST-003, INV-002), and all functions must compile (INV-005) and be zero-unwrap
(INV-001). The test plan below exercises every branch of `build_incident_report`, every
branch of `build_repair_hints`, and the I/O boundary of `cmd_incident`.

---

## 2. Unit Test Plan

All unit tests live in `crates/vb_cli/src/commands_incident.rs` under
`#[cfg(test)] mod tests { ... }`.

### 2.1 build_incident_report Tests

#### T-001 — Empty events
- **Purpose**: Prove `build_incident_report` returns clean defaults with no events.
- **Input**: `run_id = "run-1"`, `events = []` (empty slice)
- **Expected output**:
  - `report.run_id == "run-1"`
  - `report.failure_found == false`
  - `report.failure_code == ""`
  - `report.failed_at_step == None`
  - `report.side_effects.is_empty()` — `true`
  - `report.repair_hints.is_empty()` — `true`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001 (failure_found, failure_code, failed_at_step defaults)

#### T-002 — RunFailedEvent with preceding StepStarted
- **Purpose**: Prove failure detection + step tracking when a `RunFailedEvent` follows a `StepStarted`.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(5_u16), .. },
    JournalEvent::RunFailedEvent { .. },
  ]
  ```
- **Expected output**:
  - `report.failure_found == true`
  - `report.failure_code == "RunFailed"`
  - `report.failed_at_step == Some(5)`
  - `report.side_effects.is_empty()` — `true`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001

#### T-003 — ActionCompletedEvent + RunFailedEvent (side_effects)
- **Purpose**: Prove `ActionCompletedEvent` generates a side_effect entry with `certainty: "confirmed"`.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(3_u16), .. },
    JournalEvent::ActionCompletedEvent { step: StepIdx::from(3_u16), action: ActionId::from("save_db"), .. },
    JournalEvent::RunFailedEvent { .. },
  ]
  ```
- **Expected output**:
  - `report.failure_found == true`
  - `report.failure_code == "RunFailed"`
  - `report.side_effects.len() == 1`
  - `report.side_effects[0]["step"] == 3`
  - `report.side_effects[0]["action"] == "save_db"`
  - `report.side_effects[0]["certainty"] == "confirmed"`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001 (side_effects)

#### T-004 — ActionFailedEvent + RunFailedEvent (side_effects)
- **Purpose**: Prove `ActionFailedEvent` generates a side_effect entry with `certainty: "failed"`.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(2_u16), .. },
    JournalEvent::ActionFailedEvent { step: StepIdx::from(2_u16), action: ActionId::from("upload"), .. },
    JournalEvent::RunFailedEvent { .. },
  ]
  ```
- **Expected output**:
  - `report.failure_found == true`
  - `report.failure_code == "RunFailed"`
  - `report.side_effects.len() == 1`
  - `report.side_effects[0]["certainty"] == "failed"`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001 (side_effects)

#### T-005 — Multiple actions (ActionCompleted + ActionFailed + RunFailed)
- **Purpose**: Prove multiple side_effects accumulate correctly.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(1_u16), .. },
    JournalEvent::ActionCompletedEvent { step: StepIdx::from(1_u16), action: ActionId::from("step1_act"), .. },
    JournalEvent::ActionFailedEvent { step: StepIdx::from(1_u16), action: ActionId::from("step1_fail"), .. },
    JournalEvent::RunFailedEvent { .. },
  ]
  ```
- **Expected output**:
  - `report.side_effects.len() == 2`
  - `report.side_effects[0]["certainty"] == "confirmed"`
  - `report.side_effects[1]["certainty"] == "failed"`
  - `report.side_effects[0]["step"] == report.side_effects[1]["step"] == 1`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001 (side_effects accumulation)

#### T-006 — RunCancelled
- **Purpose**: Prove cancellation detection.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(7_u16), .. },
    JournalEvent::RunCancelled { .. },
  ]
  ```
- **Expected output**:
  - `report.failure_found == true`
  - `report.failure_code == "RunCancelled"`
  - `report.failed_at_step == Some(7)`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001

#### T-007 — Multiple StepStarted → failed_at_step is LAST step before failure
- **Purpose**: Prove `last_step_started` tracking updates on each `StepStarted`, so the last one wins.
- **Input**:
  ```
  events = [
    JournalEvent::StepStarted { step: StepIdx::from(1_u16), .. },
    JournalEvent::StepStarted { step: StepIdx::from(2_u16), .. },
    JournalEvent::StepStarted { step: StepIdx::from(4_u16), .. },
    JournalEvent::RunFailedEvent { .. },
  ]
  ```
- **Expected output**:
  - `report.failed_at_step == Some(4)` (NOT 1 or 2)
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-001

#### T-008 — Unknown/ignored variants → no panic
- **Purpose**: Prove the `_` arm in the match does not panic or produce side effects.
- **Input**: Events containing any `JournalEvent` variant other than `StepStarted`,
  `ActionCompletedEvent`, `ActionFailedEvent`, `RunFailedEvent`, or `RunCancelled`
  (e.g., `WorkflowStarted`, `StepCompleted`, or any future variant).
  Construct a minimal slice like:
  ```
  events = [
    JournalEvent::WorkflowStarted { .. },  // or any un-matched variant
    JournalEvent::StepCompleted { .. },    // or any un-matched variant
  ]
  ```
- **Expected output**:
  - Function returns normally (no panic, no crash)
  - `report.failure_found == false`
  - `report.failure_code == ""`
  - `report.side_effects.is_empty()` — `true`
  - `report.repair_hints.is_empty()` — `true`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: INV-001 (zero-panic on unknown variants)

---

### 2.2 build_repair_hints Tests

#### T-009 — RunFailed + empty side_effects + no step → 1 hint
- **Purpose**: Prove minimum hint output for `RunFailed` (step output investigation).
- **Input**: `failure_code = "RunFailed"`, `side_effects = []`, `failed_at_step = None`
- **Expected output**:
  - `hints.len() == 1`
  - `hints[0].as_str() == Some("investigate step output and engine logs for the failed step")`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-002 (RunFailed → always 1 hint)

#### T-010 — RunFailed + side_effects + step → 3 hints
- **Purpose**: Prove full hint output for `RunFailed` when all conditions are met.
- **Input**:
  ```
  failure_code = "RunFailed"
  side_effects = [serde_json::json!({"step": 1, "action": "save", "certainty": "confirmed"})]
  failed_at_step = Some(3)
  ```
- **Expected output**:
  - `hints.len() == 3`
  - `hints[0]` contains "investigate step output"
  - `hints[1]` contains "review side effects"
  - `hints[2]` contains "consider retry from step 3"
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-002 (RunFailed → side_effects + step → 3 hints)

#### T-011 — RunCancelled + empty side_effects → 1 hint
- **Purpose**: Prove cancellation hint when no side effects.
- **Input**: `failure_code = "RunCancelled"`, `side_effects = []`, `failed_at_step = None`
- **Expected output**:
  - `hints.len() == 1`
  - `hints[0].as_str() == Some("run was cancelled; check if cancellation was intentional")`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-002 (RunCancelled → always 1 hint)

#### T-012 — RunCancelled + side_effects → 2 hints
- **Purpose**: Prove cancellation hint + partial cleanup hint.
- **Input**:
  ```
  failure_code = "RunCancelled"
  side_effects = [serde_json::json!({"step": 2, "action": "write", "certainty": "confirmed"})]
  failed_at_step = Some(2)
  ```
- **Expected output**:
  - `hints.len() == 2`
  - `hints[0]` contains "cancelled"
  - `hints[1]` contains "partial cleanup"
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-002 (RunCancelled → side_effects → 2 hints)

#### T-013 — Unknown failure code → 0 hints
- **Purpose**: Prove `_` arm returns empty vec.
- **Input**: `failure_code = "UnknownError"`, `side_effects = []`, `failed_at_step = None`
- **Expected output**:
  - `hints.is_empty()` — `true`
- **Location**: `commands_incident.rs`, test module
- **Contract clauses**: POST-002

---

## 3. Integration Test Plan

Integration tests live in `crates/vb_cli/tests/incident_integration.rs` (new file).

### T-014 — Failed run → JSON output with failure_code
- **Setup**: Create a temp Fjall database directory. Write a `FjallJournal`, insert events
  simulating a failed run (StepStarted → ActionCompletedEvent → RunFailedEvent).
- **Command**:
  ```
  velvet-ballastics incident <run_id> --db <temp_db_path> --format json
  ```
- **Expected output**:
  - stdout is valid JSON (parseable by `serde_json::from_str`)
  - Parsed JSON contains `"failure_code": "RunFailed"`
  - Parsed JSON contains `"failure_found": true`
  - Exit code is `CliExitCode::Success`
- **Location**: `crates/vb_cli/tests/incident_integration.rs`
- **Contract clauses**: POST-003, INV-003 (JSON validity)

### T-015 — Non-existent run → structured error, no stack trace
- **Setup**: Create a temp Fjall database directory. Open journal, do NOT write events for the
  target run_id (or open a valid journal where the run simply doesn't exist).
- **Command**:
  ```
  velvet-ballastics incident <nonexistent_run_id> --db <temp_db_path> --format json
  ```
- **Expected output**:
  - stdout (or stderr, depending on json_error path) contains valid JSON
  - JSON contains `"success": false`
  - JSON contains an `"error"` field with a plain text message (e.g. "no events found")
  - Output contains NO text matching: `backtrace`, `Stack trace`, `std::backtrace`, `at crates/`, `::{{.*}}::`
  - Exit code is `CliExitCode::StorageError`
- **Location**: `crates/vb_cli/tests/incident_integration.rs`
- **Contract clauses**: POST-003, INV-002 (no stack traces)

### T-016 — Successful run → "not an incident" exit code
- **Setup**: Create a temp Fjall database. Write events simulating a successful run
  (StepStarted → ActionCompletedEvent → StepCompleted). No RunFailedEvent, no RunCancelled.
- **Command**:
  ```
  velvet-ballastics incident <run_id> --db <temp_db_path> --format json
  ```
- **Expected output**:
  - Exit code is `CliExitCode::StorageError` (not success — the run had no failure, so it's "not an incident")
  - Output contains `"success": false` in JSON
  - Error message contains "not an incident"
- **Location**: `crates/vb_cli/tests/incident_integration.rs`
- **Contract clauses**: POST-004

### T-017 — Text output format → deterministic key ordering
- **Setup**: Same as T-014 (failed run with known events).
- **Command**:
  ```
  velvet-ballastics incident <run_id> --db <temp_db_path> --format text
  ```
- **Expected output**:
  - Text output lines appear in deterministic key order:
    1. `incident report for run <run_id>`
    2. `  failure_code:  RunFailed`
    3. `  failed_at_step: <N>`
    4. `  side_effects:`
    5. `    step=... action=... certainty=...`
    6. `  repair_hints:`
    7. `    - <hint text>`
  - No randomization, no hash-map-order-dependent output
- **Location**: `crates/vb_cli/tests/incident_integration.rs`
- **Contract clauses**: INV-004 (text structure)

### T-018 — JSONL output format → single line
- **Setup**: Same as T-014 (failed run with known events).
- **Command**:
  ```
  velvet-ballastics incident <run_id> --db <temp_db_path> --format jsonl
  ```
- **Expected output**:
  - stdout is exactly one line of JSON (no pretty-printing, no newlines within the JSON)
  - Parsed JSON matches the report structure (run_id, failure_code, side_effects, repair_hints)
- **Location**: `crates/vb_cli/tests/incident_integration.rs`
- **Contract clauses**: POST-003, INV-003

---

## 4. Static Scan Plan

These are verified by build system commands, not by test execution.

| Obligation | Command | What it verifies | Expected result |
|---|---|---|---|
| **COMPILE-001** | `cargo check --workspace` | 0 E0061 errors for `recover_full_journal` | Build succeeds |
| **COMPILE-002** | `cargo check --workspace` | 0 E0061 errors for `replay_events` | Build succeeds |
| **UNWRAP-001** | `cargo clippy --workspace --lib --bins -- -D warnings` | No `unwrap_or_default` on `serde_json` output in `cmd_incident`; no `.unwrap()`, `.expect()`, `.unwrap_or()` on fallible ops | 0 warnings |
| **UNWRAP-002** | Contract review (waiver) | `as_str().unwrap_or()` on `serde_json::Value` is safe (Option, not Result) | Waiver approved in contract.md |
| **DEAD-001** | `cargo check --workspace` | No dead_code warnings for `parse_incident` in `args/run_db.rs` | Function removed; 0 dead_code warnings |

---

## 5. Test-to-Contract Traceability

| Contract Clause | Covered By | Evidence |
|---|---|---|
| **PRE-001** (valid run_id) | T-014, T-015, T-016, T-017, T-018 (integration tests use valid run_id) | Integration test setup |
| **PRE-002** (db path accessible) | T-015 (non-existent run in valid db), T-014/016/017/018 (valid temp db) | Integration test setup |
| **PRE-003** (non-null run_id, valid events slice) | T-001 through T-008 (unit tests pass valid args) | Unit test inputs |
| **PRE-004** (valid hints args) | T-009 through T-013 (unit tests pass valid args) | Unit test inputs |
| **POST-001** (IncidentReport structure) | T-001, T-002, T-003, T-004, T-005, T-006, T-007, T-008 | Unit test assertions on report fields |
| **POST-002** (Repair hint taxonomy) | T-009, T-010, T-011, T-012, T-013 | Unit test assertions on hints vec |
| **POST-003** (JSON/JSONL/Text, no stack traces) | T-014, T-015, T-016, T-017, T-018 | Integration test stdout capture + JSON parse + grep for stack trace text |
| **POST-004** (exit code for non-failed run) | T-016 | Integration test exit code assertion |
| **INV-001** (zero-unwrap) | UNWRAP-001 (clippy), T-008 (no panic on unknown), UNWRAP-002 (waiver) | Clippy + unit test |
| **INV-002** (no stack traces) | T-015 (grep for backtrace text), QA-001 (manual inspection) | Integration test + manual QA |
| **INV-003** (JSON validity) | T-014, T-015, T-018 (serde_json::from_str parse) | Integration test |
| **INV-004** (text key ordering) | T-017 (deterministic output) | Integration test |
| **INV-005** (compile correctness) | COMPILE-001, COMPILE-002 | cargo check |
| **INV-006** (dead code removal) | DEAD-001 | cargo check |

**Coverage**: 100% — every contract clause is covered by at least one test or static scan.

---

## 6. Order of Execution

```
Phase 1 — Compile baseline (blocks everything):
  D3: Remove dead code (args/run_db.rs lines 144–151)
  D1: Fix 56 E0061 errors (add &[] to recover_full_journal + replay_events calls)
  → Verify: cargo check --workspace passes

Phase 2 — Zero-unwrap fixes (blocks integration tests):
  D2: Fix unwrap_or_default → match Result in cmd_incident (lines 3181, 3185)
  → Verify: cargo clippy --workspace --lib --bins -- -D warnings passes

Phase 3 — Unit tests (parallel, no I/O dependency):
  D4: Write 8 unit tests for build_incident_report (T-001 through T-008)
  D5: Write 5 unit tests for build_repair_hints (T-009 through T-013)
  → Verify: cargo test --package vb_cli --lib commands_incident::tests passes

Phase 4 — Integration tests (requires D1 + D2):
  D6: Write 5 integration tests (T-014 through T-018)
  → Verify: cargo test --package vb_cli --test incident_integration passes

Phase 5 — Quality gates:
  D7: Static scans (COMPILE-001, COMPILE-002, UNWRAP-001, DEAD-001)
  QA-001: Manual QA inspection (no stack traces in any output path)
  → Verify: moon ci passes
```

---

## 7. Failure Classification

| Classification | When to use | Action |
|---|---|---|
| **BLOCK_LOCAL** | A test fails due to a bug in the function it's testing (e.g., `build_incident_report` returns wrong `failure_code`) | Fix the implementation, re-run the test. Do NOT adjust the test. |
| **FAIL_REGRESSION** | A previously passing test now fails after a code change | Investigate the diff, fix the implementation, re-run. This is a regression. |
| **BLOCK_GLOBAL** | The workspace does not compile (E0061 errors, clippy warnings) | All tests below the compilation gate are non-runnable. Fix compile first. |
| **WAIVED** | A violation is proven safe by contract (e.g., UNWRAP-002: `Option::unwrap_or` is zero-panic) | Document waiver in contract.md; do not fix. |
| **DEFERRED_GLOBAL** | A test requires infrastructure not yet available (e.g., Fjall temp-db fixture not yet scaffolded) | Record in follow-up bead; do not block current work. |
| **MANUAL_FAIL** | Manual QA (QA-001) finds stack trace text in output | Fix the output formatting in `cmd_incident`; re-test. |

**Rule**: Never adjust a test to match broken behavior. If a test fails, the code is wrong
(unless the test itself has a bug — then fix the test).

---

## 8. Summary

| Category | Count | Status |
|---|---|---|
| Unit tests (build_incident_report) | 8 (T-001 to T-008) | Planned |
| Unit tests (build_repair_hints) | 5 (T-009 to T-013) | Planned |
| Integration tests (cmd_incident) | 5 (T-014 to T-018) | Planned |
| Static scan obligations | 4 (+ 1 waiver) | Planned |
| Manual QA | 1 (QA-001) | Planned |
| **Total obligations** | **22** | |

All 22 obligations trace to contract clauses (PRE-001 through POST-004, INV-001 through INV-006).
Test coverage is 100% on contract clauses.
