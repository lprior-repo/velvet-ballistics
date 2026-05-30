# ARCHITECTURAL DRIFT REPORT: `compile/mod.rs`

**File**: `crates/vb_compile/src/compile/mod.rs`
**Line Count**: 894 ( HARD VIOLATION: 894 >> 300 limit)
**Status**: REFACTOR REQUIRED

---

## EXECUTIVE SUMMARY

This file violates the **<300 line rule** by 298% (894 lines). It is a **God Module** that conflates 8+ distinct compilation responsibilities. Every `lower_*` function, the `SlotCompiler` aggregate, scope validation, digest computation, and artifact emission are all jammed into a single file with no domain boundary enforcement.

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 894 | 300 | **VIOLATION (+198%)** |
| Functions | ~40 | ~15 | **VIOLATION** |
| Modules co-located | 1 | N/A | God Module |

---

## 2. RESPONSIBILITY MAPPING

The file handles **8 distinct compilation responsibilities** that MUST be separated:

| Responsibility | Lines | Function(s) | Domain |
|----------------|-------|------------|--------|
| Top-level workflow compilation | 21–110 | `compile_source`, `compile_workflow` | **Workflow Compilation** |
| Scope validation | 112–180 | `validate_canonical_compile_scope` | **Validation** |
| Digest computation | 220–261 | `canonical_digest`, `digest_step_primitive` | **Digest** |
| Slot compiler (aggregate) | 780–873 | `SlotCompiler` impl | **Slot Management** |
| Primitive lowering: set/finish | 315–329, 692–702 | `lower_set`, `lower_finish` | **IR Lowering** |
| Primitive lowering: control flow | 331–690 | `lower_do`, `lower_choose`, `lower_for_each`, `lower_together`, `lower_collect`, `lower_reduce`, `lower_repeat`, `lower_wait`, `lower_ask` | **IR Lowering** |
| Artifact emission | 704–730 | `validate_ir`, `emit_compiled_artifact`, `compile_to_generated_rust` | **Codegen** |
| Idempotency checking | 732–778 | `check_idempotency_gates` | **Contract Validation** |

**Scott Wlaschin Violation**: These responsibilities belong to different bounded contexts (Compilation, Validation, Digest, Codegen). Colocating them creates implicit coupling and prevents independent evolution.

---

## 3. PRIMITIVE OBSESSION VIOLATIONS

### 3.1 `&str` for Domain Identifiers

```rust
// Line 30: HashMap<&str, SlotIdx> — step names are just raw strings
let mut outputs: HashMap<&str, SlotIdx> = HashMap::new();

// Line 38: Vec<Box<str>> for step names — no StepName newtype
let mut step_names: Vec<Box<str>> = Vec::with_capacity(steps.len());

// Line 54, 70: outputs.contains_key(output.as_str()) — &str as lookup key
```

**Violation**: `StepId`, `OutputName`, `StepName` are domain concepts represented as raw `&str` or `Box<str>`. No `newtype` wrapping.

**Refactor Required**:
```rust
// Instead of HashMap<&str, SlotIdx>
struct StepOutputs(HashMap<OutputName, SlotIdx>);

// Instead of Vec<Box<str>>
struct StepNameTable(Vec<StepName>);

// Instead of Box<str> for names
struct OutputName(Box<str>);
```

### 3.2 Unbounded Integers for Limits

```rust
// Line 378: u32 for limit — no bounds checking at type level
pub fn lower_for_each(id: StepIdx, input: SlotIdx, item_slot: SlotIdx, limit: u32, ...)

// Line 466-467: u32 for limit and page_size
pub fn lower_collect(id: StepIdx, source: SlotIdx, limit: u32, page_size: u32, ...)

// Line 565: u16 for max_attempts
pub fn lower_repeat(id: StepIdx, max_attempts: u16, ...)
```

**Violation**: `u32` and `u16` have no domain meaning. `limit: u32` could be 0 or 4 billion. No `NonZero`, no bounded range.

**Refactor Required**:
```rust
// Bounded newtypes
struct IterationLimit(u32);  // 1..=MAX_ITERATIONS
struct PageSize(u32);        // 1..=MAX_PAGE_SIZE
struct MaxAttempts(u16);    // 1..=MAX_ATTEMPTS
```

### 3.3 Raw `i64` Parsing

```rust
// Line 59: Raw i64 from string parsing
let parsed = value.parse::<i64>().map_err(|_| {
    CompileErrors(vec![CompileError::StepFieldShape { ... }])
});
```

**Violation**: `i64` is not a domain value. The workflow should have `IntegerValue`, `NaturalValue`, etc.

### 3.4 Raw `usize` for Index Arithmetic

```rust
// Line 829-833: Raw usize in SlotCompiler
let value = slot.as_usize();
self.max_slot = Some(match self.max_slot {
    Some(current) => current.max(value),
    None => value,
});
```

**Violation**: `usize` is machine-architecture dependent. `SlotCount`, `StepCount` newtypes needed.

### 3.5 Exposed `blake3::Hasher` in Digest Computation

```rust
// Lines 221-240: Raw hasher exposed in canonical_digest
let mut hasher = blake3::Hasher::new();
hasher.update(source.version().as_bytes());
// ...
WorkflowDigest::from_bytes(hasher.finalize().into())
```

**Violation**: Domain boundary leak. `DigestComputation` is a pure function that should abstract the hashing algorithm.直接使用 `blake3::Hasher` 破坏了领域边界.

### 3.6 `Box<str>` Without Domain Wrappers

```rust
// Lines 56, 96, 135, 190: Box<str> scattered
name: output.clone().into_boxed_str(),           // line 56
name: Box::from(source.name()),                  // line 96
id: Box::from(step.id.as_str()),                // line 135
name: name.clone().into_boxed_str(),            // line 190
```

**Violation**: `Box<str>` is a memory allocation detail, not a domain concept. `StepName`, `OutputName`, `WorkflowName` newtypes should wrap these.

---

## 4. SCOTT WLASCHIN DDD VIOLATIONS

### 4.1 No Value Objects for Step/Slot Indices

`StepIdx` and `SlotIdx` exist in `vb_core`, but usage within this file treats them as transparent wrappers around `u16`. The `SlotCompiler` directly accesses `.as_usize()` and does raw arithmetic:

```rust
// Line 831: Direct .as_usize() — ignores bounds semantics
let value = slot.as_usize();
```

**Should be**: `SlotIdx` should expose only safe operations (`next()`, `checked_add()`) and hide raw `u16`.

### 4.2 No `Parse, Don't Validate` Coherence

The `validate_canonical_compile_scope` function validates *restrictions* (what is NOT allowed) but doesn't parse into domain types:

```rust
// Lines 144-173: Validates by checking .is_some() on fields
if step.condition.is_some() {
    errors.push(CompileError::UnsupportedStepControlField { ... });
}
```

**Violation**: This is validation-after-parsing, not parse-first. The canonical source should be parsed into domain types directly, with invalid structures rejected at parse time.

### 4.3 `SlotCompiler` is a Non-Trivial Aggregate

```rust
// Lines 780-787: SlotCompiler holds 5 collections
pub struct SlotCompiler {
    nodes: Vec<CompiledNode>,
    constants: Vec<ConstValue>,
    expressions: Vec<ExprProgram>,
    accessors: Vec<AccessorProgram>,
    max_slot: Option<usize>,
}
```

**Violation**: This is a **parameter object** with hidden invariants (max_slot must be consistent with recorded slots). It should be split:
- `NodeBuilder` for nodes
- `ConstantPool` for constants  
- `ExpressionTable` for expressions
- `AccessorTable` for accessors
- `SlotRegistry` for slot tracking

### 4.4 Workflow State Machine Not Modeled

The `lower_*` functions construct `CompiledNode` chains but there's no explicit state machine model. The workflow has implicit states (Running, Waiting, Finished) but they're encoded in `CompiledNodeKind` enum rather than modeled as a proper state machine.

---

## 5. NAMING VIOLATIONS

| Line | Current | Should Be | Issue |
|------|---------|-----------|-------|
| 19 | `WORKFLOW_VERSION: &str` | `WORKFLOW_VERSION_STR: &str` | Constant naming |
| 275 | `build_slot_layout` | `slot_layout` | Getter style |
| 279 | `build_accessor_table` | `accessor_table` | Getter style |
| 283 | `build_constant_pool` | `constant_pool` | Getter style |
| 331 | `lower_do` | `lower_action` or `lower_call` | "do" is not a domain term |
| 416 | `lower_together` | `lower_parallel` | "together" is not a domain term |

---

## 6. REQUIRED REFACTORING

### Phase 1: Split into Modules (Target: <300 lines each)

```
crates/vb_compile/src/compile/
├── mod.rs          (re-export only, ~50 lines)
├── source.rs       (~200 lines) — compile_source, validate_canonical_scope
├── digest.rs       (~150 lines) — canonical_digest, digest_step_primitive  
├── lower.rs        (~200 lines) — lower_set, lower_finish, lower_ask
├── lower_control.rs (~200 lines) — lower_do, lower_choose, lower_for_each, lower_together, etc.
├── slot.rs         (~200 lines) — SlotCompiler struct and impl
├── emit.rs         (~150 lines) — emit_compiled_artifact, compile_to_generated_rust
└── validate.rs     (~100 lines) — validate_ir, check_idempotency_gates
```

### Phase 2: Newtype Wrappers

Create in `vb_core` or `vb_compile`:

```rust
// Newtypes for domain identifiers
struct StepName(Box<str>);
struct OutputName(Box<str>);
struct WorkflowName(Box<str>);

// Bounded integers
struct IterationLimit(u32);
struct PageSize(u32);
struct MaxAttempts(u16);

// Parsed value objects
enum ParsedIntegerValue { Natural(u64), Negative(i64) }
```

### Phase 3: SlotCompiler Decomposition

```rust
// Split into focused builders
struct WorkflowBuilder {
    nodes: NodeBuilder,
    constants: ConstantPool,
    expressions: ExpressionTable,
    accessors: AccessorTable,
    slots: SlotRegistry,
}
```

---

## 7. VERIFICATION CHECKLIST

- [ ] File split into modules <300 lines each
- [ ] `StepName` / `OutputName` newtypes introduced
- [ ] `IterationLimit`, `PageSize`, `MaxAttempts` bounded types
- [ ] `SlotCompiler` decomposed into focused builders
- [ ] Raw `blake3::Hasher` abstracted behind `DigestComputation`
- [ ] `WORKFLOW_VERSION` renamed to `WORKFLOW_VERSION_STR`
- [ ] Getter methods renamed to Rust convention (no `build_` prefix)
- [ ] All `&str` lookups replaced with domain type lookups
- [ ] `lower_together` renamed to `lower_parallel`
- [ ] `lower_do` renamed to `lower_action`

---

## VERDICT

**ARCHITECTURAL DRIFT: CONFIRMED**

The file is a **God Module** at 894 lines that must be split. Primitive obsession is rampant: raw `&str` for domain identifiers, unbounded `u32`/`u16` for limits, raw `usize` for arithmetic, and exposed `blake3::Hasher` in digest computation.

**Mandate**: Refactor before any new work lands on this module.
