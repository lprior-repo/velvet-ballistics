# Machine Gate Report — vb-qi37.1.4

## State: 11 (formal-verifier)

## Bead
- **ID**: vb-qi37.1.4
- **Title**: runtime/recovery: Fail closed on incomplete recovery
- **Date**: 2026-05-14

---

## Tool Availability

| Tool | Status | Path |
|------|--------|------|
| verus | ✓ Available | /home/lewis/.local/bin/verus |
| tlc | ✓ Available | /home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc |
| cargo | ✓ Available | /home/lewis/.cargo/bin/cargo |

---

## Verus Gate

**Command**: `verus verification/verus/recovery_verification.rs`

**Output**:
```
verification results:: 7 verified, 0 errors
```

**Result**: PASS ✓

---

## TLA Gate

**Command**: `tlc -config RecoveryReplay.cfg RecoveryReplay.tla`

**Output**:
```
Fatal errors while parsing TLA+ spec in file RecoveryReplay
java.lang.NullPointerException: Cannot invoke "String.length()" because "str" is null
Error: Parsing or semantic analysis failed. Module-Table lookup failure for module name RecoveryReplay
```

**Result**: FAIL — Tooling issue with TLC/Java interaction

---

## Cargo Gate

**Command**: `cargo check -p vb_storage`

**Output**:
```
error: failed to select a version for the requirement `verus = "^1"`
candidate versions found which didn't match: 0.0.0
```

**Result**: FAIL — verus dependency not on crates.io (tooling limitation)

---

## Summary

| Gate | Result | Notes |
|------|--------|-------|
| verus | PASS | 7 verified, 0 errors |
| tlc | FAIL | TLC/Java tooling issue |
| cargo | FAIL | verus dependency not on crates.io |

---

*machine-gate-report: state 11 (formal-verifier) for vb-qi37.1.4*