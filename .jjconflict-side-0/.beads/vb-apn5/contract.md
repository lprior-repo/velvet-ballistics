bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 3
updated_at: 2026-05-09T00:00:00Z

# Contract Specification: Single-Server Database Lock Enforcement

## Context
- Feature: Ensure only one process can open a Fjall database at a time
- Domain terms:
  - `ProcessLock`: POSIX flock-based exclusive lock on `.process.lock` file
  - `FjallJournal`: storage engine that acquires ProcessLock on open
  - `JournalError::ProcessLockHeld`: typed error when another process holds the lock
  - `JournalError::ProcessLockIo`: typed error for I/O failures during lock acquisition
  - Doctor: CLI diagnostic command that opens the journal
- Assumptions:
  - Lock file is `.process.lock` inside the database directory
  - Lock is advisory (requires cooperation)
  - Lock auto-releases on process exit or journal Drop
  - PID is written to lock file for diagnostic reporting
- Open questions: NONE

## Preconditions
- P1: Database path exists and is writable, OR can be created
- P2: Lock file can be created/opened in the database directory
- P3: POSIX flock is available on the target platform

## Postconditions
- PO1: First process opening a DB path succeeds and holds the exclusive lock
- PO2: Second process opening the same DB path fails with `JournalError::ProcessLockHeld`
- PO3: Second process receives the holder PID in the error if discoverable
- PO4: After the first process drops the journal, a second process can successfully open
- PO5: Lock acquisition happens BEFORE any Fjall mutation (keyspace creation)
- PO6: Doctor command reports lock failure when DB is held by another process

## Invariants
- I1: At most one process holds the exclusive lock per database path
- I2: `ProcessLockHeld` error always includes the lock file path
- I3: Lock release is automatic (no manual unlock needed)
- I4: No Fjall keyspace is created if lock acquisition fails

## Error Taxonomy
- `JournalError::ProcessLockHeld { path, source, holder_pid }`: another process holds the lock
- `JournalError::ProcessLockIo { path, source }`: I/O error creating/opening lock file
- `RuntimeError::StorageJournalAppend { source }`: runtime wraps storage errors including lock errors

## Contract Signatures
```rust
// Already implemented in vb_storage:
impl FjallJournal {
    pub fn open(path: impl AsRef<Path>, config: Option<FjallConfig>) -> Result<Self, JournalError>;
}

// Already implemented in vb_storage:
impl ProcessLock {
    pub(crate) fn acquire(db_path: &Path) -> Result<Self, JournalError>;
}
```

## Non-goals
- Do not implement distributed locking
- Do not implement lock timeout or retry logic
- Do not change the lock file location or format
- Do not add multi-writer support
