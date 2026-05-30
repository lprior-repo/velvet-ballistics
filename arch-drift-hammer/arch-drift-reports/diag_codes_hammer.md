# ARCHITECTURAL DRIFT REPORT: `diag_codes.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/diag_codes.rs`  
**Total Lines:** 336  
**Threshold:** 300  
**Status:** 🚨 VIOLATES LINE COUNT RULE

---

## EXECUTIVE SUMMARY

| Issue | Severity | Category |
|-------|----------|----------|
| File exceeds 300 lines (336 lines, +12%) | CRITICAL | Line Count Violation |
| Primitive obsession: raw `u16` for all codes | HIGH | Type Safety |
| Duplicate code enumeration (constants + test vec) | MEDIUM | DRY Violation |
| No compile-time range enforcement | MEDIUM | Type Safety |
| Test module is 261 lines of repetitive boilerplate | MEDIUM | Code Bloat |

---

## 1. LINE COUNT VIOLATION

**Finding:** 336 lines total
- Lines 1-74: Constant definitions (74 lines)
- Lines 75-336: Test module (261 lines)
- **Over threshold by 36 lines (12% overflow)**

**Required Action:** Split into at minimum 2 files.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw `u16` For All Diagnostic Codes

All 58 diagnostic codes are defined as raw `u16` constants:

```rust
pub const CODE_DUPLICATE_KEY: u16 = 0x0101;
pub const CODE_UNKNOWN_REFERENCE: u16 = 0x0201;
// ... etc
```

**Problem:** There is zero type distinction between error categories. A function expecting `CODE_UNKNOWN_REFERENCE` (E02xx) can be passed `CODE_DUPLICATE_KEY` (E01xx) with no compile-time error.

**Scott Wlaschin Principle Violated:** "Make illegal states unrepresentable." The type system should prevent mixing error categories.

### 2.2 No Category Types

The codes are semantically grouped into 6 ranges:
- **E01xx** (0x01xx): Schema validation errors
- **E02xx** (0x02xx): Reference validation errors
- **E03xx** (0x03xx): Control-flow errors
- **E04xx** (0x04xx): Type/taint/resource errors
- **E05xx** (0x05xx): Gate verifier errors
- **E06xx** (0x06xx): Contract-discovery errors

**Problem:** This grouping exists only in comments and is enforced only by runtime tests (lines 199-329). The compiler does not know that a `SchemaCode` and a `ControlFlowCode` are different types.

### 2.3 No Newtype Wrappers

The file lacks:
- `struct DiagCode(u16)` — single newtype
- `struct SchemaCode(u16)` — category-specific newtypes
- Or an `enum DiagCategory { Schema, Reference, ControlFlow, TypeTaint, Gate, ContractDiscovery }` with associated code constants

**Ideal Refactoring:**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagCode(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaCode(pub DiagCode);
// ... or alternatively an enum-based approach
```

---

## 3. DUPLICATE CODE ENUMERATION

### 3.1 Constants vs Test Vector

The file defines codes twice:
1. As individual `pub const` declarations (lines 6-73)
2. As a `vec!` in the test `all_codes()` function (lines 80-179)

**DRY Violation:** If a developer adds a new constant but forgets to add it to `all_codes()`, the uniqueness test and range tests will silently miss it.

### 3.2 Silent Drift Risk

When adding a new diagnostic code:
1. Developer adds: `pub const CODE_NEW_ERROR: u16 = 0x010C;`
2. Developer runs tests
3. `all_codes()` still returns the old 58 codes
4. `code_count_matches_total` test FAILS
5. But `schema_codes_are_in_e01xx_range` still passes for old codes

**The test vector is the source of truth drift risk.**

---

## 4. RUNTIME-ONLY RANGE ENFORCEMENT

All category range validation happens in tests (lines 199-329) using bit manipulation:

```rust
let high = (code >> 8) & 0xFF;
assert_eq!(high, 0x01, "schema code {code:#06x} should be in E01xx range");
```

**Problems:**
- This runs only at test time, not compile time
- A bug in production code that uses the wrong code category won't be caught until tests run
- The hex ranges are implicit in the assertions, not explicit in types

**Compile-time alternative (not implemented):**
- Each category as a distinct type with a private constructor that validates the range
- Or const generics: `struct DiagCode<const RANGE: u8, const CODE: u16>` where only specific RANGE values are constructible

---

## 5. TEST MODULE BLOAT

The test module (lines 75-336) is 261 lines of repetitive patterns:

### 5.1 Structural Duplication

Each range test follows the exact same pattern:
```rust
#[test]
fn xxx_codes_are_in_eYYxx_range() {
    let category_codes = [...];  // hardcoded array
    for code in category_codes {
        let high = (code >> 8) & 0xFF;
        assert_eq!(high, 0xYY, "...");
    }
}
```

**Lines per test:** ~20-25 lines × 6 tests = ~150 lines of near-identical code

### 5.2 Potential Reduction

A parameterized approach could reduce this to ~30 lines:
```rust
#[test]
fn codes_respect_range_boundaries() {
    let categories = [
        (0x01, "schema", &[CODE_DUPLICATE_KEY, ...]),
        (0x02, "reference", &[CODE_UNKNOWN_REFERENCE, ...]),
        // ...
    ];
    for (range, name, codes) in categories {
        for code in *codes {
            assert_eq!((code >> 8) & 0xFF, range, "{name} code {code:#06x}...");
        }
    }
}
```

### 5.3 `all_codes()` Duplication

The `all_codes()` function manually lists every code name and value pair (lines 80-179). This is 100 lines of:
```rust
("CODE_NAME", CODE_NAME),
```
Which directly mirrors the constant definitions. This could be auto-generated via macro or reduced via const-based iteration.

---

## 6. RESPONSIBILITY MAPPING

| Lines | Responsibility | Assessment |
|-------|----------------|------------|
| 1-4 | Module header, feature flags | ✓ Clean |
| 6-17 | Schema validation error codes (E01xx) | ✓ Well-named constants |
| 19-23 | Reference validation error codes (E02xx) | ✓ Well-named constants |
| 25-33 | Control-flow error codes (E03xx) | ✓ Well-named constants |
| 36-47 | Type/taint/resource error codes (E04xx) | ✓ Well-named constants |
| 50-68 | Gate verifier error codes (E05xx) | ✓ Well-named constants |
| 71-73 | Contract-discovery error codes (E06xx) | ✓ Well-named constants |
| 76-336 | Test module | 🚨 Bloated, needs refactor |

**Observation:** The constant definitions themselves (lines 1-73) are clean and well-organized. The problem is entirely in the test module.

---

## 7. RECOMMENDED REFACTORING

### 7.1 File Split

**Option A (Minimal):**
```
diag_codes.rs        (lines 1-74, constants only)
diag_codes_tests.rs  (test module, move from lines 75-336)
```

**Option B (Preferred — with NewTypes):**
```
diag_codes.rs         (constants + basic newtype)
diag_codes/category.rs (enum + category types)
diag_codes/tests.rs    (leaner test module)
```

### 7.2 NewType Introduction

```rust
/// A diagnostic error code for the velvet-ballistics workflow engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagCode(pub u16);

impl DiagCode {
    pub const fn schema(v: u8) -> Self { Self(0x0100 | v) }
    pub const fn reference(v: u8) -> Self { Self(0x0200 | v) }
    pub const fn control_flow(v: u8) -> Self { Self(0x0300 | v) }
    pub const fn type_taint(v: u8) -> Self { Self(0x0400 | v) }
    pub const fn gate(v: u8) -> Self { Self(0x0500 | v) }
    pub const fn contract_discovery(v: u8) -> Self { Self(0x0600 | v) }

    pub const fn category(&self) -> u8 { (self.0 >> 8) & 0xFF }
}
```

### 7.3 Test Reduction

Replace 261-line test module with:
- Property-based test using `proptest` to check range invariants
- Single parameterized test for all range checks
- `quickcheck` or const-based compile-time verification where possible

---

## 8. VERDICT

| Category | Finding | Severity |
|----------|---------|----------|
| Line Count | 336 > 300 | CRITICAL |
| Primitive Obsession | Raw `u16` | HIGH |
| Category Type Safety | None | HIGH |
| DRY | Double enumeration | MEDIUM |
| Range Enforcement | Runtime only | MEDIUM |
| Test Bloat | 261 repetitive lines | MEDIUM |

**OVERALL: REFACTOR REQUIRED**

The constant definitions are high quality. The test module is the primary offender — it needs splitting and could benefit from parameterized testing. The type system should be extended to prevent cross-category code misuse at compile time.

---

*Report generated: 2026-05-29*  
*Agent: architectural-drift*  
*Repository: velvet-ballistics*
