bead_id: vb-qi37.16.1
bead_title: cli/runtime: Implement durable cancel transition
phase: 5
updated_at: 2026-05-09T00:00:00Z

## State 5: TDD Red Phase — COMPLETE

### Tests Added

#### 1. CLI Parsing Tests (velvet_ballastics/src/args.rs) — ALL PASS
- `parse_cancel_accepts_run_id_and_db`
- `parse_cancel_accepts_reason`
- `parse_cancel_accepts_json_output`
- `parse_cancel_rejects_missing_db`
- `parse_cancel_rejects_reason_longer_than_256_bytes`
- `parse_cancel_accepts_reason_exactly_256_bytes`

#### 2. CLI Integration Tests (velvet_ballastics/tests/cli_integration.rs) — ALL FAIL (RED)
- `cli_cancel_nonexistent_run_returns_success_idempotent` → Fails: `cancel command not yet implemented`
- `cli_cancel_with_reason_persists_to_journal` → Fails: stub returns error
- `cli_cancel_json_output_contains_success_and_status` → Fails: JSON `{"success":false,"error":"cancel command not yet implemented"}`

#### 3. Shard Cancel-with-Reason Tests (vb_runtime/src/shard/tests.rs) — ALL PASS
- `shard_cancel_with_reason_persists_reason_to_journal`
- `shard_cancel_without_reason_persists_none_to_journal`

#### 4. Storage Codec Test (vb_storage/src/codec.rs) — PASS
- `encode_decode_roundtrip_journal_event_run_cancelled_with_reason`

### Red Phase Evidence
```
---- cli_cancel_nonexistent_run_returns_success_idempotent stdout ----
cancel nonexistent run should succeed idempotently failed: stderr=cancel command not yet implemented

---- cli_cancel_json_output_contains_success_and_status stdout ----
cancel with --json should succeed failed: stdout={"error":"cancel command not yet implemented","success":false}
```

### Pre-existing Issues Note
The workspace has pre-existing test compilation errors in:
- `vb_storage/src/tests.rs` — Missing `attempt` field in JournalEvent constructors (242 errors)
- `vb_runtime/tests/vb_jggy_property_tests.rs` — Missing imports and macros
- `velvet_ballastics/src/mode_activation_tests.rs` — prop_assert format string issues

These were introduced by the parent commit and are outside the scope of this bead. The production code compiles cleanly.

### Retry Budget: 7/7

## Next: State 6
Implement `cmd_cancel` to make red tests green.
