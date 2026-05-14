# State 8 Xtask Panic-Lint Repair — vb-qi37.16.4

**bead_id:** vb-qi37.16.4
**phase:** state-8 xtask panic_in_result_fn repair
**date:** 2026-05-11
**release_critical:** true
**STATUS:** REPAIRED

---

## Fix Applied

### `xtask/src/proof.rs:199` — `clippy::panic_in_result_fn` / `clippy::panic`

**Root cause:** The `write_proof_evidence` function returns `Result<PathBuf, String>`, but at line 199 it used `unwrap_or_else(|| panic!("Obligation not found: {}", id))` to handle a missing obligation lookup. This is a `panic_in_result_fn` violation.

**Fix:** Replaced the panic with typed `Result` error propagation:

- Changed `.unwrap_or_else(|| panic!(...))` to `.ok_or_else(|| format!("Obligation not found: {}", id))?`
- Wrapped the closure body in `Ok(...)` since the closure now returns `Result<ObligationStatus, String>`
- Added explicit type annotation `.collect::<Result<Vec<ObligationStatus>, String>>()?` to enable proper error collection

**Code change:**

```rust
// Before:
let obligations_status: Vec<ObligationStatus> = results
    .iter()
    .map(|(id, passed)| {
        let obl = obligations
            .iter()
            .find(|o| o.id == *id)
            .unwrap_or_else(|| panic!("Obligation not found: {}", id));
        ObligationStatus { ... }
    })
    .collect();

// After:
let obligations_status: Vec<ObligationStatus> = results
    .iter()
    .map(|(id, passed)| {
        let obl = obligations
            .iter()
            .find(|o| o.id == *id)
            .ok_or_else(|| format!("Obligation not found: {}", id))?;
        Ok(ObligationStatus { ... })
    })
    .collect::<Result<Vec<ObligationStatus>, String>>()?;
```

**Safety rationale:**
- No `unwrap`, `expect`, `panic`, `todo`, or `unsafe` used
- Error propagates as `String` via `?` operator
- `ok_or_else` is lazy and only constructs the error message if needed
- The error type `String` matches the function's `Result` return type

---

## Command Evidence

### 1. `rtk cargo fmt -- --check`

```
(no output — clean)
```

**STATUS: PASS**

### 2. `rtk cargo clippy -p xtask -- -D clippy::panic_in_result_fn -D clippy::panic`

```
cargo clippy: 0 errors, 1 warnings
```

**STATUS: PASS** — 0 errors. The 1 warning is a pre-existing duplicate package warning unrelated to lint failures.

### 3. `rtk cargo check -p xtask --all-targets`

```
cargo build: 0 errors, 1 warnings (3 crates)
```

**STATUS: PASS** — 0 errors. The 1 warning is the same pre-existing duplicate package warning.

---

## Gate Summary

| Gate | Result |
|------|--------|
| `rtk cargo fmt -- --check` | PASS (clean) |
| `rtk cargo clippy -p xtask -- -D clippy::panic_in_result_fn -D clippy::panic` | PASS (0 errors) |
| `rtk cargo check -p xtask --all-targets` | PASS (0 errors) |

**All three required gates pass.**

---

## Classification

The `panic_in_result_fn` violation was in `xtask/src/proof.rs` (a support crate, not runtime). The fix uses `ok_or_else` with `?` propagation to replace the panic with typed error handling. No panics, unwraps, expects, or unsafe code remain.

---

## Non-Touched Files

Per instruction, the following were NOT modified:
- `fuzz/` directory
- `crates/vb_ipc/`
- `vb_ui_model` crate
