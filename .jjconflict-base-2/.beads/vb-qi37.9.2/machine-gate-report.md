# Machine Gate Report — vb-qi37.9.2

**Bead**: vb-qi37.9.2 — F64 bytecode semantics
**Date**: 2026-05-14
**Gate**: State 11 formal-verifier machine gates

---

## Clippy Gate

```
cargo clippy -p vb_expr -p vb_core --lib --bins -- -D warnings 2>&1
Result: PASS (exit 0)
Warnings: 0
```

---

## Build Gate

```
cargo build -p vb_expr -p vb_core 2>&1
Result: PASS (exit 0)
```

vb_expr: compiled successfully
vb_core: compiled successfully

---

## Test Suite Summary

| Package | Test Filter | Result | Count |
|---------|-------------|--------|-------|
| vb_core | finite_f64 | PASS | 14 tests |
| vb_core | finite_f64_accepts | PASS | 1 test |
| vb_expr | f64 | PASS | 38 tests |
| vb_expr | f64_div | PASS | (included above) |
| vb_expr | stack_overflow | PASS | 3 tests |
| vb_expr | integer_overflow | PASS | 4 tests |
| **vb_expr total** | **(all)** | **PASS** | **338 tests** |

---

## Kani Gate

```
cargo kani --package vb_expr 2>&1
Result: PASS
Harnesses: 7 successfully verified, 0 failures
Total checks: 639 (0 failed, 5 unreachable)
```

---

## Cargo Careful Gate

```
cargo careful test -p vb_expr 2>&1
Result: PASS (exit 0)

cargo careful test -p vb_core 2>&1
Result: PASS (exit 0)
```

---

## Pre-existing Global Debt

| Item | Evidence | Classification |
|------|----------|----------------|
| vb_runtime build failure (missing chunk_001.rs) | baseline-report.md | DEFERRED_GLOBAL — outside scope, not blocking |

---

**Machine Gate Status**: PASS
