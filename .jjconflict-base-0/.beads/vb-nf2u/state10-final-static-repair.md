STATUS: PASS

## Repair
- Replaced `xtask/tests/integration_gates.rs:51` banned `assert!(result.is_ok(), ...)` with `assert_eq!(result, Ok(()), ...)` after mapping the cleanup error to a comparable concrete failure string.
- Production code unchanged.

## Command Results

### Static banned assertion scan
Command: `grep/static scan for assert!(result.is_ok())|assert!(result.is_err()) in xtask/tests/integration_gates.rs`

Result: PASS — no matches found.

### xtask nextest
Command: `cargo nextest run -p xtask`

Result: PASS — 91 tests run, 91 passed, 0 skipped.

### Format check
Command: `rtk cargo fmt --all --check`

Result: PASS — command completed successfully with no output.

## Verification
- Report file is non-empty.
- Cleanup behavior remains strict: cleanup failure still fails the test, now with a concrete `Ok(())` assertion instead of a banned hollow success assertion.
