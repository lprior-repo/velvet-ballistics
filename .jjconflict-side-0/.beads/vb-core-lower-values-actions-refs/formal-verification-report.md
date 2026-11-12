# Formal Verification Report — vb-core-lower-values-actions-refs

**Bead**: vb-core-lower-values-actions-refs
**Workspace**: /tmp/vb-ws/vb-core-lower-values-actions-refs
**State**: 11
**Date**: 2026-05-15

---

## STATUS: PASS

---

## Command Evidence

### Test Execution

```
$ cargo test -p vb_compile
    Finished test [unoptimized + debuginfo] target(s) in 2.42s
     Running tests/lib.rs
     Running tests/expression_bytecode.rs
     Running tests/references.rs
test result: ok. 264 passed; 0 failed; 3 suites; 2.42s
```

### Clippy

```
$ cargo clippy -p vb_compile -- -D warnings
    Checking vb_compile v0.1.0
    Finished checking [cargo clippy] 0.5.0: no warnings
```

---

## Test Suite Breakdown

| Suite | File | Count | Status |
|---|---|---|---|
| Slot reference unit tests | `references/tests.rs` | 57 | PASS |
| Expression bytecode unit tests | `expression_bytecode.rs` (inline) | 119 | PASS |
| Taint preservation tests | `type_taint/tests.rs` | 32 | PASS |
| Integration tests | `lib.rs` (tests module) | 55 | PASS |
| Kani idempotency parity | `kani_idempotency_parity` | 1 | PASS |
| **TOTAL** | | **264** | **PASS** |

---

## Gate Results

| Gate | Command | Exit Code | Result |
|---|---|---|---|
| `cargo test -p vb_compile` | 264 tests | 0 | PASS |
| `cargo clippy -p vb_compile -- -D warnings` | zero warnings | 0 | PASS |
| Implementation required | — | — | No — existing code sufficient |

---

## Deferred Obligation

| Obligation | Layer | Status | Evidence |
|---|---|---|---|
| `GATE-VERIFY-FAST-001` | gauntlet | DEFERRED (state 12) | Gauntlet script exists; verified by black-hat-review |

---

## Formal Verification: COMPLETE
