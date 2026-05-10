bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 2
updated_at: 2026-05-09T00:00:00Z

# Codebase Map: Single-Server Database Lock Enforcement

## Existing Infrastructure (Already Implemented)

### vb_storage/src/process_lock.rs (104 lines)
- `ProcessLock` struct: `pub(crate)`, uses POSIX `flock` via `rustix::fs::flock`
- `ProcessLock::acquire(db_path: &Path) -> Result<Self, JournalError>`
  - Creates `.process.lock` file in DB directory
  - Non-blocking exclusive flock
  - On contention: returns `JournalError::ProcessLockHeld { path, source, holder_pid }`
  - On I/O error: returns `JournalError::ProcessLockIo { path, source }`
  - Writes holder PID to lock file for diagnostics
  - Lock auto-released on Drop (file descriptor close)
- `read_holder_pid(file: &File) -> Option<u32>`: best-effort PID read

### vb_storage/src/error.rs (327 lines)
- `JournalError::ProcessLockHeld { path, source, holder_pid }` (line 219)
- `JournalError::ProcessLockIo { path, source }` (line 229)
- Diagnostic codes: `PROCESS_LOCK_HELD_CODE = 0x401A`, `PROCESS_LOCK_IO_CODE = 0x401B`

### vb_storage/src/journal.rs (2397 lines)
- `FjallJournal` struct includes `_process_lock: ProcessLock` field (line 58)
- `FjallJournal::open` calls `ProcessLock::acquire(path_ref)?` at line 97
- Lock is acquired BEFORE creating Fjall keyspaces

### vb_storage/src/lib.rs (170 lines)
- `pub mod process_lock` (line 33) - module is public
- `pub use error::JournalError` (line 49)

### vb_storage/src/security_tests.rs (951 lines)
- Existing test: `second_journal_open_on_same_path_is_prevented_by_process_lock` (line 897)
  - Only asserts `result.is_err()` - does NOT verify exact error type

### vb_runtime/src/lib.rs (947 lines)
- `RuntimeError::StorageJournalAppend { source: Arc<vb_storage::JournalError> }` (line 92)
- `impl From<vb_storage::JournalError> for RuntimeError` (line 462)
- No dedicated `RuntimeError` variant for process lock held

### CLI: velvet_ballastics/src/bench.rs (131 lines)
- `cmd_doctor(db: &Path) -> ExitCode` (line 67)
  - Opens journal via `vb_storage::FjallJournal::open(db, None)`
  - On error: prints "FAIL: cannot open journal" and returns `ExitCode::FAILURE`
  - Already handles `ProcessLockHeld` via generic error path

## What's Missing / Needs Work

1. **Exact error assertion in existing test**: Should assert `matches!(result, Err(JournalError::ProcessLockHeld { .. }))`
2. **Lock release test**: Drop journal, verify second open succeeds
3. **Doctor lock report test**: Test doctor command when DB is locked
4. **Runtime lock error test**: Test runtime startup when DB is locked
5. **No accidental multi-writer mode test**: Ensure FjallJournal::open always acquires lock
6. **Potential**: Expose `ProcessLock` or lock-check function for doctor pre-check

## Key Files for Changes
- `crates/vb_storage/src/security_tests.rs` - strengthen/add tests
- `crates/vb_storage/src/process_lock.rs` - may need public API
- `crates/vb_storage/src/tests.rs` - add lock tests
- `crates/vb_runtime/src/` - add runtime startup lock tests
- `crates/velvet_ballastics/src/` - add doctor lock tests

## Integration Points
- Storage → Runtime: `JournalError` propagates via `From` impl
- Runtime → CLI: `RuntimeError` display includes storage source
- CLI doctor: Generic error handling already covers lock errors
