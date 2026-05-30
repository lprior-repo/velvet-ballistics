# ARCHITECTURAL DRIFT HAMMER REPORT

## TARGET
`/home/lewis/src/velvet-ballistics/crates/vb_compile/src/expression_bytecode.rs`

## SEVERITY: CATASTROPHIC

**Lines: 2533 | Limit: 300 | Violation: 8.4x over budget**

---

## PHASE MIXING VIOLATIONS

This file is a **God Module** that conflates 4 distinct compilation phases:

### Phase Map (current混 conflated state)

| Phase | Lines | Status |
|-------|-------|--------|
| Reference Resolution | 80-351 | MIXED |
| Expression Lowering | 353-472 | MIXED |
| Bytecode Ops Mapping | 474-551 | MIXED |
| Tests | 553-2533 | OVERSIZED |

### Functions by Phase

**Reference Resolution Phase** (272 lines):
- `ExpressionReferenceResolver` trait (81-84)
- `RejectingReferenceResolver` impl (86-94)
- `SlotAccessorReferenceResolver` impl (96-105)
- `StepSlotReferenceResolver` impl (107-122)
- `lower_slot_reference` (124-135)
- `parse_slot_reference_parts` (137-157)
- `parse_step_reference_parts` (168-191)
- `lower_step_reference` (200-229)
- `resolve_step_slot` (232-246)
- `parse_field_path_segments` (253-273)
- `split_reference_tail` (275-280)
- `parse_slot_reference_index` (282-291)
- `lower_accessor_reference` (293-312)
- `numeric_path_segments` (314-326)
- `parse_list_index_segment` (328-338)
- `unsupported_accessor_reference` (340-351)

**Expression Lowering Phase** (119 lines):
- `lower_expr` (353-370)
- `lower_reference` (372-379)
- `lower_literal` (381-400)
- `lower_unary` (402-417)
- `lower_numeric_negation` (419-430)
- `lower_binary` (432-444)
- `lower_helper` (446-459)
- `push_expression_constant` (461-472)

**Bytecode Ops Mapping Phase** (78 lines):
- `binary_op` (474-489)
- `helper_op` (491-507)
- `validate_helper_arity` (509-520)
- `helper_arity` (522-533)
- `helper_name` (535-551)

**Public API Phase** (lines 11-78):
- `compile_expr_to_bytecode`
- `compile_expr_to_bytecode_with_accessors`
- `compile_expr_to_bytecode_with_step_slots`
- `compile_expr_to_bytecode_with_resolver`

**Tests Phase** (1981 lines):
- 553-2533: ~100+ test functions

---

## PRIMITIVE OBSESSION VIOLATIONS

### 1. Raw `u16` Parsing (lines 283-290)
```rust
fn parse_slot_reference_index(reference: &str, slot: &str) -> Result<SlotIdx, CompileError> {
    let parsed = slot
        .parse::<u16>()  // ← PRIMITIVE OBSESSION: raw u16
        .map_err(|_| CompileError::UnknownReferenceName { ... })?;
    Ok(SlotIdx::new(parsed))
}
```
**Should be**: A dedicated `SlotIndex` value object with its own `try_from` implementation.

### 2. `&str` Everywhere for References
Every function takes `reference: &str` instead of a typed `Reference<'a>` or `ParsedReference<'a>`.
```rust
fn resolve_reference(&mut self, reference: &str) -> Result<ExprOp, CompileError>;
fn lower_slot_reference(reference: &str, ...) -> Result<ExprOp, CompileError>;
fn parse_step_reference_parts(reference: &str) -> Result<(&str, Option<&str>), CompileError>;
```
**Should be**: A `Reference` enum with variants for `Slot`, `Slots`, `Step`, `Steps`.

### 3. Raw Integer Arithmetic for Index Overflow (lines 212-216, 302-306, 465-468)
```rust
let index = u16::try_from(accessors.len()).map_err(|_| {
    CompileError::ExpressionLoweringUnsupported {
        feature: "accessor table overflow".into(),
    }
})?;
```
**Should be**: A typed `AccessorIndex::new()` that encapsulates the overflow check.

### 4. Untyped Path Segments
`parse_field_path_segments` returns `Vec<PathSegment>` but parsing is done with raw string splitting:
```rust
for segment in field_path.split('.') {
    if segment == "result" {
        segments.push(PathSegment::Index(0));
    }
}
```
**Should be**: A `FieldPath` value object that validates during construction.

### 5. Helper Metadata as Runtime Functions (lines 522-551)
```rust
const fn helper_arity(helper: ExpressionHelper) -> usize {
    match helper {
        ExpressionHelper::Exists | ... => 1,
        ...
    }
}
```
**Should be**: `impl ExpressionHelper { const fn arity(&self) -> usize }` as a method on the enum, or derive.

---

## ARCHITECTURAL DECAY ROOT CAUSES

### 1. God Module Anti-Pattern
Single file handles ALL expression→bytecode lowering. Should be a module with multiple files:
```
src/bytecode/
├── mod.rs          # Re-exports, public API
├── lower.rs        # Expression lowering (353-472)
├── resolver.rs     # Reference resolution (80-351)
├── ops.rs          # Op mappings (474-551)
└── metadata.rs     # Helper arity/name
```

### 2. Test Inflation
1981 lines of tests are INLINE in a 2533-line file. Tests should be:
```
tests/
└── expression_bytecode_tests.rs  # All 100+ tests
```
Inline tests should be minimal (10-20 lines max).

### 3. Missing Type Segregation
- `ParsedExpression` lowering is mixed with `ExprOp` bytecode generation
- No clear boundary between "I have a parsed tree" and "I am emitting bytecode"

### 4. Resolver Pattern Over-Engineering
Three resolver types for near-identical logic:
- `RejectingReferenceResolver`
- `SlotAccessorReferenceResolver`
- `StepSlotReferenceResolver`

This is Strategy pattern abuse. A single resolver with configuration would suffice.

---

## MANDATORY REFACTORING prescription

### Step 1: Extract Tests (no logic changes)
```bash
# Move all tests to tests/expression_bytecode_tests.rs
# Keep 3-5 inline smoke tests
```

### Step 2: Split into Module
```
src/bytecode/
├── mod.rs          (~50 lines: re-exports + public API)
├── lower.rs        (~130 lines: lower_expr, lower_literal, lower_unary, lower_binary, lower_helper)
├── resolver.rs     (~280 lines: all reference resolution)
├── ops.rs          (~80 lines: binary_op, helper_op, helper_arity, helper_name)
```

### Step 3: Introduce Value Objects
```rust
// reference.rs
pub struct Reference<'a>(&'a str);

pub enum ReferenceRoot {
    Slot(u16),
    Slots(u16),
    Step(&'static str),
    Steps(&'static str),
}
```

### Step 4: Derive Helper Metadata
```rust
impl ExpressionHelper {
    pub const fn arity(&self) -> usize {
        match self {
            Self::Exists | Self::Length | ... => 1,
            Self::AppendIf => 3,
            _ => 2,
        }
    }
}
```

### Step 5: Target Line Counts
| Module | Max Lines |
|--------|-----------|
| `mod.rs` | 50 |
| `lower.rs` | 150 |
| `resolver.rs` | 280 |
| `ops.rs` | 80 |
| **Total** | **560** |

Still over 300-line limit for the largest modules. `resolver.rs` needs further splitting.

---

## DDD principles violated

1. **Mixed bounded contexts**: Reference parsing IS bytecode lowering is IS expression tree walking
2. **Primitive obsession**: `&str`, `u16`, raw `usize` everywhere
3. **Feature envy**: `resolver.rs` accesses `accessors.len()` from external scope
4. **Inappropriate intimacy**: Resolvers know too much about accessor table internals

---

## VERDICT

**UNACCEPTABLE** - File must be拆分 (split) before any new features land.

**Recommended bead**: "Split expression_bytecode.rs into bytecode module (<300 lines per file)"

---

*Generated by architectural-drift enforcer*
*Framework: Scott Wlaschin DDD + <300 line rule*
