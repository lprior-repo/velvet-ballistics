# Architectural Drift Report: `value.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_core/src/value.rs`
**Total Lines**: 1253
**Line Limit**: 300
**Violation Ratio**: 4.18x OVER LIMIT

---

## 1. LINE COUNT VIOLATION (CRITICAL)

The file is **1253 lines**, exceeding the 300-line threshold by **953 lines**.

### Breakdown by Section

| Section | Lines | Type |
|---------|-------|------|
| `Taint` enum + `join_taint` | 14–45 | Core type |
| `FiniteF64` newtype | 47–120 | Core type |
| `SlotValue` enum | 122–174 | Core type |
| `proptests` module | 191–238 | Tests (~47 lines) |
| `mod tests` | 240–1140 | Tests (~900 lines) |
| `SlotValue` impl | 1142–1174 | Implementation |
| `SlotValueDisplay` | 1176–1253 | Display helper |

**Root Cause**: 947 lines of inline tests (~76% of file) are compressed into a single module.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### Violation A: `SlotValue::I64(i64)` — Unwrapped Integer

```rust
I64(i64),
```

**Problem**: `i64` is a raw primitive used to represent "deterministic arithmetic scaffolding." This is classic primitive obsession — the semantic intent (an integer value in the runtime's type system) is lost.

**DDD Violation**: In Scott Wlaschin's "making illegal states unrepresentable" doctrine, this should be wrapped:

```rust
// Suggested: newtype wrapper
pub struct RuntimeInteger(i64);

impl RuntimeInteger {
    pub fn new(value: i64) -> Self { Self(value) }
    pub fn get(self) -> i64 { self.0 }
}
```

**Current Severity**: MEDIUM — `i64` is not inherently wrong, but lacks domain semantics.

---

### Violation B: `SlotValue::Bool(bool)` — Bare Boolean

```rust
Bool(bool),
```

**Problem**: While `bool` is semantically clear, wrapping it in a newtype would make the type system more explicit about boolean values in the slot model.

**Current Severity**: LOW — `bool` is self-documenting in this context.

---

### GOOD: `FiniteF64` — Proper Newtype ✓

```rust
F64(FiniteF64),
```

The `FiniteF64` wrapper is **exemplary DDD**:
- Rejects NaN and infinity at construction
- Zero-dependency implementation
- Validates in both debug AND release builds
- `new()` returns `CoreResult<Self>` — parse, don't validate

---

### GOOD: Handle Types ✓

```rust
Symbol(SymbolId),
List(ListId),
Object(ObjectId),
Blob(BlobId),
```

All handle types use proper newtypes from `crate::ids`. No violation.

---

## 3. DDD STRUCTURAL VIOLATIONS

### Violation C: Monolithic File Organization

The file contains **4 distinct domain concepts** crammed into one file:

1. **Taint** — Secret propagation lattice (14–45)
2. **FiniteF64** — Validated floating-point (47–120)
3. **SlotValue/ConstValue** — Runtime value model (122–189, 1142–1174)
4. **SlotValueDisplay** — Display helper (1176–1253)

Each of these should be its own file in a `value/` directory:

```
value/
├── mod.rs          (re-exports)
├── taint.rs        (~50 lines)
├── finite_f64.rs   (~80 lines)
├── slot_value.rs   (~150 lines + impl)
├── const_value.rs (~50 lines)
└── display.rs     (~80 lines)
```

### Violation D: Inline Tests Bloating Production Module

947 lines of tests (76% of file) are embedded inline:

```
mod proptests { ... }   // 47 lines
mod tests { ... }        // 900 lines
```

**DDD Principle Violated**: Tests are not part of the domain model. They should live in:
- `tests/` directory at repository root (workspace_tests)
- Or `value/` directory if crate-level

---

## 4. RECOMMENDED REFACTORING

### Phase 1: File Splitting

| New File | Content | Est. Lines |
|----------|---------|------------|
| `value/taint.rs` | `Taint` enum + `join_taint` | ~50 |
| `value/finite_f64.rs` | `FiniteF64` newtype | ~80 |
| `value/slot_value.rs` | `SlotValue` enum + impl | ~150 |
| `value/const_value.rs` | `ConstValue` enum + conversion | ~50 |
| `value/display.rs` | `SlotValueDisplay` | ~80 |
| `value/mod.rs` | Re-exports | ~20 |
| **TOTAL** | | **~430** |

Move tests to `crates/vb_core/tests/value_*` or `crates/workspace_tests/`.

### Phase 2: Primitive Obsession Fixes

Consider wrapping `i64` in `SlotValue::I64`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct RuntimeInteger(i64);

impl RuntimeInteger {
    pub const fn new(value: i64) -> Self { Self(value) }
    pub const fn get(self) -> i64 { self.0 }
}
```

Then update `SlotValue::I64(RuntimeInteger)`.

---

## 5. SUMMARY

| Issue | Severity | Action Required |
|-------|----------|-----------------|
| Line count 1253 > 300 | **CRITICAL** | Split file into `value/` module |
| `i64` primitive obsession | MEDIUM | Consider `RuntimeInteger` wrapper |
| `bool` in `SlotValue::Bool` | LOW | Acceptable, optional wrapper |
| Inline tests (947 lines) | HIGH | Move to `tests/` directory |
| Monolithic file | HIGH | Create `value/` directory |

---

## 6. VERDICT

**STATUS: MUST REFACTOR**

The file violates both the 300-line hard limit (by 4x) and DDD principles by cramming 4 distinct domain concepts into one file with 76% test code.

**Required Actions**:
1. Create `crates/vb_core/src/value/` directory
2. Split into separate files per domain concept
3. Move tests out of production module
4. Update `lib.rs` to use new module structure
5. Optionally wrap `i64` in `RuntimeInteger` newtype

---
*Report generated by architectural-drift enforcer*
