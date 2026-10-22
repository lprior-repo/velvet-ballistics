# Code Review: vb-ydy4w v5

**Candidate:** 917f89d539f96a9f2b5d1f295c2c44d54258b3d8
**Baseline:** 58a1f958
**Scope:** Storage marker only (emergency-deny overlay keyspace)

## Summary

Adds dedicated `emergency_deny` Fjall keyspace for emergency-deny overlay marker storage. Fixed key `active`, value `[1]` (version 1). Missing=Inactive, valid=Active, malformed/len=typed error, wrong version=typed error. Parser uses direct borrowed slice, no to_vec allocation. Activation validates then strict-batch commit with SyncAll ALWAYS. No reset/delete/public raw keyspace/arbitrary writer API. Declaration count updated 10→11 everywhere.

## Changed Files

**Production (2 new):**
- `crates/vb_storage/src/journal/emergency_deny.rs` (90 lines)
- `crates/vb_storage/src/batch/emergency_deny.rs` (16 lines)

**Production modified (9):**
- `crates/vb_storage/src/journal/core.rs` - add keyspace field, open, declared_keyspaces() 10→11
- `crates/vb_storage/src/journal/readonly.rs` - declared_keyspaces() 10→11
- `crates/vb_storage/src/journal/mod.rs` - module imports/exports
- `crates/vb_storage/src/batch/mod.rs` - module import
- `crates/vb_storage/src/batch/putters.rs` - comment cleanup
- `crates/vb_storage/src/constants.rs` - KEYSPACE_EMERGENCY_DENY constant
- `crates/vb_storage/src/error/mod.rs` - two new error variants
- `crates/vb_storage/src/error/codes.rs` - two new error codes + match arms
- `fuzz/fuzz_targets/decode_record.rs` - exhaustive match arms
- `fuzz/fuzz_targets/journal_decode.rs` - exhaustive match arms
- `fuzz/src/journal_target/errors.rs` - exhaustive match arms
- `fuzz/tests/proptest_journal_error_exhaustiveness.rs` - exhaustive match arms

**Tests (1 new, 4 modified):**
- `crates/vb_storage/src/journal/emergency_deny_tests.rs` (549 lines, 13 tests)
- `crates/vb_storage/src/journal/tests.rs` - declared_keyspaces count 10→11
- `crates/vb_storage/src/tests.rs` - declared_keyspaces tests + exhaustive match
- `crates/workspace_tests/tests/fjall_keyspace_manifest_tests.rs` - count 10→11, required names

## Verification

### Contract Requirements
- [x] Fixed key `active`, exact one-byte value `[1]`
- [x] Missing key = Inactive
- [x] Valid `[1]` = Active
- [x] Wrong length = EmergencyDenyMarkerMalformed
- [x] Wrong version = EmergencyDenyMarkerUnsupportedVersion { version }
- [x] Parser uses direct borrowed slice (no to_vec)
- [x] Activation validates then strict-batch commit with SyncAll ALWAYS
- [x] No reset/delete/public raw keyspace/arbitrary writer API
- [x] Idempotent (same constant marker, full equality)
- [x] No sequence counter or overflow

### Norms
- [x] No forbidden Rust (unsafe, unwrap, expect, panic, todo, unimplemented, dbg)
- [x] No unchecked indexing or arithmetic
- [x] Production files ≤300 logical lines (90, 16)
- [x] New functions ≤25 logical lines
- [x] src/*_tests.rs ≤1500 lines (549)
- [x] #![forbid(unsafe_code)] on all new files

### Declaration Consistency
- [x] declared_keyspaces() count 10→11 (core.rs)
- [x] declared_keyspaces() count 10→11 (readonly.rs)
- [x] probe_health() includes emergency_deny keyspace
- [x] Cross-crate test updated (workspace_tests)
- [x] Intra-crate tests updated (journal/tests.rs, tests.rs)

### Error Exhaustiveness
- [x] JournalError::EmergencyDenyMarkerUnsupportedVersion { version } defined
- [x] JournalError::EmergencyDenyMarkerMalformed defined
- [x] Error codes 0x4042, 0x4043 assigned
- [x] diagnostic_code() match arms added
- [x] symbolic_code() match arms added
- [x] Fuzz target exhaustive matches updated (3 targets + 1 proptest)
- [x] Intra-crate exhaustive match updated (tests.rs)

### Durability
- [x] batch.strict() sets PersistMode::SyncAll
- [x] Activation always performs strict-batch commit even if already Active
- [x] Validation rejects corrupt/unsupported state without overwriting

### Test Coverage (13 tests)
- [x] new_store_is_inactive
- [x] activate_returns_active_state
- [x] repeated_activation_is_idempotent
- [x] marker_persists_across_journal_close
- [x] separate_stores_are_isolated
- [x] malformed_marker_wrong_length_errors (5-byte)
- [x] unsupported_version_errors (version 99)
- [x] malformed_empty_marker_errors (0-byte)
- [x] malformed_trailing_bytes_errors (2-byte)
- [x] precommit_batch_failure_returns_error_not_success
- [x] retry_after_injected_failure
- [x] already_active_retry_performs_strict_persistence
- [x] activation_does_not_corrupt_run_events

## Findings

None.

## Status

APPROVED