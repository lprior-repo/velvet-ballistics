# Architectural Drift Report: vb_cli_cross_crate_adversarial.rs

**File:** `crates/vb_cli/tests/cross_crate_adversarial.rs`
**Date:** 2026-05-29
**Agent:** architectural-drift

---

## Summary

| Metric | Value |
|--------|-------|
| Total Lines | **1509** |
| Test Count | **70** |
| Size Limit | 300 lines |
| Status | **DRIFT DETECTED** |

---

## Drift Analysis

### 1. Size Violation

**Severity: HIGH**

The file contains **1509 lines**, which **exceeds the 300-line limit** by 1209 lines (403% of limit).

This file is a monolithic integration test suite that tests cross-crate seams. It is organized into logical sections:

| Section | Lines | Description |
|---------|-------|-------------|
| Helpers | 1–57 | Test utilities and helpers |
| SEAM 1: yaml→validate | 60–141 | YAML parse error propagation |
| SEAM 2: validate→compile | 143–219 | Schema validation across seam |
| SEAM 3: compile→core | 221–286 | Compile boundary tests |
| SEAM 4: core→runtime | 288–476 | Engine execution tests |
| SEAM 5: runtime→storage | 478–607 | Storage/journal tests |
| SEAM 6: runtime→ipc | 609–820 | IPC wire protocol tests |
| Taint propagation | 822–886 | Cross-crate taint tracking |
| Error propagation | 888–963 | Error chain preservation |
| Resource limits | 965–1010 | Limit enforcement |
| Expression eval | 1012–1140 | Expression compilation/eval |
| Compile pipeline | 1142–1214 | End-to-end compilation |
| Diagnostic codes | 1216–1266 | Code parsing/validation |
| Serialization | 1268–1303 | Artifact round-trip |
| Type taint | 1305–1358 | Validation type checks |
| Runtime shard | 1360–1509 | Runtime integration |

---

## Recommendations

### REFACTOR REQUIRED

**The file MUST be split.** The 300-line hard limit is violated by 403%.

**Proposed Split Strategy:**

```
crates/vb_cli/tests/cross_crate_adversarial/
├── mod.rs                           # ~50 lines (header + re-exports)
├── seam1_yaml_parse_tests.rs        # ~90 lines (6 tests)
├── seam2_validate_schema_tests.rs   # ~80 lines (3 tests)
├── seam3_compile_boundary_tests.rs  # ~80 lines (4 tests)
├── seam4_core_runtime_tests.rs      # ~200 lines (4 tests)
├── seam5_storage_tests.rs           # ~140 lines (6 tests)
├── seam6_ipc_tests.rs               # ~220 lines (14 tests)
├── taint_propagation_tests.rs       # ~70 lines (5 tests)
├── error_propagation_tests.rs        # ~80 lines (4 tests)
├── resource_limit_tests.rs           # ~50 lines (5 tests)
├── expr_eval_tests.rs               # ~140 lines (4 tests)
├── compile_pipeline_tests.rs        # ~80 lines (5 tests)
├── diagnostic_code_tests.rs         # ~60 lines (2 tests)
├── serialization_tests.rs          # ~40 lines (1 test)
├── type_taint_tests.rs             # ~60 lines (3 tests)
└── runtime_shard_tests.rs           # ~160 lines (2 tests)
```

### Alternative: Module-based Organization

If file-per-seam is too granular, group by concern:

```
crates/vb_cli/tests/cross_crate_adversarial/
├── mod.rs                           # ~50 lines
├── yaml_seams.rs                    # ~170 lines (seams 1-2)
├── compile_seams.rs                 # ~170 lines (seam 3 + compile pipeline)
├── runtime_seams.rs                 # ~350 lines (seams 4-6)
├── taint_and_errors.rs              # ~200 lines
└── integration_seams.rs             # ~280 lines
```

---

## DDD Cohesion Check

| Criterion | Status | Notes |
|-----------|--------|-------|
| Single Responsibility | ⚠️ WARN | Tests multiple crate boundaries |
| Primitive Obsession | ✅ PASS | Uses typed IDs (RunId, StepIdx, SlotIdx) |
| Parse, Don't Validate | ✅ PASS | Error variants are exact types |
| State Transitions | N/A | This is a test file |

---

## Files Affected by Refactor

1. `crates/vb_cli/tests/cross_crate_adversarial.rs` → DELETE after split
2. `crates/vb_cli/tests/mod.rs` → UPDATE with new module tree

---

## Verification Command

After refactoring, verify:

```bash
# Check line counts
wc -l crates/vb_cli/tests/cross_crate_adversarial/**/*.rs

# Verify tests still discoverable
cargo test -p vb_cli --test cross_crate_adversarial -- --list
```

---

**STATUS: REFACTOR REQUIRED**
