# Error Taxonomy: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Railway Error Structure

```
Result<CompiledWorkflow, CompileErrors>
  └── CompileErrors(Vec<CompileError>)
        ├── CompileError::Workflow(WorkflowError)
        │      └── (structural validation failures from try_from_parts)
        ├── CompileError::Validation(vb_validate::ValidationError)
        │      └── (contract completeness / idempotency gate failures)
        └── ... (parse, schema, reference, taint, lowering errors)
```

## Error Categories

### 1. Parse & Profile Errors (Pre-Lowering)
- `Utf8`, `Parse`, `EmptySource`, `SourceTooLarge`
- `CanonicalYaml`, `DocumentCount`, `TopLevelNotMapping`
- `DepthLimit`, `NodeLimit`, `SequenceLimit`, `MappingLimit`, `ScalarLimit`
- YAML feature rejections: `NonStringKey`, `DuplicateKey`, `AliasForbidden`, `AnchorForbidden`, `MergeKeyForbidden`, `TagForbidden`, `FloatForbidden`, `BadValue`

### 2. Schema & AST Errors (Pre-Lowering)
- `MissingField`, `UnknownTopLevelField`, `InvalidVersion`
- `InvalidTriggerCount`, `UnknownTriggerKind`, `TriggerShape`, `UnknownTriggerField`, `MissingTriggerField`, `InvalidTriggerField`
- `FieldShape`, `InvalidInputSchema`, `UnknownInputSchemaField`
- `EmptySteps`, `MissingStepId`, `DuplicateStepId`, `InvalidName`
- `StepShape`, `UnknownStepField`, `UnknownStepPrimitiveField`, `MissingStepPrimitive`, `MultipleStepPrimitives`
- `UnsupportedStepPrimitive`, `UnsupportedStepControlField`, `MissingStepField`, `StepFieldShape`
- `DuplicateOutputName`, `UnknownOutputName`, `UnsupportedTopLevelResult`, `LastStepMustFinish`

### 3. Reference & Taint Errors (Pre-Lowering)
- `UnknownReferenceRoot`, `IllegalReference`, `UnknownReferenceName`
- `UnsupportedAccessorReference`
- `TypeMismatch`, `UnknownSlotType`
- `SecretTaintLeak`
- `BackwardBranchTarget`, `UnknownStepTarget`, `UnreachableStep`

### 4. Expression Errors (Lowering)
- `ExpressionUnexpectedChar`, `ExpressionUnterminatedString`
- `ExpressionIntegerOutOfRange`, `ExpressionFloatOutOfRange`
- `ExpressionLimitExceeded`, `ExpressionUnexpectedToken`
- `ExpressionUnknownIdentifier`, `ExpressionLoweringUnsupported`, `ExpressionHelperArity`

### 5. Index & Limit Errors (Lowering)
- `StepIndexOutOfRange`, `SlotIndexOutOfRange`, `BranchTargetOutOfRange`
- `PrimitiveLoweringLimitExceeded`

### 6. Structural Validation Errors (Emission — **THIS BEAD**)
Surfaced via `try_from_parts` → `CompileError::Workflow(...)`:

| Error | Meaning | Typical Cause |
|-------|---------|---------------|
| `EmptyNodes` | Compiler emitted zero nodes | Empty steps not caught earlier |
| `EntryOutOfBounds` | Entry step >= node count | Layout miscalculation |
| `StepOutOfBounds` | Node target >= node count | Branch/loop target overflow |
| `SlotOutOfBounds` | Slot reference >= slot_count | Slot allocator bug |
| `ConstOutOfBounds` | Const reference >= constants.len() | Constant table builder bug |
| `NodeIdMismatch` | Node.id != table index | Node sequencing bug |
| `Expression(...)` | Expr program invalid | Expression lowering bug |
| `ResourceContractExceeded` | Actual > declared limit | Resource tracking bug |
| `ResourceContractTooLarge` | Declared > hard limit | Contract generation bug |
| `EmptyBranchTable` | Choose with no branches and no otherwise | Lowering omission |
| `UnreachableNode` | Node not reachable from entry | Control flow lowering bug |
| `BackwardEdge` | Non-loop edge points backward | Layout bug |
| `ImproperLoopNesting` | Inner loop spans outer loop done | Loop scaffolding bug |
| `BudgetPolicyExceeded` | Whole-workflow budget overflow | Budget computation bug |
| `StepCountOverflow` | Budget step count overflowed u64 | Extreme workflow / bug |
| `SymbolOutOfBounds` | SymbolId >= symbols_count | Symbol table bug |
| `AccessorPathTooDeep` | Path depth > MAX_PATH_DEPTH | Deep accessor lowering bug |
| `JumpCycle` | Jump creates execution cycle | Control flow lowering bug |

### 7. Contract & Idempotency Errors (Post-Compilation)
- `Validation(...)` — action contract completeness, expression stack, slot cycles.
- `IdempotencyViolation` — unsafe side-effect + retry configuration.

## Error Mapping Strategy

`WorkflowError` from `try_from_parts` maps to `CompileError::Workflow` via `#[from]`.
This makes structural validation failures first-class compile diagnostics with stable codes (`INVALID_COMPILED_WORKFLOW`, `LIMIT_EXCEEDED`, etc.).

## Semantic Errors vs. Panic

- Before fix: structural invalidity would reach runtime as logic errors or panics.
- After fix: structural invalidity is a **semantic compile error** with human-readable message and machine-readable code.
