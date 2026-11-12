# vb-tphef Test Cleanup Handoff

## Scope

Test construct cleanup only. No production code changes.

## Files Modified

1. `crates/vb_core/src/policy/new_tests.rs`
   - Replaced `.rule().unwrap()` with `matches!` assertion on Some(rule)
   - Replaced `match identity { Configured => assert_eq!, Unconfigured => panic! }` with direct `matches!` assertion with guard
   - Replaced `match unverified { Artifact => panic!, Unverified => {} }` with direct `matches!` assertion

2. `crates/vb_runtime/src/policy/emergency_and_admission_tests.rs`
   - Replaced `match record_a.admission { Artifact(d) => assert_eq!, Unverified => panic! }` with direct `assert_eq!(record_a.admission, PinnedAdmissionPolicy::Artifact(digest_a))`

3. `crates/vb_runtime/src/policy/tests.rs`
   - Replaced unchecked byte index assignments (`bytes[0] = 0xAB; bytes[31] = 0xCD`) with full array literal passed to `PolicyDigest::from_bytes()`

## Verification Commands and Results

### Tests

```bash
python3 /tmp/vb-run-gate.py 300 .beads/vb-tphef/evidence/TEST-CLEAN-001 cargo test -p vb_core -p vb_runtime --lib -- policy --test-threads=1
```
- Exit code: 0
- Duration: 19.951s
- Result: 92 passed, 0 failed, 0 ignored

### Clippy

```bash
python3 /tmp/vb-run-gate.py 300 .beads/vb-tphef/evidence/CLIPPY-CLEAN-001 cargo clippy -p vb_core -p vb_runtime --lib -- -D warnings
```
- Exit code: 0
- Duration: 8.417s
- Result: No warnings

### Format Check

```bash
python3 /tmp/vb-run-gate.py 120 .beads/vb-tphef/evidence/FMT-CHECK-002 cargo fmt --check -- crates/vb_core/src/policy/new_tests.rs crates/vb_runtime/src/policy/emergency_and_admission_tests.rs crates/vb_runtime/src/policy/tests.rs
```
- Exit code: 0
- Duration: 2.072s
- Result: All files formatted correctly

## Evidence Location

All gate runner logs are in `.beads/vb-tphef/evidence/`:
- `TEST-CLEAN-001.*` - test run
- `CLIPPY-CLEAN-001.*` - clippy run
- `FMT-CHECK-001.*` - first fmt check (failed, led to formatting fix)
- `FMT-CHECK-002.*` - final fmt check (passed)

## Notes

- All panic branches in test code replaced with `matches!` assertions preserving existing field assertions
- Unchecked index assignments removed in favor of array literals
- No production API changes made
- All original behavior assertions preserved