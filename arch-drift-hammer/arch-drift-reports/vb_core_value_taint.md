# Architectural Drift Report: `vb_core/src/value.rs`

## File Status: NON-Existent Path
- **Requested Path**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/value/taint.rs`
- **Actual Path**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/value.rs`
- **Note**: No `value/` subdirectory exists. `Taint` is defined in `value.rs`.

---

## 1. Line Count Analysis

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | **1253** | 300 | ❌ VIOLATION |

**The file is 4.2x over the 300-line limit.**

---

## 2. DDD Cohesion Analysis

### Multiple Domain Concepts in Single File

`value.rs` contains **5 distinct domain concepts** crammed into one file:

| Concept | Type | DDD Role |
|---------|------|----------|
| `Taint` | enum + function | Value Object - secret propagation marker |
| `FiniteF64` | newtype | Value Object - validated floating point |
| `SlotValue` | enum | Entity - runtime slot handle |
| `ConstValue` | enum | Value Object - compile-time constants |
| `SlotValueDisplay` | struct | Service - display formatting |

**DDD Smell**: LOW cohesion - "Everything Value" antipattern

### `join_taint` Placement Issue
- `join_taint` is a standalone function rather than `impl Taint { fn join(self, other: Taint) -> Taint }`
- Violates "type-owned operations" principle

---

## 3. Violations

| # | Violation | Severity | Description |
|---|-----------|----------|-------------|
| 1 | **File size exceeded** | CRITICAL | 1253 lines vs 300 line limit (318% of threshold) |
| 2 | **Low DDD cohesion** | HIGH | 5+ distinct domain concepts in single file |
| 3 | **Operation not on type** | MEDIUM | `join_taint` should be `Taint::join(self, other)` |
| 4 | **Non-exhaustive enums** | LOW | `Taint`, `SlotValue`, `ConstValue` are `#[non_exhaustive]` - may cause consumer breakage |

---

## 4. Recommendations

### Immediate (Refactor Required)
Split `value.rs` into multiple modules:

```
src/value/
├── mod.rs          # Re-exports
├── taint.rs        # Taint enum + join_taint
├── finite_f64.rs   # FiniteF64 newtype
├── slot_value.rs   # SlotValue + ConstValue
└── display.rs      # SlotValueDisplay service
```

### Priority
| Priority | Action | Effort |
|----------|--------|--------|
| **P0** | Split file to satisfy 300-line rule | High |
| P1 | Move `join_taint` to `impl Taint` | Low |
| P2 | Review `#[non_exhaustive]` usage | Medium |

---

## 5. Evidence

```
$ wc -l value.rs
   1253 value.rs
```

---

**STATUS**: ❌ REQUIRES REFACTORING

**Priority**: P0 - File must be split before further analysis.
