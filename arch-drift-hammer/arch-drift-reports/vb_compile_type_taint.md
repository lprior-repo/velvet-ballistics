# Architectural Drift Report: `vb_compile/src/type_taint.rs`

**File:** `crates/vb_compile/src/type_taint.rs`  
**Analyzed:** 2026-05-29  
**Status:** ❌ VIOLATIONS FOUND

---

## 1. Line Count Check

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | **514** | 300 | ❌ FAIL (+214 lines over) |

---

## 2. DDD Cohesion Analysis

### Domain Concepts Identified

| Concept | Type | Location | Concern |
|---------|------|----------|---------|
| `ValueType` | Enum | Lines 14-36 | Typestate for runtime values |
| `ValueFact` | Struct | Lines 38-68 | Value + Taint pair |
| `Taint` | External Enum | Line 6 | Security classification |
| `Facts` | Struct | Lines 71-100 | Type environment (God object) |
| Schema parsing | Functions | Lines 102-147 | Schema → ValueFact |
| Expression typing | Functions | Lines 290-417 | Expr → ValueFact |
| Workflow validation | Functions | Lines 185-251 | Step → type checking |

### Cohesion Score: **LOW**

The file exhibits **Too Many Responsibilities** anti-pattern. Single file handles:
1. Type algebra (`ValueType`)
2. Taint tracking (`Taint` + `ValueFact::merge`)
3. Fact collection/environment (`Facts`)
4. Schema fact inference
5. Expression fact inference
6. Workflow step validation
7. Reference resolution

---

## 3. Violations

### ❌ CRITICAL: Line Count Exceeded
- 514 lines vs. 300 line maximum
- **Must be split into 2+ files**

### ⚠️ HIGH: Primitive Obsession
- `&str` used for `field: &'static str` (lines 94, 267, 280, 290, 303, etc.)
- `&str` used for `reference: &str` (line 487)
- No newtypes for: `SlotIndex`, `FieldName`, `SchemaType`, `Reference`

### ⚠️ HIGH: God Object - `Facts` Struct
- `Facts` holds 4 HashMaps: `inputs`, `vars`, `secrets`, `slots`
- Violates Single Responsibility Principle
- Access pattern suggests 3-4 separate environments exist

### ⚠️ MEDIUM: Feature Envy
- `Facts::read_slot` and `Facts::write_slot` dominate step validation
- Heavy coupling between `Facts` and step handling logic

### ⚠️ MEDIUM: Hybrid Bounded Context
- "Type" and "Taint" are separate domain concerns
- `type_taint` module name suggests conflation

### ⚠️ LOW: `#[allow(unreachable_code)]` on line 61
- Comment claims safety rationale but `allow` weakens lint

---

## 4. Recommended Refactor

```
type_taint.rs (514 lines) → split into:

type_taint/
├── mod.rs          (~30 lines)  - re-exports
├── value_type.rs   (~80 lines)  - ValueType enum, as_str
├── value_fact.rs   (~70 lines)  - ValueFact struct, merge logic
├── facts.rs        (~100 lines) - Facts struct, slot management
├── schema.rs       (~80 lines)  - schema_type, input_facts, value_facts
├── expression.rs   (~130 lines) - expression_fact, parsed_expression_fact, helpers
└── workflow.rs     (~70 lines)  - validate_steps, validate_condition
```

---

## 5. DDD Smell Summary

| Smell | Severity |
|-------|----------|
| File too large (>300 lines) | CRITICAL |
| God Object (Facts) | HIGH |
| Primitive Obsession (str references) | HIGH |
| Too Many Responsibilities | HIGH |
| Feature Envy | MEDIUM |
| Conflated bounded contexts (type + taint) | MEDIUM |

---

## 6. Priority Assessment

| Priority | Action |
|----------|--------|
| **P0** | Split file to meet <300 line requirement |
| **P1** | Extract `Facts` into separate `fact_table.rs` or similar |
| **P2** | Introduce newtypes for `SlotIndex`, `FieldName` |
| **P3** | Consider separating `Taint` tracking into its own module |

---

**END REPORT**
