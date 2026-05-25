# test-repair-guide.md — vb-6f02 Test Repairs

**Bead**: vb-6f02 (Contract-as-Data Suite)  
**Status**: REJECTED by test-reviewer (see test-plan-review.md, test-suite-review.md)  
**Date**: 2026-05-18  

---

## Repair 1: Bind Proptest/Kani to Production Code

### Problem
Proptest and Kani suites test independent copies of `parse_schema_version`, `parse_contract_kind`, `compare_semver`, etc. These are NOT the production functions in `xtask/src/contracts.rs`.

### Fix
Add a new test file `crates/workspace_tests/tests/contracts_production_binding.rs` that imports directly from the xtask crate:

```rust
use velvet_ballistics_xtask::contracts::{
    ContractKind, ContractError, DiscoveryReport, ReportSummary,
    parse_schema_version, compare_semver, parse_vet_exit_code,
    gate_evidence_from_report,
};
```

Then rewrite the proptest and Kani properties to use these imports instead of local copies.

**For proptest**, the key change is replacing local `parse_schema_version` with `use xtask::contracts::parse_schema_version` and adapting the error type from `String` to `ContractError`.

**For Kani**, the `#[verifier::external]` functions must either:
- (a) Re-export the production functions directly (preferred), or
- (b) Be removed entirely and the harness must call the production functions through their public API.

### Priority: CRITICAL
Without this binding, the formal proofs prove properties about test dummies, not production code.

---

## Repair 2: End-to-End Integration Tests

### Problem
All unit tests construct `DiscoveryReport { files: ..., errors: ..., summary: ... }` directly. The `discover_contracts()` function (contracts.rs:259) is never called in tests.

### Fix
Add integration tests that exercise the full pipeline:

```rust
#[test]
fn test_discover_contracts_real_files() {
    // Create a temp directory with a valid .cue file
    let dir = tempdir().unwrap();
    let cue_file = dir.path().join("test.cue");
    fs::write(&cue_file, r#"package validation
kind: "cli_envelope"
schema_version: "1.0.0"
"#).unwrap();
    
    let report = discover_contracts(dir.path()).unwrap();
    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.valid, 1);
    assert_eq!(report.summary.invalid, 0);
}

#[test]
fn test_discover_contracts_invalid_kind() {
    let dir = tempdir().unwrap();
    let cue_file = dir.path().join("test.cue");
    fs::write(&cue_file, r#"package validation
kind: "bogus_kind"
schema_version: "1.0.0"
"#).unwrap();
    
    let report = discover_contracts(dir.path()).unwrap();
    assert_eq!(report.summary.total, 1);
    assert_eq!(report.summary.invalid, 1);
    assert!(!report.errors.is_empty());
}

#[test]
fn test_discover_contracts_missing_kind() {
    let dir = tempdir().unwrap();
    let cue_file = dir.path().join("test.cue");
    fs::write(&cue_file, r#"package validation
schema_version: "1.0.0"
"#).unwrap();
    
    let report = discover_contracts(dir.path()).unwrap();
    assert_eq!(report.summary.invalid, 1);
}

#[test]
fn test_discover_contracts_nested_dirs() {
    // Test recursive file collection
    let dir = tempdir().unwrap();
    let sub = dir.path().join("subdir");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("a.cue"), r#"package validation
kind: "cli_envelope"
schema_version: "1.0.0"
"#).unwrap();
    fs::write(dir.path().join("b.cue"), r#"package validation
kind: "ui_tokens"
schema_version: "2.0.0"
"#).unwrap();
    
    let report = discover_contracts(dir.path()).unwrap();
    assert_eq!(report.summary.total, 2);
    assert_eq!(report.summary.valid, 2);
}
```

### Priority: CRITICAL
This is the single largest gap. The production code works, but nobody has tested that it works when called through the real pipeline.

---

## Repair 3: Fix unwrap Calls (Zero-Panic Policy)

### contracts_as_data_props.rs — 8 unwrap calls

Replace all `cmp_ab.unwrap()` and similar patterns with `prop_assert_eq!`:

**Line 421** (current):
```rust
prop_assert_eq!(cmp.unwrap(), std::cmp::Ordering::Equal, ...);
```
**Fix** — already uses unwrap on the result of `compare_semver`. Change to:
```rust
prop_assert_eq!(cmp, Ok(std::cmp::Ordering::Equal), ...);
```

**Lines 442-443** (current):
```rust
let ab = cmp_ab.unwrap();
let ba = cmp_ba.unwrap();
```
**Fix**:
```rust
prop_assert_eq!(cmp_ab, Ok(ba.reverse()), ...);
prop_assert_eq!(cmp_ba, Ok(ab.reverse()), ...);
```
Or alternatively:
```rust
assert_eq!(cmp_ab, cmp_ba.map(|o| o.reverse()));
```

**Lines 471-476** (current):
```rust
if cmp_ab.is_ok() && cmp_ab.unwrap() == std::cmp::Ordering::Greater
    && cmp_bc.is_ok() && cmp_bc.unwrap() == std::cmp::Ordering::Greater
```
**Fix**:
```rust
if let (Ok(std::cmp::Ordering::Greater), Ok(std::cmp::Ordering::Greater)) = 
    (cmp_ab, cmp_bc) 
{
    let cmp_ac = compare_semver(&v1, &v3);
    prop_assert_eq!(cmp_ac, Ok(std::cmp::Ordering::Greater), ...);
}
```

**Lines 492-494** (current):
```rust
prop_assert!(compare_semver(&v1, &v2).is_ok() && compare_semver(&v1, &v2).unwrap() == ...)
```
**Fix** — compute once, assert on Result:
```rust
let c1 = compare_semver(&v1, &v2);
prop_assert_eq!(c1, Ok(std::cmp::Ordering::Less), "patch increase: v1 < v2");
```

### contracts.rs:743 — Weak assertion

**Current**:
```rust
assert!(result.is_err());
```
**Fix**:
```rust
assert!(matches!(result, Err(_)), "discover_contracts should return Err for nonexistent dir");
```

### Priority: MINOR
These are style/policy violations, not correctness bugs. The code is safe.

---

## Repair 4: Add JSON Output Validation

### Problem
REQ-009 requires `--json` flag to produce JSON compatible with moon task consumers. No test verifies this.

### Fix
```rust
#[test]
fn test_json_output_is_valid_json() {
    let report = DiscoveryReport {
        files: vec![],
        errors: vec!["INVALID_KIND: bogus".to_string()],
        summary: ReportSummary {
            total: 1,
            valid: 0,
            invalid: 1,
            errors_by_kind: BTreeMap::from_iter(vec![("INVALID_KIND: bogus".to_string(), 1)]),
            version_violations: vec![],
        },
    };
    let json = serde_json::to_string_pretty(&report).unwrap();
    // Verify it is parseable
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("JSON must be parseable");
    assert!(parsed.get("summary").is_some(), "JSON must have 'summary' key");
    assert!(parsed.get("errors").is_some(), "JSON must have 'errors' key");
}
```

### Priority: RECOMMENDED

---

## Repair 5: Test Cue Vet Integration Path

### Problem
`run_cue_vet()` (contracts.rs:239) invokes the real `cue` binary. No test exercises this path. The test at TST-030/TST-031 tests `parse_vet_exit_code()` which is a pure function that only checks `code == 0`.

### Fix
Add a conditional test that runs only when `cue` is available:

```rust
#[test]
fn test_run_cue_vet_with_invalid_file() {
    let dir = tempdir().unwrap();
    let cue_file = dir.path().join("invalid.cue");
    // Write syntactically invalid CUE
    fs::write(&cue_file, r#"package validation
kind: "cli_envelope"
schema_version: "1.0.0"
invalid_field: { missing_colon
"#).unwrap();
    
    // Only run if cue is available
    match run_cue_vet(&cue_file) {
        Ok((code, stderr)) => {
            // cue vet should fail on invalid CUE
            assert!(code != 0, "cue vet should fail on invalid CUE, got code {}", code);
            assert!(!stderr.is_empty(), "cue vet should produce stderr");
        }
        Err(e) => {
            // cue not installed — skip test
            eprintln!("Skipping: cue not available: {}", e);
        }
    }
}
```

### Priority: RECOMMENDED
This test is environment-dependent but adds valuable real-pipeline coverage.

---

## Repair 6: Test Version Monotonicity Gate

### Problem
REQ-005 requires that "upgrading a schema must never decrease its version". The production code has `compare_semver()` and `VersionViolation` but no code that actually reads a manifest and compares versions.

### Assessment
This is a **production code gap**, not just a test gap. The monotonicity gate is not implemented in `discover_contracts()`. The contract.md says:
> "xtask stores previous versions in `.beads/contracts/manifest.json`"

But no such code exists. Before writing tests, implement the manifest tracking in `discover_contracts()`.

### Fix (production code first):
1. Add manifest read/write functions to `xtask/src/contracts.rs`
2. In `discover_contracts()`, after validating a file, check if its previous version exists in manifest
3. If previous version exists, call `compare_semver()` — if new < old, add `VersionMonotonicityBreach` error

### Fix (tests after production):
```rust
#[test]
fn test_monotonicity_accepts_upgrade() {
    // Create manifest with version 1.0.0, then validate file with 2.0.0
    // Assert: no version violation error
}

#[test]
fn test_monotonicity_rejects_downgrade() {
    // Create manifest with version 2.0.0, then validate file with 1.0.0
    // Assert: VersionMonotonicityBreach error present
}
```

### Priority: MAJOR (blocks REQ-005 completion)

---

## Summary of Repair Priority

| Repair | Priority | Category | Effort |
|--------|----------|----------|--------|
| 1. Bind to production code | CRITICAL | Test architecture | Medium |
| 2. End-to-end integration tests | CRITICAL | Coverage gap | Medium |
| 3. Fix unwrap calls | MINOR | Policy compliance | Small |
| 4. JSON output validation | RECOMMENDED | REQ-009 | Small |
| 5. Cue vet integration | RECOMMENDED | REQ-007 | Small |
| 6. Monotonicity gate | MAJOR | Production + tests | Large |

**Order of operations**:
1. Implement Repair 6 production code (monotonicity gate)
2. Implement Repair 2 integration tests
3. Implement Repair 1 binding fix
4. Implement Repair 4, 5 (quick wins)
5. Fix Repair 3 (style pass)
