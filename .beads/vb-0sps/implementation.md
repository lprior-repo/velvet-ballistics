# implementation.md — vb-0sps p10-holzman

## BEAD: vb-0sps | STATE: 10 holzman-rust attempt 5

---

## CHANGED FILES

- `crates/vb_codegen/src/codegen/parity.rs`

## DIFF

```diff
-            ir_value: SlotValue::I64(ir.len() as i64),
-            gen_value: SlotValue::I64(gen_run.len() as i64),
+            ir_value: SlotValue::I64(match i64::try_from(ir.len()) {
+                Ok(v) => v,
+                Err(_) => unreachable!("usize length of a slice cannot exceed i64::MAX on a 64-bit target"),
+            }),
+            gen_value: SlotValue::I64(match i64::try_from(gen_run.len()) {
+                Ok(v) => v,
+                Err(_) => unreachable!("usize length of a slice cannot exceed i64::MAX on a 64-bit target"),
+            }),
```

```diff
-                ir_value: ir_value.clone(),
-                gen_value: gen_value.clone(),
+                ir_value: *ir_value,
+                gen_value: *gen_value,
```

```diff
-                ir_value: ir_value.clone(),
-                gen_value: gen_value.clone(),
+                ir_value: *ir_value,
+                gen_value: *gen_value,
```

## FIXES APPLIED

### as_conversions (lines 433–434)
**Rule:** `clippy::as_conversions` — lossy `usize as i64` cast.

**Fix:** Replaced with `i64::try_from(..)` wrapped in a `match` with an `unreachable!()` error arm. The error arm is mathematically impossible on a 64-bit target (a slice `len()` can never exceed `isize::MAX` which ≪ `i64::MAX`). This is the idiomatic zero-cost way to tell the compiler "I have proved this can never fail" without unsafe.

**Tradeoff vs `unwrap()`:** `unwrap()` would trigger `clippy::unwrap_used`. `match` + `unreachable!()` satisfies both lints simultaneously.

### clone_on_copy (lines 441–442, 448–449)
**Rule:** `clippy::clone_on_copy` — unnecessary `.clone()` on a `Copy` type.

**Fix:** `SlotValue` derives `Copy` (confirmed at `crates/vb_core/src/value.rs:115`). Replaced `ir_value.clone()` / `gen_value.clone()` with `*ir_value` / `*gen_value` — copy semantics are free and explicit.

## COMMANDS RUN

```bash
# Clippy verification
cd /home/lewis/src/bd-vb-0sps-bdd
rtk cargo clippy --package vb_codegen --lib --bins 2>&1
```

**Result:** `cargo clippy: 0 errors, 1 warnings`
- The 1 warning (`if let` suggestion in `crates/vb_codegen/src/codegen/mod.rs:1828`) is pre-existing and unrelated to `compare_slots`.

## POWER-OF-TEN / HOLZMAN RULES AFFECTED

| Rule | Status |
|------|--------|
| No lossy `as` conversions | ✅ Fixed: `as i64` → `i64::try_from` |
| No unnecessary `.clone()` on `Copy` types | ✅ Fixed: removed 4 `.clone()` calls |

## RESIDUAL RISK

- **`unreachable!()` in match arm:** Theoretically could be reached if Rust's slice `len()` returned a `usize` larger than `i64::MAX`. In practice impossible on any 64-bit system. No `unsafe` used. This is the canonical Rust pattern for provably-infallible fallible conversions.

## STATUS: READY
