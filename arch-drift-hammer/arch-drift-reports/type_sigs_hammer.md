# Architectural Drift Report: `type_sigs.rs`

**File**: `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/type_sigs.rs`  
**Total Lines**: 362 (exceeds 300-line limit by **62 lines**)  
**Enforcement Action**: MANDATORY REFACTOR

---

## 🚨 CRITICAL VIOLATIONS

### 1. FILE SIZE VIOLATION
- **Status**: FAILS
- **Current**: 362 lines
- **Limit**: 300 lines
- **Overflow**: 62 lines

### 2. PRIMITIVE OBSESSION - `ResourceLimits` (Lines 111-145)

**VIOLATION**: 16 raw `usize` fields representing distinct semantic concepts.

| Field | Semantic Concept | Should Be |
|-------|------------------|-----------|
| `max_steps` | Workflow step count limit | `MaxSteps(u32)` |
| `max_slots` | Slot allocation limit | `MaxSlots(u32)` |
| `max_constants` | Constant pool limit | `MaxConstants(u32)` |
| `max_accessors` | Accessor table limit | `MaxAccessors(u32)` |
| `max_expressions` | Expression program limit | `MaxExpressions(u32)` |
| `max_expr_stack` | Expression stack depth | `ExprStackDepth(u8)` |
| `max_step_budget_per_tick` | Deterministic budget | `StepBudget(u32)` |
| `max_input_bytes` | Input payload size | `InputBytes(u32)` |
| `max_output_bytes` | Output payload size | `OutputBytes(u32)` |
| `max_blob_bytes` | Blob storage limit | `BlobBytes(u32)` |
| `max_ipc_payload_bytes` | IPC message size | `IpcPayloadBytes(u32)` |
| `max_retry_attempts` | Retry limit | `RetryAttempts(u8)` |
| `max_fanout` | Branch count limit | `FanoutLimit(u16)` |
| `max_collect_items` | Collection size limit | `CollectItems(u32)` |
| `max_queue_depth` | Queue depth limit | `QueueDepth(u32)` |
| `max_journal_batch_bytes` | Journal batch size | `JournalBatchBytes(u32)` |

**Scott Wlaschin Violation**: "Make illegal states unrepresentable." Raw `usize` allows:
- Zero for fields that should have minimum of 1
- `usize::MAX` for fields that should have reasonable upper bounds
- Mixing up parameter positions at call sites

---

## 🚨 PRIMITIVE OBSESSION VIOLATIONS

### 3. `InputDecl::name` (Line 103)
- **Type**: `String`
- **Violation**: Input names are not a free-form string type; they have structural constraints
- **Should Be**: `InputName(String)` wrapper or `SmolStr` with validation

### 4. `InputDecl::is_secret` (Line 107)
- **Type**: `bool`
- **Violation**: Duplicates `Taint` semantics; bool loses the merge semantics
- **Should Be**: `Taint` field directly, removing redundant `is_secret`

### 5. `TypedValue::Reference(String)` (Line 223)
- **Type**: `String` for reference paths like `$input.user`
- **Violation**: Reference paths are a distinct semantic type
- **Should Be**: `RefPath(String)` or `VarRef(String)` newtype

### 6. `TypedValue::Slot(usize)` (Line 225)
- **Type**: Raw `usize`
- **Violation**: Slot indices are bounded by `max_slots`
- **Should Be**: `SlotIndex(u32)` with bounded validation

### 7. `StepTypes::id` (Line 190)
- **Type**: `String`
- **Violation**: Step IDs have structural constraints (non-empty, unique within workflow)
- **Should Be**: `StepId(String)` newtype

### 8. `WorkflowTypes::vars` (Line 176)
- **Type**: `Vec<(String, ValueType)>`
- **Violation**: Tuple with raw String for variable names
- **Should Be**: `Vec<VarDecl>` where `VarDecl { name: VarName(String), var_type: ValueType }`

### 9. `WorkflowTypes::secrets` (Line 178)
- **Type**: `Vec<String>`
- **Violation**: Secret names are distinct from general strings (case-sensitive, scoped)
- **Should Be**: `Vec<SecretName(String)>`

---

## ⚠️ DDD COHESION VIOLATIONS

### 10. `WorkflowTypes` God Object (Lines 172-183)
- **Problem**: Aggregates 5 concerns: inputs, vars, secrets, steps, resource_contract
- **Scott Wlaschin**: "A module should have a single reason to change"
- **Should Split Into**:
  - `WorkflowSignature` (inputs, vars, secrets only - the type-level interface)
  - `WorkflowSteps` (steps only - control flow)
  - `ResourceContract` (resource_limits only - resource governance)

### 11. `TypedValue` Enum Shotgun Marriage (Lines 219-228)
- **Problem**: 4 variants representing 4 distinct semantic concepts
- **Should Refactor**:
  - `TypedValue::Literal` stays (type literal)
  - `TypedValue::Reference` → `VarRef(VarName)`
  - `TypedValue::Slot` → `SlotRef(SlotIndex)`
  - `TypedValue::Composite` → `TypedComposite { fields: Vec<TypedValue> }` or separate enum

---

## 📊 METRICS SUMMARY

| Metric | Value | Status |
|--------|-------|--------|
| File Lines | 362 | ❌ FAILS (>300) |
| Newtype Wrappers Needed | 11+ | HIGH REFACTOR COST |
| Primitive Fields | 19 | SEVERE |
| Distinct Value Types | 7 | OK |
| Distinct Step Kinds | 3 | OK |

---

## ✅ WHAT IS CORRECT

1. **`ValueType`** - Clean enum, well-scoped, `as_str()` for diagnostics ✓
2. **`Taint`** - Correct semantic modeling with merge semantics ✓
3. **`ValueFact`** - Proper composition of `ValueType` + `Taint` ✓
4. **`StepKind`** variants - Well-designed discriminated union ✓
5. **`ResourceLimits::default()`** - Sensible defaults ✓

---

## 🔧 MANDATORY REFACTOR PRESCRIPTION

### Phase 1: Newtype Wrappers (Cost: ~80 lines)
```rust
pub struct MaxSteps(u32);
pub struct MaxSlots(u32);
// ... etc for all 16 ResourceLimits fields

impl ResourceLimits {
    pub fn max_steps(&self) -> MaxSteps { MaxSteps(self.max_steps as u32) }
}
```

### Phase 2: Value Object Extraction (Cost: ~40 lines)
```rust
pub struct InputName(String);
pub struct SecretName(String);
pub struct VarName(String);
pub struct StepId(String);
pub struct RefPath(String);
pub struct SlotIndex(u32);
```

### Phase 3: Split `WorkflowTypes` (Cost: 0 lines, actually reduces)
- Extract `WorkflowSignature`, `WorkflowSteps` as separate types

### Phase 4: File Splitting (Cost: Reduces line count)
Split into:
- `value_types.rs` (~80 lines) - ValueType, Taint, ValueFact
- `typed_value.rs` (~100 lines) - TypedValue, RefPath, SlotIndex
- `workflow_model.rs` (~100 lines) - InputDecl, WorkflowTypes, StepTypes, StepKind
- `resource_limits.rs` (~80 lines) - ResourceLimits with newtypes

---

## 📋 IMMEDIATE ACTIONS REQUIRED

1. [ ] Create newtype wrappers for all `usize` fields in `ResourceLimits`
2. [ ] Create `InputName`, `SecretName`, `VarName`, `StepId`, `RefPath`, `SlotIndex` wrappers
3. [ ] Replace `is_secret: bool` with `taint: Taint` in `InputDecl`
4. [ ] Split `WorkflowTypes` into `WorkflowSignature` + `WorkflowSteps`
5. [ ] Split file into 4 smaller modules
6. [ ] Reduce total lines to <300

---

**ENFORCEMENT**: This file MUST be refactored before any new features can land.  
**PRIORITY**: CRITICAL  
**ESTIMATED REFACTOR COST**: 2-3 hours  
**TECHNICAL DEBT**: 11+ newtype wrappers + file split required
