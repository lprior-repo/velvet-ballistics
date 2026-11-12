bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 11
updated_at: 2026-05-18T00:00:00Z

## Regression Diff

### State 10 Changes

Two implementation gaps fixed in `crates/vb_cli/src/app_impl.rs`:

1. **parse_run_id zero rejection**: Added `id == 0` check → now returns `ValidationFailed` for "0"
2. **read_journal_events dir check**: Added `db.exists()` guard before `FjallJournal::open` → returns `StorageError` when dir absent

### Classification

| Failure | Class | Notes |
|---------|-------|-------|
| parse_run_id_rejects_zero (was FAIL_FIRST) | BLOCK_LOCAL → FIXED | Now PASS |
| read_journal_events_returns_storage_error (was FAIL_FIRST) | BLOCK_LOCAL → FIXED | Now PASS |

### Baseline Comparison

- Baseline: 562 passed, 2 FAIL_FIRST
- After fix: 564 passed, 0 FAIL_FIRST
- Delta: +2 tests now passing, 0 regressions
