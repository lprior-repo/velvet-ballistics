# Architectural Drift Report: `vb_runtime/src/shard/directive.rs`

## 1. Line Count

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | **473** | 300 | **VIOLATION (+173)** |

---

## 2. DDD Cohesion Analysis

### Domain Type: `ShardDirective` enum

**Cohesion: ACCEPTABLE** — The enum correctly models a **workflow state machine** (Continue → Suspend → Barrier/Migrate → Shutdown/Cancel). Methods are proper **query methods** (`allows_admission`, `completes_current_work`, `has_migration_target`, `is_alive`) following "Parse, don't validate" principles.

### DDD Violations Detected

| Violation | Severity | Location |
|-----------|----------|----------|
| **Primitive Obsession** — `u32` for `target` shard index | MEDIUM | Line 53 |
| **Oversized Test Module** — 365 test lines vs 108 impl lines | HIGH | Lines 108–473 |
| **Feature Envy in Tests** — Tests reach into internal representation | LOW | Multiple `assert_eq!(format!("{directive:?}")` |

---

## 3. Violation Details

### V1: Primitive Obsession — `u32` for shard index

```rust
Migrate {
    target: u32,  // Line 53 — should be ShardIndex(u32) NewType
},
```

**Problem**: Raw `u32` allows invalid values (e.g., `u32::MAX`). The domain concept "shard index" should be a **NewType** wrapper.

**Fix**: Create `ShardIndex(u32)` wrapper with validation.

### V2: Test Module Bloat

- **Implementation**: 108 lines (lines 1–107)
- **Tests**: 365 lines (lines 108–473)
- **Ratio**: 3.4:1 test-to-impl

**Problem**: Tests test exhaustively but duplicate what `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` guarantees. The `is_copy` and `equality` tests for each variant are redundant.

---

## 4. DDD Smell Assessment

| Smell | Present | Notes |
|-------|---------|-------|
| Primitive Obsession | **YES** | `u32` for `target` |
| Anemic Domain Model | NO | Methods are behaviorful queries |
| Feature Envy | LOW | Tests probe Debug format internals |
| Temporal Coupling | NO | No ordered initialization requirements |

---

## 5. Priority Remediation

| Priority | Action | Effort |
|----------|--------|--------|
| **P1** | Split test module into separate file `directive_tests.rs` | Low |
| **P2** | Create `ShardIndex(u32)` NewType for `Migrate.target` | Medium |

---

## 6. Summary

```
Lines:     473 / 300  ❌ VIOLATION
DDD:       Cohesive domain enum with primitive obsession on Migrate.target
Smell:     Primitive obsession (u32), oversized test module
Priority:  P1 — Extract tests to separate file
```

**STATUS**: `NEEDS_REFACTOR`
