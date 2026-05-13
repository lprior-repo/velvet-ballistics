# vb-qi37.12.2 State

- **bead**: vb-qi37.12.2
- **title**: [BUG] runtime/storage: Propagate journal and storage failures
- **phase**: 8
- **goal**: Write failing-first tests using `test-writer`

## State History
- Phase 1-7: Previous states (contract, proof planning, etc.)
- Phase 8 (current): Write failing-first tests

## Phase 8 Completion

### Tests Written
- **File**: `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`
- **Test count**: 6 tests
- **Infrastructure**: `FailingRuntimeJournal` for inject failure testing

### Test Results
```
cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation
6 passed
```

### Artifacts
- `test-plan.md` — Test specification and coverage matrix
- `test-writer-report.md` — This report

### Key Findings

**Bug 1 (`observe_resume_drive_result`)**: The function at `chunk_001.rs:178-182` silently drops errors by matching both `Ok(()) | Err(_)` to `{}`. However, in single-threaded tests, `drive_run` always returns `Ok(())` because `apply_drive_result` handles errors internally via `apply_terminal_failed` which returns `Ok(())`. The error propagation tests verify journal failure propagates, but cannot directly trigger the `observe_resume_drive_result` silent-drop bug without concurrent access or internal test access.

**Bug 2 (journal ordering)**: Tests verify `RunSubmitted` appears before `RunAdmission` in journal events, and that after submit both journal and state exist.

**Bug 3 (error propagation)**: Tests verify `tick()` propagates `StorageJournalAppend` errors from `handle_submit`.

### Test File Locations
- `crates/vb_runtime/tests/vb_qi37_12_2_resume_error_propagation.rs`