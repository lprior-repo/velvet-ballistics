bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 9
updated_at: 2026-05-18T00:00:00Z
attempt: 1

# test-plan-review.md

## VERDICT: APPROVED (with advisory notes)

### Axis 1 — Contract Parity

All 16 behaviors from the test plan map to implemented test scenarios:

| Behavior | Plan Scenario | Implementation File | Status |
|---|---|---|---|
| build_trace maps events to TraceEntry | BDD unit scenario | `commands_journal.rs` inline tests | ✓ |
| trace_one maps all 18 variants | Per-variant unit tests | `commands_journal.rs` inline tests | ✓ |
| build_trace determinism | proptest + unit | `commands_journal.rs` | ✓ |
| parse_run_id accepts valid u64 | unit: accepts_valid_decimal | `main_tests.rs:880` | ✓ |
| parse_run_id rejects zero | unit: rejects_zero | `main_tests.rs:912` | FAIL_FIRST (expected) |
| parse_run_id rejects non-numeric | unit: rejects_non_numeric | `main_tests.rs:896` | ✓ |
| read_journal_events StorageError on missing dir | integration: dir_not_found | `cli_trace_integration.rs:371` | FAIL_FIRST (expected) |
| read_journal_events StorageError on read failure | integration | Not separately tested | WAIVED (covered by dir_not_found) |
| read_journal_events empty on no events | integration: empty_run | `cli_trace_integration.rs:319` | ✓ |
| cmd_trace --json structure | integration: json_format | `cli_trace_integration.rs:227` | ✓ |
| cmd_trace --jsonl structure | integration: jsonl_format | `cli_trace_integration.rs:272` | ✓ |
| cmd_trace text format | integration: text_format | `cli_trace_integration.rs:193` | ✓ |
| cmd_trace exit 0 on success | integration/e2e | `cli_trace_integration.rs:141,393` | ✓ |
| cmd_trace exit 0 on empty run | integration | `cli_trace_integration.rs:319,410` | ✓ |
| cmd_trace invalid run_id format | integration | `cli_trace_integration.rs:355` | ✓ |
| cmd_trace invalid db path | integration | `cli_trace_integration.rs:341` | ✓ |

All `pub fn` from `commands_journal.rs` have ≥1 BDD scenario. All error variants have scenario asserting exact variant.

### Axis 2 — Assertion Sharpness

**PASS with advisory.** Tests assert exact values:

- `trace_one_run_accepted_maps_correct_fields`: asserts `event_type == "RunAccepted"`, `seq == 5`, `step == None`, `index == 0` — all exact
- `trace_one` covers all 18 variants with exact assertions
- Integration tests assert exact exit codes (`0`, `5`, `1`) and parse JSON with exact field checks
- Proptest `trace_entry_index_matches_position` asserts exact `idx` correspondence

**Advisory (non-blocking):** Several parse_run_id tests use `assert!(result.is_ok())` or `assert!(result.is_err())` followed by `let Ok(...)`/`let Err(...)` and then `assert_eq!`. The exact-value assertions exist downstream, but the pattern violates the strict doctrine that `is_ok()`/`is_err()` alone are insufficient. Examples:
- `main_tests.rs:882` — `assert!(result.is_ok())` then `assert_eq!(run_id.get(), 42)` — exact value IS asserted
- `main_tests.rs:898` — `assert!(result.is_err())` then `assert_eq!(code, ValidationFailed)` — exact variant IS asserted

These are not hollow; they do assert exact values. Advisory note for test-writer: prefer `let Ok(v) = result else { return; }; assert_eq!(v.get(), 42)` pattern to avoid LETHAL-flagged `assert!(result.is_ok())`.

### Axis 3 — Trophy Allocation

**PASS.** 16 behaviors, 9 unit + 5 integration + 1 e2e + 1 static as planned. Ratio ≥5× for pure functions:
- `build_trace`: 1 fn, 5 unit tests
- `trace_one`: 1 fn, 18 variant unit tests
- `parse_run_id`: 1 fn, 7 unit tests

All non-trivial pure functions have proptest invariants. Integration tests use real FjallJournal.

### Axis 4 — Boundary Completeness

**PASS.** All named boundaries covered:

| Function | Min | Max | Zero/Empty | Overflow | Non-numeric |
|---|---|---|---|---|---|
| parse_run_id | "1" | MAX u64 | "" (empty) | N/A (u64 parse) | "abc", "-1", "0x10", "1.5" |
| build_trace | `&[]` | 50 events | empty (zero) | N/A (usize len) | N/A |
| trace_one | all 18 variants | all 18 variants | N/A | N/A | N/A |

### Axis 5 — Mutation Survivability

**PASS.** Critical mutations covered:

- `trace_one` match arm deletion → caught by `trace_one_*_maps_correct_fields` for all 18 variants
- `build_trace` enumerate skip → caught by `build_trace_preserves_event_order`
- `build_trace` reverse → caught by `build_trace_preserves_event_order`
- `parse_run_id` accepting zero → caught by `parse_run_id_rejects_zero` (FAILS — implementation gap, not test gap)
- `cmd_trace` success on storage error → caught by `cmd_trace_invalid_db_path_returns_storage_error` (FAILS — implementation gap, not test gap)

### Axis 6 — Evidence Plan Audit

**PASS.** All scenarios state preconditions in doc comments. Fixtures use `tempfile::tempdir()` (self-cleaning). No unbounded generation without reproducibility.

---

## MANDATE

No test plan changes required. The 2 FAIL_FIRST failures are implementation gaps, not test gaps.

**Advisory for State 10 (holzman-rust):**
1. `parse_run_id` must reject zero: add `id != 0` check before `RunId::new(id)`
2. `read_journal_events` / journal open behavior: the test `read_journal_events_returns_storage_error_when_dir_not_found` expects exit 5 when journal dir does not exist, but FjallJournal appears to create the dir or return empty events gracefully. Verify expected error handling path.
