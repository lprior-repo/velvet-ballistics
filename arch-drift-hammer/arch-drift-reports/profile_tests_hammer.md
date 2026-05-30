# Architectural Drift Report: `profile_tests.rs`

**File:** `crates/vb_yaml/src/profile_tests.rs`
**Total Lines:** 570 (violates 300-line mandate by 90%)
**Drift Severity:** CRITICAL

---

## Executive Summary

`profile_tests.rs` is **570 lines** — nearly **double** the 300-line architectural ceiling. This file demonstrates severe **Test Leviathan** pattern: an oversized, repetitive test file that has not been decomposed into focused test modules. The file also exhibits **widespread primitive obsession** — tests pass raw strings, numbers, and slices where domain types should be used.

---

## 1. Line Count Violation

| Metric | Value | Limit | Violation |
|--------|-------|-------|-----------|
| Total Lines | 570 | 300 | +90% over |
| Test Functions | ~40 | — | — |
| Helper Functions | 4 | — | — |

**Required Action:** Decompose into at minimum 2 focused test modules:
- `profile_validation_tests.rs` — empty source, single/multiple docs, anchors, ambiguous scalars
- `profile_limits_tests.rs` — depth, size, scalar, node, sequence, mapping limits
- `profile_helpers.rs` — shared test builders

---

## 2. Responsibility Map

| Responsibility | Lines | Test Count |
|----------------|-------|------------|
| Empty source validation | 17-21, 132-136, 464-468 | 3 |
| Single document acceptance | 24-27, 139-143 | 2 |
| Multiple document rejection | 30-34, 146-150, 361-380 | 5 |
| Anchor/alias/merge rejection | 37-41, 153-157, 339-358 | 4 |
| Ambiguous scalar rejection (yes/no/on/off/y/n) | 44-47, 160-205, 383-419 | ~15 |
| Duplicate key detection | 64-88, 298-309, 422-433 | ~10 |
| Depth limit enforcement | 91-103, 222-240 | 2 |
| Source size limit | 106-114, 243-257, 436-447 | 4 |
| Scalar length limit | 117-126, 260-275 | 2 |
| Node limit enforcement | 278-295 | 1 |
| Sequence length limit | 515-548 | 2 |
| Mapping size limit | 521-569 | 2 |
| Forbidden features (custom tags) | 312-335, 471-507 | ~4 |
| Nested mapping/sequence acceptance | 450-461 | 2 |

---

## 3. Primitive Obsession Violations

### 3.1 Raw String Keys Instead of `YamlKey` Type

**Violations (examples):**
```rust
// Line 65-67
let keys = vec!["a", "b", "a"];  // Raw &str slice
let result = reject_duplicate_keys(&keys);
assert!(matches!(result, Err(YamlError::DuplicateKey { key }) if key.as_ref() == "a"));

// Line 86-87
let keys = vec!["a", "b", "c"];
assert_eq!(reject_duplicate_keys(&keys), Ok(()));

// Line 299-301
let keys = vec!["a", "b", "a"];
assert_eq!(result, Err(YamlError::DuplicateKey { key: "a".into() }));
```

**Domain Gap:** The domain has `DuplicateKey { key: Box<str> }` but callers still use raw `&[&str]`. Should have `YamlKey` wrapper or `KeySet` type.

### 3.2 Magic Numbers Without Domain Constants

**Violations:**
```rust
// Line 93-96 — 70 levels of nesting, limit 10
for i in 0..70 {
    let indent = "  ".repeat(i);
    yaml.push_str(&format!("{indent}b:\n"));
}

// Line 98-99
YamlLimits { max_depth: 10, .. }  // Hardcoded 10

// Line 224-226 — 15 levels, limit 10
for i in 0..15 {
    let indent = "  ".repeat(i);

// Line 234-236 — raw assertion: depth > 10
assert!(depth > 10);  // Why 10? Where is this documented?

// Line 270 — len > 50
assert!(len > 50);  // Magic number 50

// Line 280-282 — 20 items
for i in 0..20 {
    yaml.push_str(&format!("  key{i}: val{i}\n"));

// Line 284-285
YamlLimits { max_nodes: 5, .. }  // Hardcoded 5

// Line 515-517 — 10,000 items
(0..count).map(|i| format!("  - item{}\n", i))

// Line 529 — exact 10,000
let yaml = yaml_with_sequence_items(10_000);

// Line 531 — 10,001
let yaml = yaml_with_sequence_items(10_001);

// Line 553 — 1,024 entries
yaml_with_mapping_entries(1_024);

// Line 564 — 1,025 entries
yaml_with_mapping_entries(1_025);
```

**Domain Gap:** `YamlLimits::default()` defines the real constants (10_000, 1_024) but tests scatter hardcoded values. These should use the actual `YamlLimits` constants.

### 3.3 Raw `usize` for Limits Instead of Domain Wrappers

```rust
// Line 107-108 — raw byte count
let big = "x".repeat(2_000_000);
let limits = YamlLimits { max_source_bytes: 1_000_000, .. };

// Line 118-119
let long_scalar = "x".repeat(100);
let limits = YamlLimits { max_scalar_bytes: 50, .. };

// Line 244-246
let big = "x".repeat(200);
let limits = YamlLimits { max_source_bytes: 100, .. };

// Line 261-262
let long_scalar = "x".repeat(100);
let limits = YamlLimits { max_scalar_bytes: 50, .. };
```

**Domain Gap:** Tests use raw `usize` values instead of named limit types. Should use `SourceSize::from_bytes(1_000_000)` or similar.

### 3.4 Raw Ambiguous Scalar Strings

```rust
// Lines 383-419 — repeated raw string literals
let scalars = vec!["yes"];
let result = reject_yaml_1_1_ambiguous_scalars(&scalars);

let scalars = vec!["y"];
let scalars = vec!["n"];
let scalars = vec!["true"];  // This one passes

// Lines 160-205 — identical repeated pattern
Err(YamlError::AmbiguousScalar { scalar: "yes".into() })
Err(YamlError::AmbiguousScalar { scalar: "no".into() })
Err(YamlError::AmbiguousScalar { scalar: "on".into() })
Err(YamlError::AmbiguousScalar { scalar: "off".into() })
```

**Domain Gap:** `AmbiguousScalar` is a domain error variant containing `Box<str>`. Tests pass raw `&str` instead of using a `YamlScalar` wrapper type.

### 3.5 YAML Construction via String Primitives

```rust
// Lines 515-518 — inline YAML building via string concat
fn yaml_with_sequence_items(count: usize) -> String {
    let items: Vec<String> = (0..count).map(|i| format!("  - item{}\n", i)).collect();
    format!("items:\n{}", items.join(""))
}

// Lines 521-526
fn yaml_with_mapping_entries(count: usize) -> String {
    let entries: Vec<String> = (0..count)
        .map(|i| format!("  key{}: value{}\n", i, i))
        .collect();
    format!("root:\n{}\n", entries.join(""))
}

// Lines 92-96 — nested YAML construction
let mut yaml = String::from("a:\n");
for i in 0..70 {
    let indent = "  ".repeat(i);
    yaml.push_str(&format!("{indent}b:\n"));
}
```

**Domain Gap:** No `YamlBuilder` or `TestYaml` helper type. Tests manipulate raw strings instead.

---

## 4. Test Code Pattern Violations

### 4.1 Redundant Helper Macro

```rust
// Lines 7-15 — strange assertion helper
fn assertion_failed(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(assertion_failed(format_args!($($arg)*)), $($arg)*)
    };
}
```

**Issue:** This macro always fails (assertion_failed returns false). Tests use this instead of `panic!` or proper assertion helpers. This is dead code behavior.

### 4.2 Duplicate Test Coverage

The same error variants are tested multiple times with slightly different assertions:

| Error Type | Appears In Tests |
|------------|------------------|
| `EmptySource` | `empty_source_rejected`, `empty_source_returns_empty_source_error`, `reject_whitespace_only` |
| `MultipleDocuments` | `multiple_documents_rejected`, `multiple_documents_returns_exact_count`, `reject_multiple_documents_rejects_two_docs`, `unsupported_yaml_features_return_typed_diagnostics` |
| `AnchorAliasMerge` | `anchor_rejected`, `anchor_rejected_exact`, `reject_anchors_aliases_merges_rejects_anchor`, `unsupported_yaml_features_return_typed_diagnostics` |
| `AmbiguousScalar` | 6 separate tests for yes/no/on/off/y/n variants |

---

## 5. Scott Wlaschin DDD Violations

| DDD Principle | Violation |
|--------------|-----------|
| **Make illegal states unrepresentable** | Raw `Vec<&str>` for keys allows any string, not validated key format |
| **Value objects for primitives** | No `YamlSource`, `YamlKey`, `DepthLimit`, `SourceSize` types — all raw primitives |
| **Expressive function signatures** | `reject_duplicate_keys(&keys)` takes `&[&str]` instead of `&KeySet` |
| **Domain types over primitives** | `max_depth: 10` is `u16`/`usize`, not `DepthLimit(N)` |

---

## 6. Refactoring Prescription

### 6.1 Required Splits (Minimum)

```
vb_yaml/src/
  profile_tests.rs          # 570 lines → SPLIT NEEDED
  profile_validation_tests/ # NEW MODULE directory
    mod.rs                  # 50 lines — re-exports
    empty_source_tests.rs    # ~80 lines
    document_tests.rs       # ~80 lines  
    anchor_tests.rs         # ~60 lines
    scalar_tests.rs         # ~100 lines
    key_tests.rs            # ~60 lines
  profile_limits_tests/     # NEW MODULE directory
    mod.rs                  # 50 lines
    depth_tests.rs          # ~60 lines
    size_tests.rs           # ~80 lines
    node_tests.rs           # ~60 lines
    sequence_tests.rs       # ~60 lines
    mapping_tests.rs        # ~60 lines
  profile_test_helpers.rs   # ~100 lines — shared builders
```

### 6.2 Domain Types to Introduce

```rust
// In vb_yaml/src/domain/ (new module)
pub struct YamlSource(Box<str>);

pub struct YamlKey(Box<str>);

pub struct KeySet(Vec<YamlKey>);

pub struct DepthLimit(u16);
pub struct SourceSizeLimit(usize);
pub struct ScalarLengthLimit(usize);
pub struct NodeCountLimit(u32);

pub enum AmbiguousScalarVariant {
    Yes, No, On, Off, Y, N,
}
```

### 6.3 Helper Consolidation

```rust
// profile_test_helpers.rs
pub struct YamlBuilder {
    content: String,
}

impl YamlBuilder {
    pub fn new() -> Self;
    pub fn with_sequence_items(mut self, count: usize) -> Self;
    pub fn with_mapping_entries(mut self, count: usize) -> Self;
    pub fn with_nested_depth(mut self, depth: usize) -> Self;
    pub fn build(self) -> String;
}

pub const TEST_LIMITS: YamlLimits = YamlLimits {
    max_depth: 10,
    max_source_bytes: 100,
    max_scalar_bytes: 50,
    max_nodes: 5,
    max_sequence_len: 10_000,
    max_mapping_entries: 1_024,
};
```

---

## 7. Verification Checklist

- [ ] File reduced to ≤300 lines (target: 250 for margin)
- [ ] No test passes raw `&[&str]` — must use `KeySet`
- [ ] No raw `usize`/`u16` for limits — must use domain limit types
- [ ] No magic numbers — all extracted to named constants
- [ ] Helper functions extracted to `profile_test_helpers.rs`
- [ ] No duplicate coverage of same error variant
- [ ] `fail_assert!` macro eliminated

---

## 8. Severity Assessment

| Issue | Severity | Effort to Fix |
|-------|----------|---------------|
| Line count (570 >> 300) | **CRITICAL** | High — requires module decomposition |
| Primitive obsession | **HIGH** | Medium — add domain wrapper types |
| Redundant test coverage | **MEDIUM** | Low — consolidate duplicates |
| Dead `fail_assert!` macro | **LOW** | Trivial — remove |

**Recommended Action:** File is architecturally non-compliant. Requires decomposition before any new features can be added to this module.
