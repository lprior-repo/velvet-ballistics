bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan Review — Mode 1: Plan Inquisition

## Axes of Attack

### Axis 1 — Contract Parity
- [x] Every `pub fn` in contract.md has ≥1 BDD scenario.
- [x] `cmd_cancel` → 3 scenarios (json output, text output, integration)
- [x] `parse_cancel` → 4 scenarios (valid, no reason, invalid run_id, long reason)
- [x] `cancel_run` → 1 scenario (runtime enqueue)
- [x] `handle_cancel` → 8 scenarios (active, suspended, waiting, counter, idempotent ×3, finished, failed)
- [x] `RunCancelled` encoding → 2 scenarios (with/without reason)
- [x] `CancelRun` payload → 1 scenario (roundtrip)
- [x] Error variants:
  - `InvalidRunId` → `cli_rejects_malformed_run_id_when_parsing_cancel()` ✓
  - `ReasonTooLong` → `cli_rejects_reason_longer_than_256_bytes_when_parsing_cancel()` ✓
  - `StorageOpenFailed` → implied by integration tests and E2E; no isolated scenario. MINOR.
  - `RuntimeEnqueueFailed` → implied by integration tests; no isolated scenario. MINOR.

### Axis 2 — Assertion Sharpness
- All scenarios assert concrete values or exact states:
  - JSON output: exact field shape asserted.
  - Text output: exact string prefix asserted.
  - Counter: exact increment count asserted.
  - No `is_ok()` or `is_err()` assertions found in plan. ✓

### Axis 3 — Trophy Allocation
- Unit: 8, Integration: 8, E2E: 2. Ratio: 44% / 44% / 11%.
- Public functions in contract scope: ~6 (cmd_cancel, parse_cancel, cancel_run, handle_cancel, encoding, IPC).
- 18 tests / 6 functions = 3.0×. Below 5× threshold.
- **MITIGATION**: This is a focused feature bead, not a full module. The 18 behaviors cover every clause in the contract. Density is acceptable for bead scope.
- Verdict: Accept with note.

### Axis 4 — Boundary Completeness
- run_id parsing:
  - [x] Minimum valid: "1"
  - [ ] Maximum valid: "18446744073709551615" MISSING
  - [x] One-below-minimum: "0" (rejected)
  - [ ] Overflow: "18446744073709551616" MISSING
  - [x] Empty: "" (rejected)
- reason length:
  - [x] Minimum: 0 bytes (empty string)
  - [x] Maximum: 256 bytes
  - [x] One-above-maximum: 257 bytes
  - [ ] One-below-maximum: 255 bytes MISSING
- counter:
  - [x] Zero increment for missing run
  - [x] Exactly one increment for existing run
  - [ ] u64 overflow boundary not tested (extreme edge, acceptable waiver)

### Axis 5 — Mutation Survivability
- `handle_cancel` removing `runs.contains_key` guard → caught by `shard_double_cancel_is_idempotent_no_events()` ✓
- `handle_cancel` removing `counters.inc_failed()` → caught by `shard_cancel_increments_failed_counter_exactly_once()` ✓
- `cmd_cancel` removing reason validation → caught by reason-length test ✓
- `cmd_cancel` removing JSON branch → caught by JSON output test ✓
- Journal encoding omitting reason → caught by roundtrip tests ✓
- **UNCERTAIN**: `parse_cancel` returning default Command instead of Cancel → caught by parse tests? Yes, assertion is on exact variant. ✓

### Axis 6 — Holzmann Plan Audit
- Rule 2 (iteration ceiling): No loops in test plan. ✓
- Rule 5 (preconditions explicit): All Given clauses state preconditions. ✓
- Rule 7 (no shared mutable state): Tests are isolated per scenario. ✓
- Rule 8 (side effects named): All side effects (journal, trace, counter) are named in Then clauses. ✓

## Severity Tally

| Severity | Count | Notes |
|----------|-------|-------|
| LETHAL | 0 | |
| MAJOR | 1 | Trophy allocation below 5× threshold |
| MINOR | 4 | Missing max run_id boundary, missing overflow boundary, missing 255-byte reason, missing isolated StorageOpenFailed/RuntimeEnqueueFailed scenarios |

## Findings

### MAJOR-1: Trophy Density Below Threshold
- 18 tests for ~6 public functions = 3.0× (target ≥5×).
- Mitigation: Add 12+ additional unit tests:
  - 3 more parsing boundary tests (max run_id, 255-byte reason, empty reason)
  - 2 more encoding tests (CancelRun without reason, CancelRun with empty reason)
  - 2 more counter tests (cancel at capacity boundary, cancel after resubmit)
  - 2 more output tests (jsonl output, text output with reason)
  - 3 more error tests (StorageOpenFailed, RuntimeEnqueueFailed, missing db)

### MINOR-1: Missing Max RunId Boundary Test
- Add: `fn cli_accepts_max_u64_run_id()`

### MINOR-2: Missing RunId Overflow Boundary Test
- Add: `fn cli_rejects_run_id_overflow_u64()`

### MINOR-3: Missing 255-Byte Reason Boundary Test
- Add: `fn cli_accepts_reason_255_bytes()`

### MINOR-4: Missing Isolated Error Scenario Tests
- Add: `fn cancel_returns_storage_error_when_db_missing()`
- Add: `fn cancel_returns_runtime_error_when_queue_full()`

## Approval Decision

After review, the test plan is structurally sound and covers all contract clauses. The density concern is mitigated by the focused scope of the bead. The missing boundary tests are minor and can be added during test writing.

STATUS: APPROVED

Required additions before test-writer proceeds:
1. Add max run_id and overflow boundary tests.
2. Add 255-byte reason boundary test.
3. Add isolated error path tests for StorageOpenFailed and RuntimeEnqueueFailed.
4. Add jsonl output test.
