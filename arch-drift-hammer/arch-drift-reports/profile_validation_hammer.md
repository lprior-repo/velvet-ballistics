# ARCH-DRIFT REPORT: profile_validation.rs

**File**: `crates/vb_yaml/src/profile_validation.rs`  
**Line Count**: 374 (EXCEEDS 300-LINE LIMIT BY 74 LINES)  
**Status**: 🔨 HAMMER REQUIRED

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 374 | 300 | ❌ OVER |
| Excess | +74 | 0 | ❌ VIOLATION |

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### 2.1 Raw Numeric Types (Unwrapped Domain Primitives)

| Location | Primitive | Problem | Remediation |
|----------|-----------|---------|-------------|
| L63 | `depth: u16` | Raw depth counter without `Depth` wrapper | `struct Depth(u16)` with `impl Depth` |
| L64 | `node_count: u32` | Raw node counter without `NodeCount` wrapper | `struct NodeCount(u32)` |
| L65 | `document_count: usize` | Raw doc counter without `DocumentCount` wrapper | `struct DocumentCount(usize)` |
| L70-73 | `Vec<usize>` stacks | Primitive vectors tracking state | `struct DepthStack<T>` generic wrapper |
| L135-146 | `counter.checked_add(1)` | Raw `usize` arithmetic | Typed counter with `increment()` method |

### 2.2 Primitive Parameter Drilling

| Function | Parameter | Problem |
|----------|------------|---------|
| `check_source_size(text, max_bytes: usize)` | `usize` | Should be `MaxBytes(usize)` |
| `check_scalar_length(value, max_bytes: usize)` | `usize` | Should be `MaxScalarBytes(usize)` |
| `check_scalar_ambiguity(events)` | Raw `&[YamlEvent]` slice | Should accept `impl IntoIterator<Item=&YamlEvent>` |

### 2.3 Raw Stringly-Typed Error Details

| Location | Anti-pattern |
|----------|--------------|
| L254 | `detail: "null_byte_in_scalar"` — stringly-typed |
| L264 | `detail: "null_byte_in_source"` — stringly-typed |
| L274 | `is_allowed_tag(tag: &str)` — `&str` tag instead of `Tag` type |

---

## 3. DDD COHESION VIOLATIONS

### 3.1 God Function: `collect_and_validate_events`

**Lines 57-236 (180 lines)** — This single function violates SRP by混 with:

1. **Event parsing** — parser interaction
2. **Depth tracking** — nesting level enforcement
3. **Node counting** — total node enumeration
4. **Document counting** — multi-doc detection
5. **Collection entry counting** — seq/map size tracking
6. **Key/value state machine** — expecting_key stack
7. **Content detection** — found_content flag
8. **Event conversion** — `convert_event` call

**Each concern should be a separate type/function with single responsibility.**

### 3.2 Duplicate Null-Byte Checking

| Function | Lines | Violation |
|----------|-------|-----------|
| `check_null_bytes` | 251-258 | Almost identical to `check_null_bytes_in_source` |
| `check_null_bytes_in_source` | 261-268 | Almost identical to `check_null_bytes` |

**Refactor**: Single `contains_null_byte(&str) -> bool` utility used by both.

### 3.3 Duplicate Nesting Depth Check

**Lines 92-102 AND 108-118**: Identical depth overflow check duplicated for `MappingStart` and `SequenceStart`.

**Refactor**: Extract to `check_depth_limit(depth: u16, max: u16) -> YamlResult<()>`.

### 3.4 Duplicate NodeLimitExceeded Error Construction

Lines 137, 163, 185, 201, 213 — identical pattern:

```rust
.ok_or(YamlError::NodeLimitExceeded {
    count: u32::MAX,  // WRONG: should be actual count
    max: limits.max_nodes,
})?;
```

**Bug**: All use `u32::MAX` as count instead of the actual counter value.

---

## 4. BOUNDARY VIOLATIONS

### 4.1 Leaky Abstraction: `saphyr_parser` Coupling

**Lines 61, 82-86, 92, 108, 123, 178, 194, 224**

The module directly references `saphyr_parser::Parser`, `saphyr_parser::Event::*` — violating hexagonal boundary. Should wrap in `YamlParser` trait/abstraction.

### 4.2 Missing Value Object: `Depth`

**Current**: `u16` passed through multiple functions  
**Should be**: `struct Depth(u16)` with bounded constructor `Depth::new(u16) -> YamlResult<Depth>`

### 4.3 Missing Value Object: `NodeCount`

**Current**: `u32` raw counter  
**Should be**: `struct NodeCount(u32)` with `increment() -> YamlResult<NodeCount>`

---

## 5. REFACTORING prescription

### Minimum Viable Fix (Reduce to ≤300 lines)

| Action | Lines Saved |
|--------|-------------|
| Extract `check_depth_limit` helper | ~8 |
| Extract `check_node_limit` helper | ~8 |
| Merge duplicate null-byte functions | ~6 |
| Extract `Tag` type with `is_allowed()` | ~10 |
| Extract `Depth`, `NodeCount`, `DocumentCount` wrappers | ~15 |
| Extract `YamlProfileCollector` struct (isolate state) | ~25 |
| **Total** | **~72** |

### Target Structure After Hammer

```
profile_validation.rs          (~150 lines)
├── validate_yaml_profile
├── validate_yaml_profile_with_limits
└── calls:
    ├── size validation (extracted)
    ├── depth validation (extracted)  
    ├── node counting (extracted to YamlProfileCollector)
    ├── forbidden features (extracted)
    ├── dup keys (already separate module)
    └── scalar ambiguity (extracted)

profile_validation_types.rs    (~100 lines, NEW)
├── struct Depth(u16)
├── struct NodeCount(u32)
├── struct DocumentCount(usize)
├── struct MaxBytes(usize)
├── struct MaxScalarBytes(usize)
├── enum ForbiddenDetail { NullByteInScalar, NullByteInSource }
└── impl blocks with bounded constructors
```

---

## 6. VERDICT

**ARCHITECTURAL DRIFT**: ❌ CONFIRMED  
**PRIMITIVE OBSESSION**: ❌ 12+ violations  
**DDD COHESION**: ❌ God function, SRP violations  
**LINE COUNT**: ❌ 374 > 300  

**RECOMMENDATION**: Full hammer. Extract types first, then split functions. The duplicate error construction with `u32::MAX` is a BONUS BUG found during drift analysis.
