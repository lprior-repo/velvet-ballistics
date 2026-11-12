bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 4
updated_at: 2026-05-09T00:00:00Z

# Test Plan: Single-Server Database Lock Enforcement

## Summary
- Behaviors identified: 8
- Trophy allocation: 6 unit / 2 integration / 0 e2e
- Proptest: 0 (filesystem state is not deterministic enough for proptest)
- Fuzz: 0 (no parsing boundaries)
- Kani: 0 (filesystem I/O)

## 1. Behavior Inventory

1. `[FjallJournal::open] [succeeds] [when database path is available]`
2. `[FjallJournal::open] [fails with ProcessLockHeld] [when another process holds the lock]`
3. `[ProcessLock] [includes holder PID] [when lock is held by another process]`
4. `[ProcessLock] [releases automatically] [when journal is dropped]`
5. `[FjallJournal::open] [creates no keyspaces] [when lock acquisition fails]`
6. `[Doctor command] [reports lock failure] [when database is held by another process]`
7. `[Runtime startup] [propagates lock error] [when database is held]`
8. `[ProcessLockHeld error] [includes lock file path] [always]`

## 2. Trophy Allocation

| Layer | Count | Justification |
|-------|-------|---------------|
| Unit | 6 | Direct filesystem behavior testing |
| Integration | 2 | CLI doctor + runtime startup paths |
| E2E | 0 | No external API change |

## 3. BDD Scenarios

### Behavior 1: First open succeeds
```
Given: A writable temporary directory
When: FjallJournal::open is called
Then: Returns Ok(journal) and creates .process.lock file
```
Test: `test_first_open_succeeds_and_creates_lock_file`

### Behavior 2: Second open fails with exact error
```
Given: A journal already open on a path
When: FjallJournal::open is called on the same path
Then: Returns Err(ProcessLockHeld { .. }) with holder PID
```
Test: `test_second_open_returns_exact_process_lock_held_error`

### Behavior 3: Lock releases on drop
```
Given: A journal that was opened and then dropped
When: FjallJournal::open is called on the same path
Then: Returns Ok(journal)
```
Test: `test_lock_releases_on_journal_drop`

### Behavior 4: No keyspaces on lock failure
```
Given: A journal already open on a path
When: Second open fails
Then: No additional Fjall files are created in the directory
```
Test: `test_no_keyspace_created_when_lock_fails`

### Behavior 5: Doctor reports locked database
```
Given: A journal already open on a path
When: doctor command is run on the same path
Then: Returns FAILURE exit code with lock error message
```
Test: `test_doctor_reports_locked_database`

### Behavior 6: Runtime propagates lock error
```
Given: A journal already open on a path
When: runtime attempts to open the same path
Then: Returns StorageJournalAppend error wrapping ProcessLockHeld
```
Test: `test_runtime_propagates_process_lock_held`

## 4. Mutation Checkpoints

| Mutation | Catching Test |
|---|---|
| Remove `ProcessLock::acquire` call from `FjallJournal::open` | `test_second_open_returns_exact_process_lock_held_error` |
| Move lock acquisition after keyspace creation | `test_no_keyspace_created_when_lock_fails` |
| Remove PID write to lock file | `test_process_lock_held_includes_holder_pid` |
| Change flock to shared lock | `test_second_open_returns_exact_process_lock_held_error` |

## Open Questions
None.
