# Architectural Drift Report: `directive.rs`

**File**: `crates/vb_runtime/src/shard/directive.rs`
**Line Count**: 473 (VIOLATION: exceeds 300-line limit by 173 lines)
**Assessed**: 2026-05-29
**Severity**: CRITICAL

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 473 | 300 | ❌ OVER BY 173 |
| Production code | ~107 | — | — |
| Test code | ~366 | — | — |
| Test-to-code ratio | 3.4:1 | — | ⚠️ EXCESSIVE |

The test module (lines 108–473) is **360 lines** of repetitive, variant-by-variant
assertion tests. This is not behavior discovery — it is combinatorial coverage theater.
The entire test block should be replaced by a 40-60 line parameterized test suite.

---

## 2. PRIMITIVE OBSESSION VIOLATION

### VIOLATION 1: `target: u32` in `Migrate` variant

**Location**: Lines 51–54
```rust
Migrate {
    /// Target shard index to migrate commands to.
    target: u32,
},
```

**Problem**: `u32` is a primitive. A shard index is a **domain concept** with:
- Valid range (typically `0..shard_count`, not all of `u32`)
- Invariants (e.g., `target != source_shard`)
- Semantic meaning in the migrate operation

**Scott Wlaschin DDD Violation**: Type-driven design requires making illegal states
unrepresentable. `u32` permits `target: u32::MAX`, `target: 0`, and `target: 1`
indiscriminately. There is no `ShardIndex` newtype to enforce domain rules.

**Required Fix**:
```rust
// NEW TYPE (in vb_core or vb_boundary_inventory)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct ShardIndex(u32);

impl ShardIndex {
    pub const fn new(inner: u32) -> Self { Self(inner) }
    pub const fn index(self) -> u32 { self.0 }
    pub const fn is_valid(self, max_shards: u32) -> bool { self.0 < max_shards }
}

impl From<ShardIndex> for u32 { ... }
impl From<u32> for ShardIndex { ... } // with bounds check

// REPLACED VARIANT
Migrate {
    target: ShardIndex,  // was u32
},
```

**Side Benefit**: Enables `ShardIndex` to carry its own validation logic rather than
scattering `target < max_shards` checks across the runtime.

---

## 3. SHARD DIRECTIVE RESPONSIBILITY MAP

| Responsibility | Location | Assessment |
|----------------|----------|------------|
| Enum definition | Lines 13–61 | ✅ Clean, well-documented |
| `allows_admission()` | Lines 72–75 | ✅ Pure predicate, no side effects |
| `completes_current_work()` | Lines 85–88 | ✅ Pure predicate |
| `has_migration_target()` | Lines 93–96 | ✅ Pure predicate |
| `is_alive()` | Lines 102–105 | ✅ Pure predicate |
| Unit tests | Lines 108–473 | ❌ 360 lines of repetitive variant coverage |

---

## 4. TEST BOILERPLATE ANALYSIS

Every test in the block follows the same pattern:
```
#[test]
fn shard_directive_<variant>_<property>() {
    assert!(ShardDirective::<variant>.<method>());
}
```

This is **combinatorial inflation**, not behavior coverage. Examples:
- `shard_directive_continue_is_alive` (line 450)
- `shard_directive_suspend_is_alive` (line 455)
- `shard_directive_cancel_is_alive` (line 460)
- `shard_directive_barrier_is_alive` (line 465)
- `shard_directive_migrate_is_alive` (line 470)

Five tests to cover one `is_alive()` method with one line of logic:
```rust
pub fn is_alive(&self) -> bool {
    !matches!(self, Self::Shutdown)
}
```

**Recommended replacement** (one test per method):
```rust
#[test]
fn shard_directive_methods() {
    let all = [
        ShardDirective::Continue,
        ShardDirective::Suspend,
        ShardDirective::Cancel,
        ShardDirective::Barrier,
        ShardDirective::Migrate { target: ShardIndex::new(0) },
        ShardDirective::Shutdown,
    ];

    // is_alive: only Shutdown is dead
    for d in &all[..5] { assert!(d.is_alive(), "{d:?} should be alive"); }
    assert!(!ShardDirective::Shutdown.is_alive());

    // allows_admission: only Continue
    assert!(ShardDirective::Continue.allows_admission());
    for d in &[Suspend, Cancel, Barrier, Shutdown] { assert!(!d.allows_admission()); }
}
```

---

## 5. DDD COHESION ASSESSMENT

| DDD Concept | Present? | Notes |
|-------------|----------|-------|
| Value Object | ❌ | `ShardIndex` missing — `u32` is raw |
| Enum (Status/State) | ✅ | `ShardDirective` is a proper state enum |
| Pure domain methods | ✅ | All four methods are side-effect free |
| No primitives in public API | ❌ | `u32` leaks through `Migrate { target }` |

---

## 6. SUMMARY SCORECARD

| Check | Status |
|-------|--------|
| File < 300 lines | ❌ 473 lines |
| No primitive obsession | ❌ `u32` for shard index |
| Tests not excessive | ❌ 360-line test block |
| DDD types over primitives | ❌ `ShardIndex` missing |
| Public API type-safe | ❌ `Migrate { target: u32 }` |

---

## 7. MANDATORY REFACTORS (in order)

1. **Create `ShardIndex` newtype** in `vb_core` (or appropriate domain crate):
   - `struct ShardIndex(u32)` with `const fn new(u32) -> Self`
   - `const fn index(self) -> u32`
   - `From<u32>` implementation (unchecked, for internal use)
   - `const MIN: ShardIndex = ShardIndex(0);`

2. **Replace `target: u32` with `target: ShardIndex`** in `Migrate` variant.

3. **Collapse test block to ≤60 lines** using parameterized iteration over variant arrays.

4. **Target line count**: ≤250 lines after refactor (production + tests).

---

## 8. FILES AFFECTED BY REFACTOR

| File | Change |
|------|--------|
| `crates/vb_core/src/shard.rs` (or similar) | Add `ShardIndex` newtype |
| `crates/vb_runtime/src/shard/directive.rs` | Replace `u32` with `ShardIndex`, trim tests |
| Any `Runtime::tick_shard` call-sites | Update `Migrate { target: N }` to `Migrate { target: ShardIndex::new(N) }` |

---

*Architectural drift confirmed. Hammer applied.*
