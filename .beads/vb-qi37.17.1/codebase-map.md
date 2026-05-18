# Codebase Map — vb-qi37.17.1 (cli: Add incident command)

## 1. What Exists for the Incident Command

### 1.1 CLI Layer — Pure Logic (`commands_incident.rs`)

**File**: `crates/vb_cli/src/commands_incident.rs` (115 lines)

| Item | Description |
|------|-------------|
| `IncidentReport` struct | Fields: `run_id`, `failure_code`, `failure_found`, `failed_at_step`, `side_effects`, `repair_hints` |
| `build_incident_report()` | Parses `JournalEvent` slice; detects `RunFailedEvent`, `RunCancelled`, `ActionFailedEvent`, `ActionCompletedEvent`; tracks `last_step_started` for failure context |
| `build_repair_hints()` | Generates repair hint strings for `RunFailed` and `RunCancelled` codes |

**Quality**: Clean DDD separation — pure function, no I/O, no unwrap/expect/panic. Zero unit tests.

### 1.2 CLI Layer — Argument Parsing (`args.rs`)

**File**: `crates/vb_cli/src/args.rs`

| Item | Description |
|------|-------------|
| `Command::Incident { run_id, db, output }` | Enum variant with run_id, db path, and output format |
| `parse_incident()` | Delegates to `parse_run_db_args()` (shared with `Diff`/`Inspect`/`Events`/`Replay`) |
| `VALID_COMMANDS` string | Includes `incident` in the help text list |
| Command dispatch (`args.rs:290`) | `"incident" => parse_incident(args)` |

### 1.3 CLI Layer — Command Handler (`app_impl.rs`)

**File**: `crates/vb_cli/src/app_impl.rs` (lines 3108–3230)

| Item | Description |
|------|-------------|
| `cmd_incident()` | Opens `FjallJournal`, calls `journal.events_for_run(rid)`, invokes `build_incident_report()` |
| JSON output (`OutputFormat::Json`) | Pretty-prints `IncidentReport` fields as JSON |
| JSONL output (`OutputFormat::Jsonl`) | Single-line JSON |
| Text output (`OutputFormat::Text`) | Human-readable format |
| Missing run handling | Returns structured error JSON/text: `"no events found for run {run_id}"` |
| Non-failed run handling | Returns `"run {run_id} has no failure event; not an incident"` (exit code 5) |

**Zero-unwrap violations** (must be fixed):
| Line | Violation | Fix |
|------|-----------|-----|
| 3181 | `unwrap_or_default()` on `serde_json::to_string_pretty` | Use `?` or explicit `match` with `json_error()` |
| 3185 | `unwrap_or_default()` on `serde_json::to_string` | Same |
| 3202 | `unwrap_or("unknown")` on `certainty` string | Structured output; field is always a string from `build_incident_report` |
| 3208 | `unwrap_or("unknown")` on hint string | Same — `build_repair_hints` returns `Vec<String>` |

### 1.4 Module Declaration (`main.rs`)

**File**: `crates/vb_cli/src/main.rs:13`
```rust
mod commands_incident;
```

### 1.5 Dead Code — Unreachable `args/run_db.rs`

**File**: `crates/vb_cli/src/args/run_db.rs` (151 lines)

Contains a duplicate `parse_incident()` function that is **never compiled** because `args.rs` does not declare `mod run_db;`. This file is dead code.

### 1.6 UI Model Layer (`vb_ui_model`)

**File**: `crates/vb_ui_model/src/incident.rs` (39 lines)

| Item | Description |
|------|-------------|
| `IncidentReportView` | UI-serializable struct: `run_id`, `failure_step`, `failure_action`, `failure_code`, `attempt`, `timestamp`, `severity`, `safe_to_retry`, etc. |
| `IncidentSeverity` | `Warning` / `Critical` enum |
| `EvidenceChain` | `scheduled_durable`, `completion_durable`, `side_effect_certainty`, `journal_tail` |

**Note**: The CLI `IncidentReport` and the UI model `IncidentReportView` have different schemas. The CLI command does not populate the UI model — they are independent.

### 1.7 UI Screen Layer (`vb_ui`)

**Directory**: `crates/vb_ui/src/incident/` (10 files)

| File | Content |
|------|---------|
| `mod.rs` | Module re-exports |
| `types.rs` | `IncidentType`, `IncidentSeverity`, `ReplaySafety` enums |
| `screen.rs` | `IncidentScreen` — main screen controller |
| `screen/types.rs` | Screen-level types |
| `screen/screen_ui.rs` | UI rendering |
| `screen/colors.rs` | Cyberpunk color constants |
| `screen/tests.rs` | **Extensive tests** (500+ lines): 20+ unit tests for incident screen behavior |
| `repair.rs` | Repair action types |
| `timeline.rs` | Timeline visualization |
| `console.rs` | Console output |

### 1.8 Existing Tests That Mention "incident"

| File | Test | Scope |
|------|------|-------|
| `vb_cli/src/main_tests.rs:71` | `ai_context_failed_run_suggests_incident_command` | Tests that ai-context suggests `incident` CLI command |
| `vb_cli/src/mode_activation_tests.rs:450` | `command_mode_incident_is_storage` | Tests that `incident` command routes to storage mode |
| `vb_cli/tests/vb_qi37_13_structured_reconciliation.rs:8` | Hardcoded expected command list | Only checks `"incident"` is listed as known command |

---

## 2. All Files Touched

### Files that need changes for this bead:

| File | Action | Reason |
|------|--------|--------|
| `crates/vb_cli/src/commands_incident.rs` | Add tests | Zero test coverage |
| `crates/vb_cli/src/app_impl.rs` | Fix 4 unwrap violations | Lines 3181, 3185, 3202, 3208 |
| `crates/vb_cli/src/args/run_db.rs` | Remove or integrate | Dead code (unreachable `mod`) |

### Files that are read-only (known-good, referenced by incident command):

| File | Usage |
|------|-------|
| `crates/vb_storage/src/fjall/mod.rs` (or equivalent) | `FjallJournal::open()` |
| `crates/vb_storage/src/fjall/mod.rs` (or equivalent) | `journal.events_for_run(run_id)` |
| `crates/vb_ui_model/src/incident.rs` | UI model types (not used by CLI) |
| `crates/vb_ui/src/incident/` | UI screen (not wired to CLI) |

---

## 3. Compile Error Analysis

### Status: BUILD IS CLEAN

**Finding**: The compile errors documented in the baseline report (`recover_full_journal` signature mismatch) have already been resolved. All call sites across all crates now use the correct 5-argument form.

### `recover_full_journal` — Current Signature

**File**: `crates/vb_storage/src/recovery/replay/core.rs:127`
```rust
pub fn recover_full_journal(
    journal: &FjallJournal,
    run: RunId,
    tracker: &mut ActionReplayTracker,
    _expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
    expected_policy_digests: &[(StepIdx, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>>
```

### `replay_events` — Current Signature

**File**: `crates/vb_storage/src/recovery/replay/core.rs:34`
```rust
pub fn replay_events(
    events: &[JournalEvent],
    tracker: &mut ActionReplayTracker,
    _expected_action_abi_digests: &[(ActionId, WorkflowDigest)],
) -> RecoveryResult<Vec<JournalEvent>>
```

### All Current Call Sites (5-arg form, verified correct)

| File | Line | Args Count |
|------|------|------------|
| `vb_storage/tests/recovery_bdd_tests.rs` | 226 | 5 ✓ |
| `vb_storage/tests/recovery_bdd_tests.rs` | 489 | 5 ✓ |
| `vb_storage/tests/recovery_bdd_tests.rs` | 569 | 5 ✓ |
| `vb_storage/tests/recovery_bdd_tests.rs` | 636 | 5 ✓ |
| `vb_storage/tests/recovery_bdd_tests.rs` | 652 | 5 ✓ |
| `vb_storage/tests/recovery_bdd_tests.rs` | 722 | 5 ✓ |
| `vb_storage/tests/replay_resume.rs` | 104 | 5 ✓ |
| `vb_storage/tests/replay_resume.rs` | 154 | 5 ✓ |
| `vb_storage/tests/replay_resume.rs` | 166 | 5 ✓ |
| `vb_storage/tests/replay_resume.rs` | 219 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 104 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 167 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 303 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 405 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 476 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 545 | 5 ✓ |
| `vb_storage/tests/vb_h6ix_integration.rs` | 554 | 5 ✓ |
| `vb_storage/tests/recovery_integration.rs` | 635 | 5 ✓ |
| `vb_storage/tests/recovery_integration.rs` | 694 | 5 ✓ |
| `vb_storage/tests/recovery_integration.rs` | 755 | 5 ✓ |
| `vb_storage/tests/recovery_integration.rs` | 787 | 5 ✓ |
| `vb_storage/tests/slot_written_ordering_integration_tests.rs` | 585 | 5 ✓ |
| `vb_storage/tests/slot_written_ordering_integration_tests.rs` | 1042 | 5 ✓ |
| `vb_storage/tests/slot_written_ordering_integration_tests.rs` | 1083 | 5 ✓ |
| `vb_storage/tests/slot_written_ordering_integration_tests.rs` | 1224 | 5 ✓ |
| `vb_storage/src/recovery/tests.rs` | 647 | 5 ✓ |
| `vb_storage/src/recovery/tests.rs` | 989 | 5 ✓ |
| `vb_storage/src/recovery/tests.rs` | 1031 | 5 ✓ |
| `vb_storage/src/recovery/tests.rs` | 1306 | 5 ✓ |
| `vb_storage/src/lib.rs` | 132 | 5 ✓ (re-export) |
| `vb_runtime/src/collect_tests.rs` | 2203 | 5 ✓ |
| `vb_runtime/src/collect_tests.rs` | 2253 | 5 ✓ |
| `vb_runtime/src/collect_tests.rs` | 2316 | 5 ✓ |
| `vb_cli/src/storage.rs` | 244 | 5 ✓ |
| `vb_cli/src/app_impl.rs` | 2577 | 5 ✓ |

**Total call sites: 35 (all correct). Zero compile blockers.**

### Unused Import Warning

**File**: `crates/vb_storage/tests/recovery_bdd_tests.rs:11`
```rust
use vb_core::{ActionId, CapabilitySet, RunId, RuntimePolicy, SlotIdx, SlotValue, StepIdx, WorkflowDigest};
```
- `CapabilitySet` and `RuntimePolicy` are imported but unused.

---

## 4. Test Gaps

### 4.1 Zero Dedicated Tests for Incident Command

The incident command has **zero** test coverage of its core logic:

| What | Where | Status |
|------|-------|--------|
| `build_incident_report` — failed run detection | `commands_incident.rs` | **MISSING** |
| `build_incident_report` — cancelled run detection | `commands_incident.rs` | **MISSING** |
| `build_incident_report` — non-failed run (empty failure) | `commands_incident.rs` | **MISSING** |
| `build_incident_report` — missing run (empty events) | `commands_incident.rs` | **MISSING** |
| `build_incident_report` — side_effects collection | `commands_incident.rs` | **MISSING** |
| `build_incident_report` — repair_hints generation | `commands_incident.rs` | **MISSING** |
| `cmd_incident` — JSON output format | `app_impl.rs` | **MISSING** |
| `cmd_incident` — JSONL output format | `app_impl.rs` | **MISSING** |
| `cmd_incident` — Text output format | `app_impl.rs` | **MISSING** |
| `cmd_incident` — missing run error | `app_impl.rs` | **MISSING** |
| `cmd_incident` — non-failed run error | `app_impl.rs` | **MISSING** |
| No stack traces in output | `app_impl.rs` | **NEEDS VERIFICATION** |

### 4.2 Acceptance Criteria Verification Status

| Criterion | Status | Notes |
|-----------|--------|-------|
| Incident returns structured failure evidence | **DONE** | JSON output contains failure_code, failed_at_step, side_effects, repair_hints |
| No stack traces | **NEEDS VERIFICATION** | No stack traces found in code, but no test proves this |
| Tests for failed runs | **MISSING** | No test covers `RunFailedEvent` processing |
| Tests for missing runs | **MISSING** | No test covers empty events path |
| Tests for non-failed runs | **MISSING** | No test covers `RunFinished` (no failure) path |

---

## 5. Recommendations for Delivery Scope

### 5.1 Priority 1 — Fix Zero-Unwrap Violations (4 fixes)

These are in `app_impl.rs` and are directly in the incident command's output paths:

1. **Line 3181**: Replace `serde_json::to_string_pretty(&json_report).unwrap_or_default()` with proper error handling
2. **Line 3185**: Replace `serde_json::to_string(&json_report).unwrap_or_default()` with proper error handling
3. **Line 3202**: `se["certainty"].as_str().unwrap_or("unknown")` — safe fallback acceptable, but field is guaranteed by `build_incident_report` to always be a string
4. **Line 3208**: `hint.as_str().unwrap_or("unknown")` — same

### 5.2 Priority 2 — Write Unit Tests for `build_incident_report`

Tests should go in `crates/vb_cli/src/commands_incident.rs` as a `#[cfg(test)] mod tests` block:

- `test_build_incident_report_run_failed` — inject events with `RunFailedEvent`, verify failure_code = "RunFailed"
- `test_build_incident_report_run_cancelled` — inject `RunCancelled`, verify failure_code = "RunCancelled"
- `test_build_incident_report_no_failure` — inject only `RunFinished`, verify failure_found = false
- `test_build_incident_report_empty_events` — inject empty slice, verify failure_found = false
- `test_build_incident_report_side_effects` — inject `ActionCompletedEvent` + `ActionFailedEvent`, verify side_effects
- `test_build_incident_report_repair_hints_failed` — verify hints for RunFailed
- `test_build_incident_report_repair_hints_cancelled` — verify hints for RunCancelled
- `test_build_incident_report_failed_at_step` — verify last_step_started tracking

### 5.3 Priority 3 — Integration Test for `cmd_incident`

Add to `crates/vb_cli/tests/`:

- Create a temporary FjallJournal with known events
- Run `velvet-ballastics incident <run_id> --db <path> --json`
- Parse JSON output and verify structure
- Test missing run case
- Test non-failed run case

### 5.4 Priority 4 — Remove Dead Code

- **File**: `crates/vb_cli/src/args/run_db.rs`
- This file is unreachable (no `mod run_db` in `args.rs`)
- Delete or integrate its contents into `args.rs`

### 5.5 Scope Boundary

**In scope**: `vb_cli` crate only — incident command logic, arg parsing, command handler, tests.

**Out of scope**: `vb_storage`, `vb_ui`, `vb_ui_model` — these are read-only references for this bead. The build is clean. The compile errors from the baseline have been resolved by prior work.

### 5.6 Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| serde_json serialization panic (unwrap_or_default) | Medium | Fix with match/json_error |
| Zero test coverage (acceptance criteria) | High | Write unit tests for build_incident_report |
| Dead code in args/run_db.rs | Low | Delete during cleanup |
| Build is clean (no regressions) | Low | Already verified |
