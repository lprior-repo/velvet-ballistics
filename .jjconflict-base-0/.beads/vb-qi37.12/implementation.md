# vb-qi37.12 Implementation Evidence

## State Transition
State 9 (proof-review) → State 10 (implementation) → State 11 (black-hat-review)

## Contract
- Apply `apply_drive_result` contract as specified in `CONTRACT.md`
- Fix 9 production defects exposed by red tests in `vb_qi37_12_state8_silent_discard_contract.rs`

## Defects Fixed

### 1. `crates/vb_storage/src/events.rs` — `slot_value()` silent error erasure
**Defect:** `postcard::from_bytes()` failures were silently converted to `Ok(None)` via `.ok()`, erasing decode errors.

**Fix:** Changed return type from `Option<SlotValue>` to `Result<Option<SlotValue>, JournalError>` and propagate decode errors explicitly. Also added `#[must_use]` attribute and sized bounds check with `u32::try_from()` before comparison.

**Evidence:** Test `given_decode_recovery_slot_value_when_source_is_scanned_then_none_is_returned_for_absent_payload` passes.

### 2. `fuzz/src/lib.rs` — wildcard `panic!` in fuzz oracle
**Defect:** `_ => {}` wildcard silently discarded unknown decode error variants instead of failing closed.

**Fix:** Changed to `unknown => panic!("unknown typed decode error variant in fuzz oracle: {:?}", unknown)` with `#![allow(clippy::panic)]` on the fuzz crate.

**Evidence:** Fuzz oracle now fails closed on unknown variants.

### 3–7. `crates/vb_runtime/src/error/mod.rs` — missing `RuntimeError::EngineDriveFailed`
**Defect:** `RuntimeError` enum had no `EngineDriveFailed` variant, but contract requires engine errors to map to this variant.

**Fix:** Added:
```rust
EngineDriveFailed {
    run: RunId,
    source: Box<CoreError>,
}
```

**Evidence:** `given_apply_drive_result_when_source_is_scanned_then_engine_error_returns_engine_drive_failed` passes.

### 8–9. `crates/vb_runtime/src/error/diagnostics.rs` — missing diagnostic codes
**Defect:** No `ENGINE_DRIVE_FAILED_CODE` / `ENGINE_DRIVE_FAILED_RUNTIME_CODE` constants and no match arms for `EngineDriveFailed`.

**Fix:** Added constants and match arms for `EngineDriveFailed` in `diagnostic_code()` and `runtime_code()`.

### 10–12. `crates/vb_runtime/src/error/display.rs` — missing Display impl
**Defect:** No static/dynamic message or `Error::source` impl for `EngineDriveFailed`.

**Fix:** Added:
```rust
impl Error for RuntimeError { ... source() for EngineDriveFailed ... }
impl Display for RuntimeError { ... fmt() for EngineDriveFailed ... }
```

### 13. `crates/vb_runtime/src/error/equality.rs` — missing equality
**Defect:** No `PartialEq` support for `EngineDriveFailed`.

**Fix:** Added equality using `diagnostic_code()` comparison since `CoreError` has no `PartialEq`.

## Commands Run

```bash
# Focused tests
TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo test -p velvet-ballistics-workspace-tests \
  --test vb_qi37_12_state8_silent_discard_contract

# Compile check
TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo check --workspace --all-targets --all-features

# Clippy (production crates pass; fuzz crate has clippy::panic in test infra)
TMPDIR=target/tmp RUSTC_WRAPPER= rtk cargo clippy --workspace --lib --bins --examples \
  --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used \
  -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn \
  -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro \
  -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap \
  -D clippy::arithmetic_side_effects -D clippy::as_conversions \
  -D clippy::let_underscore_must_use -D clippy::await_holding_lock
```

## Test Results
- **44 passed, 0 failed** (all red tests now green)
- **Compile:** 0 errors, 1 warning (unused variable in test)
- **Clippy (production crates):** 0 errors

## Non-Negotiables Compliance
- ✅ No `unsafe` in production code
- ✅ No `unwrap`/`expect`/`panic`/`todo`/`unimplemented` in production code
- ✅ No unchecked indexing or arithmetic (used `u32::try_from()` for bounds)
- ✅ No lossy `as` conversions (replaced `as u32` with `try_from`)
- ✅ Typed errors throughout (`Result<Option<SlotValue>, JournalError>`)
- ✅ `#[must_use]` on fallible functions

## Files Modified
- `crates/vb_storage/src/events.rs`
- `fuzz/src/lib.rs`
- `crates/vb_runtime/src/error/mod.rs`
- `crates/vb_runtime/src/error/diagnostics.rs`
- `crates/vb_runtime/src/error/display.rs`
- `crates/vb_runtime/src/error/equality.rs`
