# Error Taxonomy: Nested Choose Lowering

## Bead Context

- **Bead ID:** vb-xi2f.13
- **Domain:** Error classification for `lower_canonical_choose` with branch body lowering.

---

## 1. Error Categories

### Category A: Compile-Time — Fanout / Structural

| Code | Variant | Trigger | Severity |
|---|---|---|---|
| **C-CHOOSE-FANOUT** | `PrimitiveLoweringLimitExceeded { primitive: "choose", field: "branches", value, limit: 64 }` | `branches.len() > 64` | **FATAL** — cannot compile |
| **C-CHOOSE-EMPTY** | `Workflow(EmptyBranchTable)` | `branches.is_empty() && otherwise.is_none()` | **FATAL** — cannot compile |
| **C-CHOOSE-LABEL** | `UnknownStepLabel { step, label }` | `otherwise` label not in `step_names` | **FATAL** — cannot compile |
| **C-CHOOSE-LABEL-OF** | `PrimitiveLoweringLimitExceeded { primitive: "choose", field: "otherwise_target", value, limit }` | Resolved otherwise index > `u16::MAX` | **FATAL** — cannot compile |

### Category B: Compile-Time — Body Lowering (New)

| Code | Variant | Trigger | Severity |
|---|---|---|---|
| **C-BODY-OVF** | `StepIndexOutOfRange { value }` | `body_width` calculation overflows `usize` | **FATAL** — cannot compile |
| **C-BODY-STEP-OF** | `StepIndexOutOfRange { value }` | Generated `StepIdx` from body node count exceeds `u16::MAX` | **FATAL** — cannot compile |
| **C-BODY-SLOT-OF** | `SlotIndexOutOfRange { value }` | Slot count exceeds `u16::MAX` during body lowering | **FATAL** — cannot compile |
| **C-BODY-PRIM** | `UnsupportedStepPrimitive { step, primitive }` | Body step uses a primitive not supported in branch bodies | **FATAL** — cannot compile |
| **C-BODY-SHAPE** | `StepFieldShape { step, field, expected }` | Body step structure is invalid (e.g., wrong field count) | **FATAL** — cannot compile |
| **C-BODY-COND** | (Expression/slot resolution error) | `slot_from_text` fails for a `when` string | **FATAL** — cannot compile |
| **C-BODY-CONST** | `UnsupportedConstantValue { step }` | Body `Set` value cannot be parsed as constant | **FATAL** — cannot compile |

### Category C: Runtime — Already Handled by Replay Engine

| Code | Error | Trigger | Severity |
|---|---|---|---|
| **R-SLOT-NOT-BOOL** | `ReplayError::Internal { reason: "choose_slot condition is not boolean" }` | `SlotBranch.condition` slot holds non-bool value | **RUNTIME ERROR** |
| **R-SLOT-OOB** | `ReplayError::SlotNotAvailable { slot }` | `SlotBranch.condition` slot out of bounds | **RUNTIME ERROR** |
| **R-SLOT-UNINIT** | `ReplayError::SlotNotAvailable { slot }` | `SlotBranch.condition` slot uninitialized | **RUNTIME ERROR** |
| **R-NO-MATCH** | `ReplayError::Internal { reason: "choose_slot no branch matched and no otherwise" }` | All branches false, no otherwise | **RUNTIME ERROR** |
| **R-PC-OOB** | `ReplayError::Internal` (from `set_pc` failure) | Branch target out of range | **RUNTIME ERROR** |

### Category D: Validation-Time — IR Integrity

| Code | Variant | Trigger | Severity |
|---|---|---|---|
| **V-CHOOSE-ROUTE** | `WorkflowError::EmptyBranchTable` | `branches.is_empty() && otherwise.is_none()` in compiled IR | **FATAL** — IR invalid |
| **V-BRANCH-SLOT** | `WorkflowError::SlotOutOfBounds` | `SlotBranch.condition` out of bounds | **FATAL** — IR invalid |
| **V-BRANCH-TARGET** | `WorkflowError::StepOutOfBounds` | `SlotBranch.target` out of bounds | **FATAL** — IR invalid |
| **V-CHOOSE-EDGE** | `WorkflowError::BackwardEdge` | Branch target points backward | **FATAL** — IR invalid |
| **V-UNREACHABLE** | `WorkflowError::UnreachableNode` | Body nodes not reachable from entry | **FATAL** — IR invalid |

---

## 2. Error Propagation Paths

```
YAML string "when"
    │
    ▼
slot_from_text()
    │
    ├── Success → SlotIdx
    │
    └── Error → CompileError::StepFieldShape or ::SlotIndexOutOfRange
                  │
                  ▼
             CompileErrors(vec![...])
                  │
                  ▼
             compile_source() returns Err

───

ChooseBranch.steps (Vec<StepAst>)
    │
    ▼
(per-step lowering)
    │
    ├── Success → CompiledNode[] pushed to builder
    │
    └── Error → CompileError::* (StepIndexOutOfRange, UnsupportedStepPrimitive, etc.)
                  │
                  ▼
             CompileErrors(vec![...])

───

CompiledNodeKind::ChooseSlot { branches, otherwise }
    │
    ▼
vb_validate::shared::validate()
    │
    ├── Success → WorkflowParts valid
    │
    └── Error → WorkflowError (transformed to CompileError by caller)
```

---

## 3. Railway Error Pattern

The lowering function follows a railway pattern:

```
lower_canonical_choose(...) -> Result<(), CompileErrors>

Each step returns Result<_, CompileErrors> or Result<_, CompileError>.
CompileError is converted to CompileErrors(vec![e]) where needed.
All errors are collected into a single CompileErrors wrapper.

No unwrap, no expect, no panic.
```

### Error accumulation design choice:
- **Current:** Single error per failure (first failure aborts lowering for this step)
- **Alternative considered:** Collect all errors across all branches before returning. Rejected for this bead — would require significant refactoring of `CompileErrors` collection. The single-error-fail-fast approach is consistent with the rest of `lower_canonical_step`.

---

## 4. Error Variants NOT Needing New Definitions

The existing `CompileError` enum (169 variants as of writing) already covers all needed error surfaces for nested choose lowering. No new enum variants are required:

- Body width overflow → `StepIndexOutOfRange { value }`
- Unsupported body primitive → `UnsupportedStepPrimitive { step, primitive }`
- Body slot overflow → `SlotIndexOutOfRange { value }`
- Constant parse failure → `UnsupportedConstantValue { step }`
- Body step shape → `StepFieldShape { step, field, expected }`

The existing `UnsupportedStepPrimitive` variant (currently used to reject non-empty choose bodies at lines 253-257) must simply be **removed** for the choose case and replaced with valid body lowering.

---

## 5. Error Distinction: Compile vs. Runtime vs. Validation

| Phase | Type | Recovery |
|---|---|---|
| **Compile** | `CompileError` / `CompileErrors` | Returns `Err` to caller; no partial IR emitted |
| **Validation** | `WorkflowError` | Rejects the IR; cannot proceed to replay |
| **Runtime** | `ReplayError` | Engine handles; may suspend or fail the run |
