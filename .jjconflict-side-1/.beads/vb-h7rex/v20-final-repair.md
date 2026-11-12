# V20 Final Repair — Handoff

## Changes Made

### 1. Fixed forbidden `.expect()` calls in policy pin legacy layout test
**File:** `crates/vb_storage/tests/policy_pin_legacy_layout.rs`

- `encode_envelope`: Changed return type from `Vec<u8>` to `Result<Vec<u8>, vb_storage::JournalError>`. Returns the `encode_record` result directly without `.expect("encode failed")`.
- `check_historic_layout`: Changed return type from `()` to `Result<(), vb_storage::JournalError>`. Uses `?` on both `encode_envelope` calls and the `decode_journal_event` call. Ends with `Ok(())`.
- `tag_24_unbound_preserves_historical_layout`: Changed return type to `Result<(), vb_storage::JournalError>`. Returns `check_historic_layout` result directly (no trailing semicolon).
- `tag_36_bound_preserves_historical_layout`: Changed return type to `Result<(), vb_storage::JournalError>`. Returns `check_historic_layout` result directly (no trailing semicolon).

Leading `24_u8`/`36_u8` historical tuple fixtures preserved. All three `assert_eq` assertions kept.

### 2. Updated keyspace count test after emergency_deny addition
**File:** `crates/workspace_tests/tests/restate_doctor_key_decode_tests.rs`

- Renamed `readonly_journal_declared_keyspaces_returns_ten` → `readonly_journal_declared_keyspaces_returns_eleven`
- Updated assertion from `assert_eq!(spaces.len(), 10)` to `assert_eq!(spaces.len(), 11)`
- Added inline comment explaining the count reflects the post-vb-h7rex addition of the `emergency_deny` keyspace

All other assertions in that test preserved.

## Other Keyspace Count Tests Already Correct
- `crates/vb_storage/src/tests.rs` → `declared_keyspaces_returns_eleven_entries` (already updated)
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` → `declared_keyspaces_count` (expects 11, already updated)
- `crates/vb_storage/src/journal/readonly.rs` → `declared_keyspaces()` returns 11 (production)

## Verification
No verification commands run. Changes are source-only edits matching the repair brief.