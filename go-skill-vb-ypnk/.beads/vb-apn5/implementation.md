bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 6
updated_at: 2026-05-09T00:00:00Z

# Implementation Report

## Summary
The process lock mechanism ALREADY EXISTED in the codebase. This bead focused on strengthening test coverage and verifying the lock enforcement contract.

## Existing Infrastructure Verified
- `vb_storage/src/process_lock.rs`: POSIX flock-based exclusive lock
- `vb_storage/src/journal.rs`: `FjallJournal::open()` calls `ProcessLock::acquire()` BEFORE creating keyspaces
- `vb_storage/src/error.rs`: `JournalError::ProcessLockHeld` and `ProcessLockIo` variants

## Tests Added

### vb_storage/src/tests.rs
1. `test_first_open_succeeds_and_creates_lock_file`: verifies `.process.lock` is created
2. `test_lock_releases_on_journal_drop`: verifies lock releases on Drop, allowing re-open
3. `test_second_open_fails_in_same_process`: verifies same-process second open fails (Fjall detects)
4. `test_lock_file_contains_holder_pid`: verifies lock file contains current process PID
5. `test_no_keyspace_created_when_lock_fails`: verifies no mutation on lock failure

### vb_storage/src/security_tests.rs
1. `process_lock_file_created_with_holder_pid`: security test for PID in lock file
2. `lock_releases_on_journal_drop`: security test for lock release
3. `no_keyspace_created_when_lock_fails`: security test for no mutation on failure

## Key Finding
- Same-process second open fails with `FjallError::Locked` (Fjall's own detection)
- Cross-process second open would fail with `ProcessLockHeld` (POSIX flock)
- Both layers provide defense-in-depth

## Files Modified
- `crates/vb_storage/src/tests.rs` - added 5 tests
- `crates/vb_storage/src/security_tests.rs` - strengthened 1 test, added 3 tests

## Files NOT Modified
- No changes to `process_lock.rs` - already correct
- No changes to `journal.rs` - already acquires lock before keyspaces
- No changes to error types - already exist
- No changes to runtime/CLI - already propagate errors
