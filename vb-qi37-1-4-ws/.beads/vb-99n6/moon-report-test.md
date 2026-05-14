# Moon :test Gate Report

**Bead:** vb-99n6
**Command:** `moon run :test`
**Exit Code:** 101
**Status:** FAILED

## Summary

The Moon :test gate failed during the `velvet-ballastics:check` task.

## Errors

### 1. vb_storage Syntax Error
**File:** `crates/vb_storage/src/batch.rs:252`
```
error: unexpected closing delimiter: `}`
```
The `JournalWriteBatch` implementation block has mismatched braces.

### 2. xtask Missing Dependencies
**File:** `xtask/src/evidence.rs`
```
error[E0433]: cannot find module or crate `serde_yaml` in this scope
```
The `serde_yaml` crate is used but not declared in xtask's Cargo.toml.

### 3. xtask Missing Functions
**File:** `xtask/src/main.rs:108-110`
```
error[E0425]: cannot find function `cmd_ai_fast` in this scope
error[E0425]: cannot find function `cmd_ai_deep` in this scope
error[E0425]: cannot find function `cmd_ai_release` in this scope
```

### 4. xtask Evidence Error Variant Mismatch
**File:** `xtask/src/evidence.rs:649`
```
error[E0026]: variant `evidence::Error::GateTimeout` does not have a field named `gate_name`
error[E0027]: pattern does not mention field `gate`
```

## Warnings (Non-fatal)

- Multiple unused imports across vb_core and vb_validate test files
- Duplicate crate entries in lock file (hashbrown, winnow, wit-bindgen)
- Unmaintained crates advisory (atomic-polyfill, fxhash)

## Task Timeline

1. `nightly-feature-gate` - OK (cached)
2. `check` - FAILED (exit code 101)
3. `supply-chain` - OK (completed despite check failure)
