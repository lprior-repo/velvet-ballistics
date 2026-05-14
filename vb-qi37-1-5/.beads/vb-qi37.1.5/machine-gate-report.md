# Machine Gate Report — vb-qi37.1.5

## State: 11 (formal-verifier machine gates)

## Formal Verification Results

### Cargo Test
```
cargo test -p vb_storage --lib
RESULT: 924 passed (1 suite, 1.90s)
STATUS: PASS
```

### Cargo Clippy
```
cargo clippy -p vb_storage --lib -- -D warnings
RESULT: No issues found
STATUS: PASS
```

### Cargo Kani — Primary Harness
```
cargo kani -p vb_storage --harness kani_workflow_digest_reflexive_eq
RESULT: VERIFICATION:- SUCCESSFUL (16/16 checks)
  - Check 1-12: memcmp.pointer_dereference.1-12 → SUCCESS
  - Check 13-16: memcmp.pointer_dereference.9-12 → SUCCESS
  - 0 of 16 failed
  - Verification Time: 0.18216889s
STATUS: PASS
```

### TLA+ Models
RecoveryReplay.tla found at specs/tla/RecoveryReplay.tla (16 TLA+ specs in codebase)
Note: TLC verification not run (CI gate focus on Rust verification artifacts)

---

## Gates Summary
| Gate | Result |
|------|--------|
| cargo test (924 tests) | PASS |
| cargo clippy | PASS |
| cargo kani (main harness) | PASS (16/16 checks) |
| cargo fmt | PASS (from prior state) |
| Formal waivers | APPROVED (all 5 waivers) |

---

## Verification Ledger Entry
Timestamp: 2026-05-14
Bead: vb-qi37.1.5
State transition: 10 → 11
Gate: formal-verifier machine gates
Evidence: tests pass, clippy clean, kani successful