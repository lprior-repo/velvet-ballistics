# Architectural Drift Report: `vb_validate/src/lib.rs`

**File**: `crates/vb_validate/src/lib.rs`  
**Total Lines**: 471 (❌ EXCEEDS 300-line limit)  
**Priority**: HIGH

---

## 1. Line Count Violation

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total lines | 471 | 300 | ❌ EXCEEDS |

---

## 2. DDD Cohesion Analysis

### Bounded Context Mapping

The file exposes **6 distinct bounded contexts** through a single monolithic `ValidationError` enum:

| Context | Error Variants | Module Ownership |
|---------|----------------|------------------|
| Schema validation (E01xx) | 12 variants | `schema/` |
| Reference validation (E02xx) | 4 variants | `references/` |
| Control-flow (E03xx) | 8 variants | `control_flow/` |
| Type/Taint/Limit (E04xx) | 13 variants | `type_taint/` |
| Gate verifier (E05xx) | 18 variants | `gates/` |
| Contract discovery (E06xx) | 3 variants | `idempotency_contract/` |

**Cohesion Score**: ⚠️ **LOW** — Single enum spans 6 domains, violating Single Responsibility Principle.

---

## 3. Violations

### 3.1 God Enum Anti-pattern (CRITICAL)

`ValidationError` (lines 105–384) is a 279-line enum with **50+ variants** covering unrelated domains. This is a classic "God Object" anti-pattern.

**Impact**:
- Every new validation domain requires modifying this enum
- The `code()` method (lines 392–462) is a 66-line match expression that must track every variant
- Test surface area is N×M (N contexts × M variants per context)

**Refactor**: Split into domain-specific error types:
```
SchemaError      → schema/ errors
ReferenceError   → reference/ errors  
ControlFlowError → control_flow/ errors
TypeTaintError   → type_taint/ errors
GateError        → gate-specific errors (E05xx)
ContractError    → contract-discovery errors
```

### 3.2 Primitive Obsession (HIGH)

| Field Type | Occurrences | Should Be |
|------------|-------------|-----------|
| `String` | 20+ fields | NewType wrapper (e.g., `FieldName`, `Version`, `StepId`, `Reference`) |
| `usize` | 15+ fields | Already appropriate for counts/indices |
| `u32` | 2 fields | Acceptable for symbol bounds |

**Violations**: `field`, `version`, `id`, `reference`, `secret`, `resource`, `trigger`, `file`, `expected`, `actual`, `chain`, `detail`, `name`

### 3.3 Low Module Cohesion

The module declarations (lines 29–37) are correctly decomposed:
```rust
pub mod control_flow;
pub mod diagnostic;
pub mod gates;
pub mod idempotency_contract;
pub mod references;
pub mod schema;
pub mod shared;
pub mod type_taint;
```

However, all validation results funnel through a **single `ValidationError` enum**, defeating the purpose of domain decomposition. Each module should own its own error type.

---

## 4. DDD Smell Summary

| Smell | Severity | Description |
|-------|----------|-------------|
| God Enum | CRITICAL | 50+ variants in one enum spanning 6 bounded contexts |
| Primitive Obsession | HIGH | Raw `String` for domain concepts (id, reference, field, etc.) |
| Anemic Domain Model | MEDIUM | `ValidationError` is a pure data carrier with no behavior beyond `code()` |
| Feature Envy | LOW | `code()` method knows too much about every variant |

---

## 5. Required Actions

### Priority 1: Split `ValidationError`
Extract domain-specific error enums. The parent `ValidationError` becomes a wrapper enum:
```rust
pub enum ValidationError {
    Schema(SchemaError),
    Reference(ReferenceError),
    ControlFlow(ControlFlowError),
    TypeTaint(TypeTaintError),
    Gate(GateError),
    Contract(ContractError),
}
```

### Priority 2: NewType String Fields
Create Value Objects for repeated String fields:
- `StepId`, `FieldName`, `Version`, `Reference`, `Secret`, `ResourceName`, `Trigger`

### Priority 3: Reduce File Size
Target: ≤300 lines. Current: 471 lines. Required reduction: ~170 lines.
- Extract `ValidationError` to its own file (~280 lines)
- Keep module declarations and public API (~100 lines)
- Move `#[cfg(test)]` modules to separate files

---

## 6. Status

```
STATUS: DRIFT DETECTED
```

**Files requiring changes**:
- `crates/vb_validate/src/lib.rs` — needs split
- New file: `crates/vb_validate/src/error/` directory for domain-specific errors
