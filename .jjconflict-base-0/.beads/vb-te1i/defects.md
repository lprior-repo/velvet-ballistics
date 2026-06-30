# Defects: vb-te1i — State 12

**Bead**: bdd: Binary IPC acceptance scenarios
**Reviewer**: black-hat-reviewer
**Date**: 2026-05-19

---

## Defects

### DEFECT-1: `assert_ok!` Macro Discards Ok Values

**File**: `crates/vb_ipc/src/frame/tests.rs`
**Lines**: 14–21
**Severity**: MAJOR
**Category**: Test anti-pattern / Weak assertion

**Description**:
The `assert_ok!` macro uses `Ok(_)` to match and discard the decoded value:
```rust
macro_rules! assert_ok {
    ($result:expr $(, $($arg:tt)+)?) => {{
        match &$result {
            Ok(_) => (),  // ← value discarded here
            Err(_) => assert_eq!(Some("Err(..)"), None::<&str> $(, $($arg)+)?),
        }
    }};
}
```

If `decode()` returns `Ok(IpcFrameHeader { correlation: N, ... })`, the macro passes silently without validating that `N` is correct.

**Compensating factor**: Every test using this macro ALSO performs a separate sharp extraction (`let Ok(...) = ... else { return }`) followed by explicit `assert_eq!` assertions on the extracted value. The macro is a guard-rail, not the primary evidence path. The actual assertions in every test using this macro are sharp.

**Required fix**:
Replace the macro with explicit guards paired with sharp assertions:
```rust
let result = decode(...);
assert!(result.is_ok(), "decode should succeed: {:?}", result);
let Ok(value) = result else { return };
assert_eq!(value.correlation, expected_correlation);
```

**Impact**: Test code only. Does not affect production `vb_ipc` crate correctness. Does not cause incorrect behavior in production.

---

## Summary

| Defect | Severity | Location | Fix Required |
|---|---|---|---|
| DEFECT-1: `assert_ok!` discards Ok values | MAJOR | `crates/vb_ipc/src/frame/tests.rs:14-21` | Yes |

**Production Impact**: None
**Test Suite Impact**: Weak assertion pattern — mitigated by explicit extractions in all callers

**Total**: 1 MAJOR, 0 LETHAL, 0 MINOR
