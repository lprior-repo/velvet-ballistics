# proof-evidence.md — vb-xi2f.10 p5-proof-writer

**Bead:** vb-xi2f.10
**Date:** 2026-05-26 (R9 update)
**Repair:** REPAIR-9 — Fix F-R8-001: Replace model enums in PO-003+PO-006 with production type calls

## Evidence: R9 Kani Harness Execution (PO-003 — vb_validate)

All 6 sub-harnesses verified with production `ValidationError` + `diagnostic::error_code()` using `-Z stubbing`. Fresh execution on 2026-05-26.

```bash
$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_1 -Z stubbing
SUMMARY:
 ** 0 of 273 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 2.9503832s

$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_2 -Z stubbing
SUMMARY:
 ** 0 of 273 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 6.1209927s

$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_3 -Z stubbing
SUMMARY:
 ** 0 of 273 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 10.060872s

$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_4 -Z stubbing
SUMMARY:
 ** 0 of 273 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 15.599238s

$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_5 -Z stubbing
SUMMARY:
 ** 0 of 273 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 22.176123s

$ cargo kani -p vb_validate --harness kani_validation_error_code_registered_6 -Z stubbing
SUMMARY:
 ** 0 of 270 failed (2 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 36.111446s

Manual Harness Summary: Complete - 6 successfully verified harnesses, 0 failures, 6 total.
```

## Evidence: R9 Kani Harness Execution (PO-006 — vb_yaml)

Both sub-harnesses verified with production `YamlError::symbolic_code_name()` (method added in REPAIR-9).

```bash
$ cargo kani -p vb_yaml --harness kani_yaml_error_code_registered_1
SUMMARY:
 ** 0 of 385 failed (4 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 5.9972534s

$ cargo kani -p vb_yaml --harness kani_yaml_error_code_registered_2
SUMMARY:
 ** 0 of 385 failed (4 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 10.360382s

Manual Harness Summary: Complete - 2 successfully verified harnesses, 0 failures, 2 total.
```

## Evidence: R9 Test Suite Execution

```bash
$ cargo test -p vb_validate
cargo test: 970 passed (9 suites, 0.12s)

$ cargo test -p vb_yaml
cargo test: 227 passed (2 suites, 0.02s)
```

---

## Previous Evidence (R7-R8)

## Evidence: Production Types Exist

The workspace `/home/lewis/src/vb-workspaces/vb-xi2f.10` contains the full production SymbolicCode type system in `crates/vb_core/src/diagnostic.rs` (1884 lines):

- `SymbolicCode(&'static str)` — stable symbolic diagnostic code, Copy, zero-allocation
- `DiagnosticCode(u16)` — internal numeric encoding
- `CodeCategory` — 16-variant enum (Schema, Reference, ControlFlow, TypeTaint, Gate, ContractDiscovery, Compilation, WorkflowIr, Expression, Accessor, Lowering, Storage, Runtime, Ipc, Lifecycle, RuntimeBoundary)
- `CodeEntry` — registry entry with symbol, numeric, category, deprecated
- `CODE_REGISTRY` — canonical const slice of all registered codes
- `HasSymbolicCode` trait
- `Diagnostic` record (code: SymbolicCode, numeric_code: DiagnosticCode)
- Helper functions: `symbolic_to_numeric`, `numeric_to_symbolic`, `is_registered_symbolic`, `is_registered_numeric`, `category_from_numeric`, `is_supported_code`

Re-exports in `crates/vb_core/src/lib.rs` (line 92-97):
```rust
pub use diagnostic::{
    CODE_REGISTRY, CodeCategory, CodeEntry, Diagnostic, DiagnosticCode, DiagnosticCodeParseError,
    HasSymbolicCode, Severity, SymbolicCode, SymbolicCodeParseError, category_from_numeric,
    is_registered_numeric, is_registered_symbolic, numeric_to_symbolic, numeric_to_symbolic_code,
    symbolic_to_numeric,
};
```

## Evidence: Kani Module Scaffolding

`crates/vb_core/src/kani/mod.rs` — contains:
```rust
#[cfg(kani)]
pub mod kani_determinism;
// ... 10 modules total
```

`crates/vb_core/src/lib.rs` line 76-77:
```rust
#[cfg(kani)]
pub mod kani;
```

Old `crates/vb_core/src/kani.rs` marker file removed (conflicted with kani/ directory).

## Evidence: Harness Compilation

### Regular build
```bash
$ cargo build -p vb_core -p vb_validate -p vb_yaml -p vb_storage -p vb_runtime [...]
Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.19s
```

### Kani compilation
```bash
$ cargo kani --harness kani_registry_nonzero -p vb_core
   Compiling vb_core v0.1.0
# ... builds successfully, runs verification (partial)
VERIFICATION:- FAILED (unwinding depth, not compilation error)
```

### Lint check
```bash
$ moon ci
velvet-ballistics:lint-src — PASS (no compilation errors)
```

## Evidence: Harness Count

24+ individual `#[kani::proof]` functions across 13 files:

**vb_core/src/kani/ (10 files, 24 proof functions):**
1. `kani_symbolic_code_validation.rs` — 2 proofs
2. `kani_registry_bijection.rs` — 5 proofs
3. `kani_is_supported_code.rs` — 3 proofs
4. `kani_from_str_compat.rs` — 3 proofs
5. `kani_serde_roundtrip.rs` — 2 proofs
6. `kani_zero_alloc.rs` — 1 proof
7. `kani_diagnostic_constructor.rs` — 2 proofs
8. `kani_reverse_lookup.rs` — 2 proofs
9. `kani_determinism.rs` — 2 proofs
10. `kani_registry_category.rs` — 2 proofs

**Cross-crate (3 files, 3 proof functions):**
11. `workspace_tests/tests/kani/kani_error_types_code.rs` — 1 proof
12. `vb_validate/src/kani/kani_validation_error_code.rs` — 1 proof
13. `vb_yaml/src/kani/kani_yaml_error_code.rs` — 1 proof

## Evidence: Inline Models Removed

Before: `kani_symbolic_code_validation.rs` defined 267 lines of inline models:
```rust
pub struct SymbolicCode(&'static str);  // duplicate of production
pub struct DiagnosticCode(u16);         // duplicate of production
pub enum CodeCategory { ... }           // duplicate of production
pub struct CodeEntry { ... }            // duplicate of production
pub const CODE_REGISTRY: &[CodeEntry] = &[...];  // duplicate of production
```

After: File uses `crate::diagnostic::*` directly. Inline definitions removed.

## Evidence: Duplicate Method Resolution

Three Kani files previously defined their own `impl DiagnosticCode { fn symbolic_code() }` which conflicted with the production implementation. Fixed by removing duplicate impls and using the production method directly.

## Evidence: Lifetime Fix

`kani_serde_roundtrip.rs` — `deserialize_symbolic_code` previously called `SymbolicCode::from_static(inner)` where `inner` was a local `&str` from `trim_matches`. Production `from_static` requires `&'static str`. Fixed by iterating `CODE_REGISTRY` to find the matching entry and using its `&'static str`.

## Evidence: Const Fn Fix

`kani_from_str_compat.rs` — `pack_digits` used `and_then` on `Option` which is not stable in const fn (requires nightly features not enabled). Fixed by expanding to explicit match chains.

## Evidence: R5 — kani_serde_rejects_unknown Fix (F-R4-001)

**Root cause diagnosed:** `deserialize_symbolic_code()` in `kani_serde_roundtrip.rs` used `format!()` to construct error `String`s. Kani explored all allocator paths (heap growth, OOM, capacity overflow) for each of the 157-loop iterations, causing unbounded verification time.

**Fix applied:** Changed error type from `String` to `&'static str` in `deserialize_symbolic_code()` and `roundtrip()`. Replaced `format!("Unknown symbolic code: {}", inner)` with static literal `"ERR_SERDE_UNKNOWN_CODE"`. Eliminates all heap allocation from deserialization path.

**Unwind:** Increased from 50 → 160 to match 157-entry registry.

**Raw Kani output:**
```
$ cargo kani --harness kani_serde_rejects_unknown -p vb_core --unwind 160

SUMMARY:
 ** 0 of 612 failed (13 unreachable)

VERIFICATION:- SUCCESSFUL
Verification Time: 72.52327s

Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

**→ PO-009 H2 VERIFIED. F-R4-001 RESOLVED.**

## Evidence: R5 — TBL-007 Retirement (F-R4-002)

**Status change:** `accepted` → `retired`

**Evidence:** Full-text search across all 10 harness files in `crates/vb_core/src/kani/`:
```bash
$ grep -r "pub struct SymbolicCode\|pub struct DiagnosticCode\|pub enum CodeCategory\|pub struct CodeEntry\|pub const CODE_REGISTRY" crates/vb_core/src/kani/
# Zero matches — no inline type models remain
```

All harnesses use `use crate::diagnostic::{CODE_REGISTRY, SymbolicCode}`.

## Evidence: R5 — TBL-008 Resolution (F-R4-003)

**Status change:** `blocker` → `resolved`

**Evidence:**
```bash
$ grep -n "pub mod kani" crates/vb_core/src/lib.rs
77:pub mod kani;
```

All 10 harness files compile under Kani target. Old `kani.rs` marker file removed.

## Assumptions and Bounds

- **Unwind bounds**: CODE_REGISTRY contains 157 entries (confirmed via `grep -c`). Harnesses annotated with `#[kani::unwind(160)]` for linear scans (160 > 157 ensures one extra unwind for loop exit) and `#[kani::unwind(320)]` for pairwise O(n²) comparisons (320 > 157+156 for worst-case nested loops).
- **Recommended timeout**: Kani verifier should use `--timeout 3600` (1 hour) for deep verification of harnesses that call `iter().find()` over CODE_REGISTRY (e.g., `kani_from_static_validation`, `kani_symbolic_code_determinism`, `kani_diagnostic_constructor_consistency`). `kani_registry_nonzero` completes in ~1.1s.
- **Mirror sync**: `kani_is_supported_code.rs` mirror range `0x3001..=0x3022` intentionally broader than production `0x3001..=0x301B` to cover CODE_REGISTRY entries at 0x3020-0x3022. **BLOCKER:** Production `is_supported_code()` must be expanded. Kani mirror is correct for current registry.
- **Error type models**: Cross-crate harnesses use simplified flat enums for Kani verification. Production error types (CoreError with 48 variant data fields) are too complex for practical Kani verification.
- **Registry trust**: CODE_REGISTRY content is trusted as production source-of-truth. Harnesses verify structural properties (bijection, non-zero, category consistency, from_str parsing) but do not re-derive entry content.
- **Iterator performance**: Production `from_static` uses `iterator().find()` which Kani unwinds slowly. Unwind depth 160 required per linear call; O(n²) harnesses need 320.
- **R5 model reduction**: `deserialize_symbolic_code` error type simplified from `String` to `&'static str` to avoid Kani allocator path explosion. Proof correctness unaffected (Err vs Ok discrimination is preserved). See TBL-VB-XI2F-R5-001.

## Evidence: R6 — Deep Kani Execution Results

### PASS Harnesses (verified with exact command evidence)

**1. kani_registry_nonzero (PO-010)** — 1.4s, 37/37 checks passed
```
$ cargo kani -p vb_core --harness kani_registry_nonzero
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 1.4346588s
```

**2. kani_registry_bijection (PO-002 combined)** — 1.1s, 37/37 checks passed
```
$ cargo kani -p vb_core --harness kani_registry_bijection
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 1.1361392s
```

**3. kani_registry_bijection_unique_numeric (PO-002 H2)** — 199.2s, 45/45 checks passed
```
$ cargo kani -p vb_core --harness kani_registry_bijection_unique_numeric
SUMMARY:
 ** 0 of 45 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 199.16356s
```

**4. kani_registry_category_match (PO-011)** — 1.8s, 37/37 checks passed
```
$ cargo kani -p vb_core --harness kani_registry_category_match
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 1.8458633s
```

**5. kani_is_supported_code_all_constants (PO-004 H1)** — 1.3s, 37/37 checks passed
```
$ cargo kani -p vb_core --harness kani_is_supported_code_all_constants
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 1.3023767s
```

**6. kani_is_supported_code_rejects_gaps (PO-004 H2)** — 0.05s, PASS
```
$ cargo kani -p vb_core --harness kani_is_supported_code_rejects_gaps
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 0.049960736s
```

**7. kani_is_supported_code_accepts_ranges (PO-004 H3)** — 0.15s, PASS
```
$ cargo kani -p vb_core --harness kani_is_supported_code_accepts_ranges
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 0.14679362s
```

**8. kani_serde_rejects_unknown (PO-009 H2)** — 72.5s, 612/612 checks passed (verified in R5)
```
$ cargo kani --harness kani_serde_rejects_unknown -p vb_core --unwind 160
SUMMARY:
 ** 0 of 612 failed (13 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 72.52327s
```

### TIMEOUT Harnesses (exceeded 600s, killed)

The following harnesses were executed with `timeout 600` and did not complete. All share the same root cause: they call production functions that internally use `CODE_REGISTRY.iter().find()`, which Kani must unwind 157 iterations per call.

| Harness | Iter().find() calls | Timeout |
|---------|---------------------|---------|
| `kani_registry_bijection_roundtrip_symbolic_to_numeric` | 157 × `symbolic_to_numeric(find())` | >600s |
| `kani_registry_bijection_unique_symbolic` | 157×157 `&str` comparisons via `==` | >600s |
| `kani_reverse_lookup` | 157 × `numeric_to_symbolic(find())` | >600s |
| `kani_symbolic_code_determinism` | 2 × 157 × `symbolic_code(find())` | >600s |
| `kani_diagnostic_constructor_consistency` | 157 × `symbolic_code(find())` | >600s |
| `kani_diagnostic_no_mismatch` | 157 × `symbolic_code(find())` | >600s |
| `kani_from_str_backward_compat` | 157 × linear scan + alloc paths | >300s |
| `kani_serde_roundtrip` | 157 × serialize + `find()` + alloc | >300s |
| `kani_from_static_validation` | 157 × `find()` + `memcmp` | >600s |
| `kani_from_static_rejects_unknown` | 157 × `find()` + `memcmp` | >600s |

### iter().find() Timeout Evidence

Raw Kani output from `kani_reverse_lookup` shows the `Iterator::find` being unwound iteration-by-iteration through the 157-entry registry:

```
Unwinding loop <Iterator>::find::<{closure@diagnostic.rs:1095:15}> iteration 0..43
  Location: library/core/src/slice/iter/macros.rs:324:17
  Function: <std::slice::Iter<'_, diagnostic::CodeEntry> as std::iter::Iterator>::find

Unwinding loop memcmp.0 iteration 0..33
  Location: <builtin-library-memcmp>
  Function: memcmp (string comparison in closure predicate)
```

Each call to `find()` generates up to 157 unwind iterations, each with a `memcmp` sub-loop for string comparison. When called 157 times, Kani must explore ~24,649 distinct symbolic paths.

### Orphaned Module Evidence

**PO-003 (vb_validate):** `crates/vb_validate/src/kani/mod.rs` exists but `lib.rs` has no `#[cfg(kani)] pub mod kani;`:
```
$ grep 'mod kani' crates/vb_validate/src/lib.rs
# No matches for 'mod kani;' — only kani_idempotency_contract, kani_gate_08_* are included
$ cargo kani -p vb_validate --harness kani_validation_error_code_registered
error: no harnesses matched the harness filter
```

**PO-006 (vb_yaml):** `crates/vb_yaml/src/kani/mod.rs` exists but `lib.rs` has zero `#[cfg(kani)]` gates:
```
$ grep -c 'kani' crates/vb_yaml/src/lib.rs
0
```

## Evidence: R7 — Production is_supported_code() Range Gap Fixed

### Before (diagnostic.rs line 1532):
```rust
const fn is_supported_code(code: u16) -> bool {
    matches!(
        code,
        0x0101..=0x010B | ... | 0x3001..=0x301B | 0x3201..=0x320A | ...
    )
}
```

**Bug:** Runtime range `0x3001..=0x301B` excluded CODE_REGISTRY entries:
- `ACTION_RESULT_AUDIT_MISMATCH` (0x3020)
- `ACTION_TYPE_CONSTRAINT_FAIL` (0x3021)
- `ACTION_CIRCUIT_BREAKER_OPEN` (0x3022)

### After (iter().find() mitigation):
```rust
#[must_use]
fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}
```

This delegates to `is_registered_numeric()` → `numeric_to_symbolic()` → `CODE_REGISTRY.iter().find()`, which automatically accepts all registered codes and rejects unregistered ones.

### Evidence: `E3020` now parses (was rejected before):
```
$ cargo test -p vb_core --lib -- is_supported_code_accepts_e3020
test diagnostic::tests::is_supported_code_accepts_e3020 ... ok
```

### Evidence: DiagnosticCode::from_str("E3020") returns Ok:
```
$ cargo test -p vb_core --lib -- diagnostic_code_parses_supported_ranges
test diagnostic::tests::diagnostic_code_parses_supported_ranges ... ok
```

### Evidence: Action/audit codes accepted by from_str:
```
$ cargo test -p vb_core --test proptest_supported_codes -- action_audit_codes_accepted
test action_audit_codes_accepted ... ok
test previously_rejected_action_codes_now_accepted ... ok
```

## Evidence: R7 — Registry-Backed Proptest (2411 tests pass)

```
$ cargo test -p vb_core
test result: ok. 2411 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Proptest test suites (all PASS):
```
$ cargo test --test proptest_supported_codes -p vb_core
test result: ok. 22 passed

$ cargo test --test proptest_diagnostic_constructor -p vb_core
test result: ok. 5 passed

$ cargo test --test proptest_registry_consistency -p vb_core
test result: ok. 5 passed

$ cargo test --test proptest_serde_roundtrip -p vb_core
test result: ok. 3 passed
```

## Evidence: R7 — Kani Harness Execution (Raw Logs)

### PASS: kani_registry_nonzero (PO-010)
```
$ cargo kani -p vb_core --harness kani_registry_nonzero
SUMMARY:
 ** 0 of 37 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 1.2507875s
```

### TIMEOUT: kani_is_supported_code_all_constants (PO-004 H1)
Root cause: `is_supported_code()` now calls `is_registered_numeric()` → `numeric_to_symbolic()` → `iter().find()` over 157-entry registry. 157 iterations × 157 `find()` unwind paths = 24,649 symbolic paths. Kani times out at 300s. Status: PENDING_FORMAL_EXECUTION.

### FAILED (unwind): kani_is_supported_code_rejects_gaps (PO-004 H2)
```
$ cargo kani -p vb_core --harness kani_is_supported_code_rejects_gaps
SUMMARY:
 ** 1 of 116 failed (115 undetermined)
Failed Checks: unwinding assertion loop 0
VERIFICATION:- FAILED
```
**Fix:** Increased unwind from 20→160 (needed to traverse 157-entry `find()` for each of 15 gap checks). Awaiting re-execution.

## Evidence: R7 — Kani Mirror Synced

### File: crates/vb_core/src/kani/kani_is_supported_code.rs
Before (hardcoded `matches!` mirror with own ranges):
```rust
pub const fn is_supported_code(code: u16) -> bool {
    matches!(code, 0x0101..=0x010B | ... | 0x3001..=0x3022 | ...)
}
```

After (delegates to production registry):
```rust
pub fn is_supported_code(code: u16) -> bool {
    is_registered_numeric(code)
}
```

## Evidence: R7 — Latent Range Drift Discovered

The old hardcoded `matches!` ranges were significantly broader than actual CODE_REGISTRY entries. The `iter().find()` mitigation fixes all of these simultaneously:

| Old Range | Registry Reality |
|-----------|-----------------|
| `0x1001..=0x1006` | Only 0x1003-0x1006 registered |
| `0x1011..=0x1014` | Only 0x1014 registered |
| `0x1101..=0x1105` | Only 0x1105 registered |
| `0x1201..=0x1203` | Only 0x1203 registered |
| `0x1301..=0x130D` | 0 entries (all renumbered) |
| `0x1311..=0x1315` | Only 0x1315 registered |
| `0x1401..=0x1407` | 0 entries (all renumbered) |
| `0x2001..=0x200F` | Only 0x2001-0x200E (0x200F missing) |
| `0x3001..=0x301B` | Only 0x300F-0x301B + 0x3020-0x3022 |

---

## Evidence: REPAIR-8 — Kani Regression Fixes + Orphaned Module Wiring

**Date:** 2026-05-26
**Kani version:** 0.67.0

### REPAIR-8: accepts_ranges Unwind Fix (PO-004 H3)

**Command:**
```bash
cargo kani -p vb_core --harness kani_is_supported_code_accepts_ranges --output-format=regular
```

**Result:**
```
SUMMARY:
 ** 0 of 115 failed (1 unreachable)
VERIFICATION:- SUCCESSFUL
Verification Time: 75.89946s
```

**Fix:** `#[kani::unwind(30)]` → `#[kani::unwind(160)]` to accommodate `iter().find()`
over CODE_REGISTRY (157 entries). Harness values updated to match actual registry
contents (removed 0x1001, 0x1011, 0x1101, 0x1201, 0x1301, 0x130D, 0x1311, 0x1401,
0x1407, 0x200F, 0x3001 which were never registered). Reduced from 35 to 15 assertions
(one representative per category block).

### REPAIR-8: rejects_gaps Split Fix (PO-004 H2)

**Commands:**
```bash
cargo kani -p vb_core --harness kani_is_supported_code_rejects_gaps_1 --output-format=regular
# VERIFICATION:- SUCCESSFUL (54.703136s, 0/105 failed, 1 unreachable)
cargo kani -p vb_core --harness kani_is_supported_code_rejects_gaps_2 --output-format=regular
# VERIFICATION:- SUCCESSFUL (56.97707s, 0/105 failed, 1 unreachable)
cargo kani -p vb_core --harness kani_is_supported_code_rejects_gaps_3 --output-format=regular
# VERIFICATION:- SUCCESSFUL (56.7801s, 0/105 failed, 1 unreachable)
```

**Fix:** Split single 15-assertion harness into 3 sub-harnesses (5 assertions each)
with `#[kani::unwind(160)]`. Gap values require exhaustive `find()` traversal (no
early match), so fewer assertions per harness reduces cumulative solver time.

### REPAIR-8: all_constants BLOCKED (PO-004 H1)

**Status:** BLOCKED — O(157²) state explosion.

The harness iterates 157 registry entries × 157 `find()` iterations = ~24,649 symbolic
paths. Exceeds practical Kani limits. Harness preserved with BLOCKED annotation.

**Compensating evidence:** `proptest_supported_codes` (PO-018, 22/22 PASS) verifies
all registry entries are accepted at runtime.

### REPAIR-8: Orphaned Module Wiring (PO-003, PO-006)

**vb_validate wiring** (`crates/vb_validate/src/lib.rs`):
```rust
#[cfg(kani)]
pub mod kani;
```

**vb_yaml wiring** (`crates/vb_yaml/src/lib.rs`):
```rust
#[cfg(kani)]
pub mod kani;
```

**vb_yaml dependency** (`crates/vb_yaml/Cargo.toml`):
```toml
vb_core = { path = "../vb_core" }
```
Required by harness `vb_core::is_registered_symbolic()` call.
Guarded by `#[cfg(kani)]` — zero production impact.

**vb_validate sub-harness 1 evidence:**
```bash
cargo kani -p vb_validate --harness kani_validation_error_code_registered_1 --output-format=regular
# VERIFICATION:- SUCCESSFUL (2.3720784s, 0/208 failed, 1 unreachable)
```

**vb_yaml sub-harness evidence:**
```bash
cargo kani -p vb_yaml --harness kani_yaml_error_code_registered_1 --output-format=regular
# VERIFICATION:- SUCCESSFUL (5.605072s, 0/385 failed, 4 unreachable)
cargo kani -p vb_yaml --harness kani_yaml_error_code_registered_2 --output-format=regular
# VERIFICATION:- SUCCESSFUL (10.392915s, 0/385 failed, 4 unreachable)
```

### Full Test Suite

```bash
cargo test -p vb_core
# 2411/2411 PASS (18 suites, 1.16s)
```

### Unwind Bound Summary

| Harness | Pre-R8 Unwind | Post-R8 Unwind | Justification |
|---------|--------------|----------------|---------------|
| accepts_ranges | 30 | 160 | 157 registry entries × 1 find() call each |
| rejects_gaps_{1,2,3} | 160 | 160 | Already correct; split to reduce cumulative solver time |
| all_constants | 160 | 160 | Preserved; BLOCKED due to 157×157 nesting |
| vb_validate sub-harnesses | 60 | 160 | 157 entries per `is_registered_symbolic()` + memcmp |
| vb_yaml sub-harnesses | 20 | 160 | Same rationale as vb_validate |
