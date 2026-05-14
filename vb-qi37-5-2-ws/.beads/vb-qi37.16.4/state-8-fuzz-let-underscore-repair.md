# State 8 Fuzz Let-Underscore-Must-Use Repair — vb-qi37.16.4

**bead_id:** vb-qi37.16.4
**phase:** state-8 fuzz let_underscore_must_use repair
**date:** 2026-05-11
**release_critical:** true
**STATUS:** REPAIRED

---

## Fix Applied

### Root Cause

The `clippy::let_underscore_must_use` lint fires when using `let _ = expr` where `expr` returns a `#[must_use]` type (e.g., `Result`). The `let _ =` pattern doesn't explicitly consume the `#[must_use]` result.

### Fix Strategy

Added `.ok()` to all fallible result expressions in fuzz targets. `.ok()` converts `Result<T, E>` to `Option<T>`, which drops the `#[must_use]` semantics and allows explicit discard via `let _ =` without lint violation.

**Preserved fuzz behavior:** Fuzz targets continue to call the functions and consume results — they just explicitly discard the fallible outcomes. No panics, unwraps, expects, or semantic narrowing.

### `fuzz/fuzz_targets/decode_record.rs`

All `vb_storage::decode_record_header` and `vb_storage::decode_record` calls now use `.ok()`:

```rust
// Before (triggers let_underscore_must_use):
let _ = vb_storage::decode_record_header(data, expected_magic, max_payload_len);

// After (lint-clean):
let _ = vb_storage::decode_record_header(data, expected_magic, max_payload_len).ok();
```

8 total occurrences fixed.

### `fuzz/fuzz_targets/lex_expr.rs`

```rust
// Before:
let _ = result.map(|tokens| {
    for token in tokens {
        let _ = token;
    }
});

// After:
let _ = result
    .map(|tokens| {
        for token in tokens {
            let _ = token;
        }
    })
    .ok();
```

---

## Command Evidence

### 1. `rtk cargo fmt -- --check`

```
FORMAT: PASS
(no diff output)
```

### 2. `rtk cargo clippy -p velvet-ballastics-fuzz --lib --bins -- -D clippy::let_underscore_must_use`

```
LINT: PASS
cargo clippy: 0 errors, 1 warnings
```

The 1 warning is a pre-existing duplicate package warning unrelated to lint failures.

### 3. `rtk cargo check -p velvet-ballastics-fuzz --all-targets`

```
CHECK: PASS
cargo build: 0 errors, 1 warnings (1 crates)
```

---

## Gate Summary

| Gate | Result |
|------|--------|
| `rtk cargo fmt -- --check` | PASS (clean) |
| `rtk cargo clippy -p velvet-ballastics-fuzz -- -D clippy::let_underscore_must_use` | PASS (0 errors) |
| `rtk cargo check -p velvet-ballastics-fuzz --all-targets` | PASS (0 errors) |

**All three required gates pass.**

---

## Classification

The `let_underscore_must_use` errors were fuzz target lint violations. Fix uses `.ok()` to explicitly discard `#[must_use]` results without panic. Fuzz behavior is preserved: functions are called and results consumed, just not asserted upon.

---

## Non-Touched Files

Per instruction, the following were NOT modified:
- `crates/vb_ipc/` (vb_ipc)
- `xtask/`
- `vb_ui_model` crate