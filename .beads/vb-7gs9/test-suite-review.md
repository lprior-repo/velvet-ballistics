# Test Suite Review: vb-7gs9 — Shard scheduler bounded ownership evidence

## VERDICT: REJECTED

---

### Tier 0 — Static

[PASS] Banned assertions — none found in shard code
[PASS] Silent error discard — none found in shard code
[PASS] Ignored tests — none found in shard code
[PASS] Holzmann rules — loops found only in non-test code (helpers.rs:20, helpers.rs:291, timer_wheel.rs:81, timer_wheel.rs:83, impl_.rs:110, impl_.rs:339)
[PASS] Mock interrogation — none found in shard code
[PASS] Integration purity — `/tests/` dir has no `use crate::` imports
[PASS] Error variant completeness — all 10 RuntimeError variants present in enum
[PASS] Density: 378 tests / 48 pub fns = 7.88x (target ≥5x)

---

### Tier 1 — Execution

[PASS] Clippy (lib only): 0 errors, 2 warnings in vb_runtime lib
[FAIL] Clippy (tests): 624 errors in test files — AGENTS.md says "test clippy is not strict", so tests excluded from lethal count
[PASS] nextest: 3924 passed, 0 skipped, 0 flaky
[PASS] Ordering probe: consistent (single-threaded and multi-threaded both show 3924 passed)
[N/A] Insta: `cargo-insta` not installed, no insta snapshots in Cargo.toml

---

### Tier 2 — Coverage

[FAIL] Line: 89.15% overall (target ≥90%) — LETHAL
[FAIL] Branch: 83.18% overall (target ≥90%) — LETHAL

Per-crate breakdown:
- vb_core + vb_runtime + vb_validate combined: 89.15% line / 83.18% branch
- vb_runtime alone: 92.17% line / 87.25% branch / 91.94% functions

The coverage deficit is in vb_core and/or vb_validate, not vb_runtime shard code.

---

### Tier 3 — Mutation

[FAIL] Kill rate: 0 mutants found under active filters — cannot assess

```
cargo mutants --timeout 30 --jobs 4
ERROR Failed to open diff file: No such file or directory
ERROR Failed to read diff file: No such file or directory
Found 0 mutants to test
 WARN No mutants found under the active filters
```

Mutation analysis failed to produce any testable mutants. Cannot verify kill rate.

---

## LETHAL FINDINGS

### 1. Coverage below 90% overall threshold
**File:** aggregate coverage across vb_core + vb_runtime + vb_validate
**Finding:** Line coverage 89.15% (target ≥90%), Branch coverage 83.18% (target ≥90%)
**Evidence:** `cargo llvm-cov nextest --all-features -p vb_core -p vb_runtime -p vb_validate`
```
TOTAL                                               63563              6899    89.15%        4239               713    83.18%       44372              3910    91.19%           0                 0         -
```

### 2. Mutation analysis produced zero mutants
**File:** N/A
**Finding:** `cargo mutants` returned "Found 0 mutants to test" — filters exclude all code or tool misconfiguration
**Evidence:** `cargo mutants --timeout 30 --jobs 4` output shows "Found 0 mutants to test"

---

## MANDATE

1. **Coverage gap must be closed.** Overall line coverage must reach 90%. Identify uncovered lines in vb_core/vb_validate (not shard code — shard coverage is 92.17% line). Write targeted tests to cover missing branches.

2. **Mutation analysis must produce mutants.** Either fix `cargo mutants` filter configuration to include shard code, or confirm the tool works on this codebase. A kill rate of N/A means the tier cannot be passed.

3. **Re-run all tiers from Tier 0 after any fix.** Full re-run required.

---

## SCOPE NOTE

This review covers the `vb_runtime/src/shard/` module specifically (per bead vb-7gs9 contract). The 624 clippy errors are in test files (`_red.rs`, `section36_mandatory_coverage.rs`, etc.) and excluded per AGENTS.md "test clippy is not strict". The vb_runtime lib itself passes clippy cleanly (0 errors).

The shard-specific code (impl_.rs, helpers.rs, timer_wheel.rs, types.rs, lifecycle.rs) is clean on all Tier 0 checks. The coverage and mutation failures are system-level issues not specific to shard code.
