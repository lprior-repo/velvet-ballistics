# Codebase Map — vb-core-lower-values-actions-refs

bead_id: vb-core-lower-values-actions-refs
bead_title: compiler: Lower v1 values actions and references
phase: 2
updated_at: 2026-05-15T00:00:00Z

## Scope Summary

Implement and test YAML AST to numeric IR lowering for:
- YAML AST values (expressions, literals, references)
- Action references
- Capability references
- Slot references
- Accessors
- Taint metadata

The goal: runtime core receives numeric/handle data only; author YAML no longer requires low-level slots/actions.

## Crate Map

### vb_yaml (cold YAML parsing / AST boundary)

| File | Purpose |
|------|---------|
| `src/ast/types.rs` | Typed AST: `WorkflowSource`, `StepAst`, `StepPrimitive` (Set, Save, Do, Choose, ForEach, Together, Collect, Reduce, Repeat, Wait, Ask, Finish), `ScalarValue`, `ChooseBranch`, `TogetherBranch`, `RetryPolicy`, `ErrorHandlerAst`, `InputField`, `VarField`, `SecretField`, `ResultMapping`, `ExampleAst` |
| `src/ast/mod.rs` | Module re-exports |
| `src/ast_parse/mod.rs` | `parse_workflow_ast` entry, submodules (workflow, trigger, fields, steps, metadata) |
| `src/ast_parse/steps.rs` | Step/primitive parsing |
| `src/ast_parse/fields.rs` | Input/var/secret field parsing |
| `src/lib.rs` | `parse_profile`, `validate_strict_profile` |
| `src/events.rs` | YAML event types |
| `src/events_conv.rs` | Event conversion |
| `src/expression.rs` | Expression parsing from YAML scalars |
| `src/profile.rs` | Profile validation |
| `src/source_map.rs` | Source location tracking |

### vb_compile (cold compiler — primary target)

| File | Purpose |
|------|---------|
| `src/lib.rs` (l.1-4351) | Facade: `YamlCompiler`, `compile_workflow`, `lower_set/do/choose/for_each/together/collect/reduce/repeat/wait/ask/finish`, `SlotCompiler` struct, `lower_steps_to_ir`, `compile_to_generated_rust`, `emit_compiled_artifact`. Key types: `CompileError`, `CompileErrors`, `SourceMark`, `YamlLimits` |
| `src/ast/types.rs` | Cold AST: `WorkflowAst`, `StepAst`, `StepKindAst`, `StepPrimitiveAst`, `AstValue` (Null/Bool/I64/Text/Reference/Sequence/Mapping), `AstExpression` (Slot//Reference/Parsed/Literal), `AstMapEntry` |
| `src/ast/parse.rs` | `parse_workflow_ast` — converts saphyr doc → cold AST |
| `src/references.rs` | `validate_workflow_ast`, builds `RefTables` from cold AST, collects refs from values/expressions/steps |
| `src/expression.rs` | Cold lexer/ParsedExpression: `Literal`, `Reference`, `Unary`, `Binary`, `HelperCall`; `ExpressionHelper`, `ExpressionLiteral`, `UnaryOp`, `BinaryOp` |
| `src/expression_bytecode.rs` | Expression → bytecode lowering: `compile_expr_to_bytecode`, `compile_expr_to_bytecode_with_accessors`, `lower_slot_reference`, `lower_accessor_reference`, `lower_expr`, `lower_literal`, `lower_unary`, `lower_binary`, `lower_helper`. Two resolvers: `RejectingReferenceResolver` (rejects refs) and `SlotAccessorReferenceResolver` (produces `AccessorProgram`s) |
| `src/control_flow.rs` | Control flow validation on cold AST |
| `src/type_taint.rs` | Type/taint validation on cold AST |
| `src/schema.rs` | Input schema validation |
| `src/strict_yaml.rs` | Strict YAML profile enforcement |
| `src/lower/mod.rs` | Re-exports: `lower_ask`, `lower_choose`, `lower_collect`, `lower_do`, `lower_finish`, `lower_for_each`, `lower_reduce`, `lower_repeat`, `lower_set`, `lower_steps_to_ir`, `lower_together`, `lower_wait`, `SlotCompiler`, `WaitKind` |
| `src/lower/tests.rs` | Lower tests (49.3K) |

### vb_core (hot runtime — receives numeric IR only)

| File | Purpose |
|------|---------|
| `src/nodes.rs` | `CompiledNode`, `CompiledNodeKind` (Nop/SetConst/Copy/EvalExpr/BuildObject/BuildList/Do/Choose/ChooseSlot/ForEachStart/ForEachNext/ForEachJoin/TogetherStart/TogetherBranch/TogetherJoin/CollectStart/CollectPage/CollectNext/CollectFinish/ReduceStart/ReduceNext/ReduceFinish/RepeatStart/RepeatAttempt/RepeatCheck/RepeatFinish/WaitUntil/WaitEvent/Ask/AskResume/RetryCheck/ErrorHandler/Jump/Finish) |
| `src/ids/mod.rs` | Numeric IDs: `StepIdx(u16)`, `SlotIdx(u16)`, `ExprIdx(u16)`, `ConstIdx(u16)`, `ActionId(u16)`, `AccessorIdx(u16)`, `SymbolId(u32)`, `ListId`, `ObjectId`, `BlobId`, `RunId`, `EventSeq`, `SeqNo`, `WorkflowDigest` |
| `src/ids/kani_id_bounds.rs` | Kani bounds proofs |
| `src/compiled_workflow.rs` | `CompiledWorkflow`, `WorkflowParts` (slot_count, symbols_count, nodes, expressions, accessors, constants, entry, step_names) |
| `src/value.rs` | Runtime value types |
| `src/value_store.rs` | Slot-based value store |
| `src/action.rs` | `ActionContract`, `ActionId`, `SideEffect`, `Idempotency`, `RetrySafety` |
| `src/accessors.rs` | `AccessorProgram`, `PathSegment` (Field/Index) |
| `src/capability.rs` | Capability types |
| `src/expressions.rs` | `ExprProgram`, `ExprOp` bytecode ops |
| `src/workflow/mod.rs` | `CompiledNode`, `SlotBranch`, `ExprBranch` |

### vb_validate (validation gates on WorkflowParts)

| File | Purpose |
|------|---------|
| `src/lib.rs` | `validate`, `validate_with_contracts`, shared validation entry |
| `src/shared.rs` | Shared validation pipeline (gates 7-15) |
| `src/references.rs` | `RefTables`, `validate_single_reference` — validates `$input.*`, `$vars.*`, `$secrets.*`, `$step.*` references; shared with `vb_compile` per DRIFT-5 |
| `src/gate_08_accessor.rs` | Gate 8: accessor path segment symbol validation |
| `src/gate_09_slots.rs` | Gate 9: slot reference bounds validation |
| `src/gate_10_node.rs` | Gate 10: node validity |
| `src/gate_11_loop.rs` | Gate 11: loop validity |
| `src/type_taint.rs` | `ValueType`, `Taint`, `ValueFact`; type inference and secret taint propagation |
| `src/type_taint_tests.rs` | 121.6K — comprehensive taint propagation tests |
| `src/taint_prop.rs` | Taint propagation logic |
| `src/fact_table.rs` | Fact table for type/taint tracking |
| `src/schema.rs` | Schema validation |
| `src/schema_fields.rs` | Field schema validation |

### vb_expr (expression engine)

| File | Purpose |
|------|---------|
| `src/lib.rs` | Expression evaluation engine (hot path) |

## Key Type Relationships

```
YAML source
  ↓ vb_yaml::ast_parse::parse_workflow_ast
WorkflowSource (vb_yaml/ast/types.rs)
  ↓ vb_compile::ast::parse_workflow_ast
WorkflowAst (vb_compile/ast/types.rs) — cold AST with string names
  ↓ vb_compile references + type_taint + control_flow validation
WorkflowParts (vb_core/compiled_workflow.rs) — numeric IR
  ↓ vb_validate::shared::validate
CompiledWorkflow (vb_core/compiled_workflow.rs) — validated hot IR
```

## Lowering Chain for Bead Focus Areas

### Values → ConstIdx
- `AstValue::Null/Bool/I64/Text` → `ConstValue` → `SlotCompiler::push_constant` → `ConstIdx`
- `AstValue::Reference` → validated via `RefTables` → `lower_slot_reference` → `ExprOp::LoadSlot` or `AccessorProgram`

### Expressions → ExprIdx
- `AstExpression::Slot(SlotIdx)` → `ExprOp::LoadSlot`
- `AstExpression::Reference` → `lower_slot_reference` or `lower_accessor_reference` → `AccessorProgram`
- `AstExpression::Parsed(ParsedExpression)` → `compile_expr_to_bytecode` → `ExprProgram` → `ExprIdx`

### Action References → ActionId
- `StepKindAst::Run { action: ActionId, input: SlotIdx }` → `lower_do` → `CompiledNodeKind::Do { action, input }`

### Slot References → SlotIdx
- `SlotCompiler::record_slot` tracks max slot
- `lower_set/do/choose/for_each/together/collect/reduce/wait/ask/finish` all call `builder.record_slot(slot)`
- Gate 9 (`gate_09_slots.rs`) validates all slot references against `slot_count`

### Accessors → AccessorProgram
- `compile_expr_to_bytecode_with_accessors` produces `AccessorProgram` list
- `AccessorProgram { root: SlotIdx, path: Box<[PathSegment]> }`
- Gate 8 (`gate_08_accessor.rs`) validates accessor path segments

### Taint Metadata
- `vb_validate/type_taint.rs`: `Taint` (Clean/Secret), `ValueType`, `ValueFact`
- `vb_validate/taint_prop.rs`: taint propagation through steps
- `vb_core/kani_taint.rs`: Kani harnesses for taint proofs
- Lowering must preserve taint: `ValueFact` carried through `FactTable`

## Risk Tags

- `temporal`: ForEach/Together/Collect/Reduce loops have complex continuation semantics
- `concurrency`: Together branches execute concurrently; Join node must see all results
- `unsafe/UB`: No unsafe in vb_compile/vb_core; all safe Rust with `#![forbid(unsafe_code)]`
- `persistence`: WorkflowParts is postcard-serialized for hot runtime loading
- `parser/codec`: YAML → AST → IR is the core pipeline; expression bytecode encoding is critical
- `dependency`: vb_compile depends on vb_yaml (parse), vb_core (IR types), vb_validate (shared refs)
- `performance`: Expression bytecode must be bounded; `ExprProgram::try_from_ops` enforces op count limits
- `public_api`: `YamlCompiler::compile`, `lower_steps_to_ir`, `SlotCompiler` are public API
- `migration`: v1 primitives from YAML AST to numeric IR (this bead); vb-f04l is the blocker

## Blockers

- **vb-f04l** ("compiler: Safe v1 primitive source lowering") is a `blocks` dependent. vb-f04l implements the primitive source lowering that this bead depends on for the lowering infrastructure. Specifically, vb-f04l handles the safe AST-to-IR lowering for all v1 primitives.

## Open Questions

1. `vb_compile/src/lower/tests.rs` (49.3K) — contains existing lowering tests. Need to read before writing new tests.
2. The `SlotCompiler::build_parts` sets `symbols_count: 0` — symbol table is not yet built by lowering. This may need to be addressed.
3. `vb_validate/type_taint_tests.rs` (121.6K) — large test suite for taint propagation. What taint tests exist for the lowered IR?
4. Capability references — `vb_core/capability.rs` exists but no clear capability reference lowering in vb_compile yet. Need to investigate.

## Downstream Owners (from artifact headers)

- `rust-contract`: requirements, assumptions, invariants for the lowering contract
- `proof-planner`: verifier lanes for bytecode bounds, slot index bounds, action ID validity
- `test-planner`: derive tests from contract + existing lowering tests
- `holzman-rust`: implement safe lowering functions in vb_compile
- `formal-verifier`: Kani for slot bounds, taint propagation; potentially Verus for IR invariants
