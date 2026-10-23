# Source-Line API Audit: vb-8eocj.4 Input Resolution

Workspace: /home/lewis/src/isoloated/vb-8eocj-source-map-local5090-v1
Audit date: 2026-10-23
Method: Read-only inspection of actual source files. No builds, no test runs.

## Files Inspected

| File | Role |
|------|------|
| crates/vb_compile/src/mod_compile_lowering/part_01.rs | Main compile_source flow, allocate_input_slots |
| crates/vb_compile/src/mod_compile_lowering/part_02.rs | Step lowering dispatch |
| crates/vb_compile/src/mod_compile_lowering/canonical_references.rs | resolve_slot_reference, StepReferenceResolver |
| crates/vb_compile/src/mod_compile_lowering/canonical_do.rs | Do input resolution |
| crates/vb_compile/src/mod_compile_lowering/canonical_scalar.rs | Set expression lowering |
| crates/vb_compile/src/mod_compile_lowering/finish_inputs.rs | Finish input slot preparation |
| crates/vb_compile/src/mod_compile_lowering/input_schema.rs | kind_from_input_schema |
| crates/vb_compile/src/mod_compile_errors/kind.rs | CompileError variants |
| crates/vb_compile/src/references.rs | Compile-time reference validation |
| crates/vb_validate/src/references.rs | Shared reference validation (RefTables) |
| crates/vb_core/src/workflow.rs | WorkflowParts (slot_count, input_slots field) |

## Verified Symbols and Locations

### 1. Input Slot Allocation

**Symbol:** `allocate_input_slots` (free function)
**Location:** `part_01.rs:95`
**Signature:** `pub(super) fn allocate_input_slots(inputs: &[InputField], builder: &mut SlotCompiler) -> Result<(), CompileError>`
**Behavior:** Iterates declared inputs, computes slot index from `builder.slot_count()` base + declaration index, calls `builder.record_slot(slot)` and `builder.record_input_slot(slot, kind)`. Kind derived via `kind_from_input_schema`.

**Called from:**
- `part_01.rs:70` — after all step lowering (non-finish case)
- `part_01.rs:47` via `prepare_finish_input_slots` — finish case with `$input.` refs

### 2. Slot Reference Resolution

**Symbol:** `resolve_slot_reference` (free function)
**Location:** `canonical_references.rs:47`
**Signature:** `pub(crate) fn resolve_slot_reference(reference: &str, outputs: &HashMap<String, SlotIdx>, step_outputs: &HashMap<String, SlotIdx>) -> Result<SlotIdx, CompileError>`
**Behavior:**
- `$`-prefixed -> `resolve_dollar_reference`
- Non-`$` -> `resolve_numeric_slot`
- Handles `$step.<id>` / `$steps.<id>` explicitly
- Direct names checked in both `outputs` and `step_outputs` HashMaps
- Unknown -> `CompileError::UnknownReferenceName { kind: "output" | "step", ... }`

**Symbol:** `StepReferenceResolver` (struct, resolver adapter)
**Location:** `canonical_references.rs:25`
**Purpose:** Wraps two HashMaps for use with expression bytecode compilation.

### 3. Error Variants (Actual)

**Location:** `kind.rs`

- `UnknownReferenceName { kind: &'static str, reference: Box<str>, name: Box<str> }` — line 138
  - Used for unknown input names (`kind: "input"`), step names (`kind: "step"`), output names (`kind: "output"`), slot names (`kind: "slot"`)
- `InvalidInputSchema { field: &'static str, expected: &'static str }` — line 82
- `TypeMismatch { field: &'static str, expected: &'static str, found: &'static str }` — line 148
- `UnknownReferenceRoot { reference: Box<str>, root: Box<str> }` — line 133
- `DuplicateOutputName { name: Box<str> }` — line 73

### 4. Input Schema Kind Mapping

**Symbol:** `kind_from_input_schema`
**Location:** `input_schema.rs:9`
**Behavior:** Maps declared schema strings to `InputSlotKind`:
- `"text"` -> `InputSlotKind::Symbol`
- `"number"` -> `InputSlotKind::I64`
- `"boolean"` -> `InputSlotKind::Bool`
- `"object"` -> `InputSlotKind::Object`
- `"any"` -> `InputSlotKind::Any`
- `"list"` (shorthand) and `"list<T>"` forms -> `InputSlotKind::List`

### 5. Do Input Resolution

**Symbol:** `resolve_do_input` (free function)
**Location:** `canonical_do.rs:57`
**Signature:** `fn resolve_do_input(input_expr: &str, outputs: &HashMap<String, SlotIdx>, step_outputs: &HashMap<String, SlotIdx>, _step_index: usize) -> Result<SlotIdx, CompileErrors>`
**Behavior:** Calls `resolve_slot_reference` directly. No wrapper logic.

### 6. Finish Input Preparation

**Symbol:** `prepare_finish_input_slots`
**Location:** `finish_inputs.rs:35`
**Behavior:**
1. If finish result is `$input.<name>`, calls `allocate_input_slots` first
2. Inserts `$input.<name>` -> slot entries into `outputs` HashMap
3. Checks for duplicate output names
4. Calls `canonical_finish_slot_with_step_outputs` for non-input finish refs

**Symbol:** `resolve_finish_input_slot` (private helper)
**Location:** `finish_inputs.rs:24`
**Behavior:** Looks up `$input.<name>` in `outputs` HashMap. Returns `UnknownOutputName` if not found (after allocation, all declared inputs should be present).

### 7. Set Expression Lowering

**Symbol:** `lower_canonical_set_expression`
**Location:** `canonical_scalar.rs:18`
**Flow:**
1. Try I64 constant parse (preserves SetConst shape)
2. Parse as expression: `parse_expression(value)`
3. Static type check: `infer_canonical_expr_type(&parsed, prior_types)`
4. Compile to bytecode: `compile_expr_to_bytecode_with_resolver(&parsed, &mut builder.constants, &mut resolver)`
   - Resolver is `StepReferenceResolver::new(outputs.clone(), step_outputs.clone())`
5. Emit `EvalExpr` node

## v4 Claims vs Actual

| v4 Claim | Actual | Status |
|----------|--------|--------|
| `DeclaredInputAllocator` type | Free function `allocate_input_slots` | INCORRECT |
| `resolve_slot()` method | Free function `resolve_slot_reference` | INCORRECT |
| `UnknownInputSlot` variant | `UnknownReferenceName { kind: "input" }` | INCORRECT |
| One SlotIdx per input name | Verified: allocation loop uses unique indices | CORRECT |
| resolve_do_input uses same resolver | Verified: calls resolve_slot_reference directly | CORRECT |
| slot_types HashMap for type mapping | InputSlotKind stored per slot, not HashMap | PARTIALLY INCORRECT |
| List shorthand `list<T>` supported | Verified in input_schema.rs | CORRECT |

## Symbols Not Invented

This audit confirms the following are NOT present in the codebase:
- No `DeclaredInputAllocator` struct or type
- No `resolve_slot` method on any type
- No `UnknownInputSlot` error variant
- No `InputReferenceResolver` type
- No separate input reference namespace in the resolver
- No `slot_types` HashMap (types are per-slot, not a global map)

## Workflow IR Field

**Symbol:** `input_slots`
**Location:** `WorkflowParts` struct in `vb_core/src/workflow.rs`
**Type:** `Box<[CompiledInputSlot]>`
**Purpose:** Declared input slot indices for runtime binding

`CompiledInputSlot` struct (from grep): contains `slot: SlotIdx` and `kind: InputSlotKind`.

