bead_id: vb-qi37.15.3
bead_title: cli: Add trace command
phase: 10
updated_at: 2026-05-18T00:00:00Z
attempt: 1

## Implementation Fixes

### Gap 1: `parse_run_id` must reject zero

**File:** `crates/vb_cli/src/app_impl.rs` (line 209)

**Problem:** `parse_run_id` accepted any u64 including 0. The contract requires rejection of "0".

**Fix:** Added zero-check after parsing u64:
```rust
Ok(id) => {
    if id == 0 {
        write_failure_message(
            &format!("invalid run_id '{raw}': run_id must be non-zero"),
            output,
            CliExitCode::ValidationFailed,
        );
        return Err(CliExitCode::ValidationFailed.into());
    }
    Ok(vb_core::RunId::new(id))
}
```

**Verification:** `cargo test -p vb_cli parse_run_id_rejects_zero` → PASS (1 passed)

### Gap 2: `read_journal_events` must return StorageError when dir not found

**File:** `crates/vb_cli/src/app_impl.rs` (line 236)

**Problem:** `FjallJournal::open` succeeds on a nonexistent directory (F jall creates it), so the function returned exit code 0 with empty events instead of StorageError.

**Fix:** Added directory existence check before `FjallJournal::open`:
```rust
if !db.exists() {
    let msg = format!("journal directory does not exist: {}", db.display());
    if output != OutputFormat::Text {
        write_failure_message(&msg, output, CliExitCode::StorageError);
    } else {
        errln!("{msg}");
    }
    return Err(CliExitCode::StorageError.into());
}
```

**Verification:** `cargo test -p vb_cli read_journal_events_returns_storage_error` → PASS (1 passed)

## Full Test Results
- `cargo test -p vb_cli`: 564 passed, 1 ignored (16 suites)
- No banned patterns, no regressions introduced
