# Domain Model Review — vb-core-lower-values-actions-refs

## Domain Objects

| Domain Object | IR Representation | Notes |
|---|---|---|
| YAML scalar value | `ConstValue::Null\|Bool\|I64\|F64\|Text` | No `Reference` in constant pool |
| Slot reference `$slot.N` | `ExprOp::LoadSlot(SlotIdx::new(N))` | Direct, no accessor |
| Numeric accessor `$slots.N.P` | `ExprOp::LoadAccessor(AccessorIdx::new(K))` + `AccessorProgram { root: SlotIdx::new(N), path: [Index(P)] }` | Path segments are `u32` indices only |
| Expression | `ExprProgram { ops: Box<[ExprOp]>, max_stack: u8 }` | Postfix bytecode |
| Action reference | `CompiledNodeKind::Do { action: ActionId, input: SlotIdx }` | ActionId is `u16` |
| Taint metadata | Carried through `Taint::Clean\|Secret` in `ValueFact` | Validated before lowering; not re-checked in IR |

## Review Findings

### Finding 1: Slot Index Overflow Prevention — ADEQUATE

`SlotCompiler::record_slot` uses `checked_add` and `try_into(u16)` to prevent overflow. The `SlotIdx` newtype wraps `u16` directly.

**Evidence**: `crates/vb_compile/src/lib.rs:877-888` — `slot_count()` returns `Err(SlotIndexOutOfRange)` on overflow.

### Finding 2: Expression Bytecode Bounds — ADEQUATE

`ExprProgram::try_from_ops` enforces both `MAX_EXPRESSION_OPS` (op count) and `MAX_EXPRESSION_STACK` (stack depth). Stack effects are validated per-op with underflow/overflow checks.

**Evidence**: `crates/vb_core/src/expressions.rs:101-113` — `check_expr_stack_bound` computes exact stack depth.

### Finding 3: Numeric-Only Accessor Paths — INTENTIONAL

`expression_bytecode.rs:154-165` (`numeric_path_segments`) only accepts `u32` index segments. Field-name accessors are rejected with `UnsupportedAccessorReference`.

**Evidence**: `crates/vb_compile/src/expression_bytecode.rs:168-178` — `parse_list_index_segment` fails on non-numeric input.

### Finding 4: Taint Metadata Preservation — ORDER-GUARANTEED

The pipeline runs `type_taint::validate_workflow_ast` before `build_workflow_parts` (lowering). `SecretTaintLeak` is caught at compile time; lowering does not re-check taint but the ordering guarantees no secret leaks reach runtime.

**Evidence**: `crates/vb_compile/src/lib.rs:163-164` — `references::validate_workflow_ast` and `type_taint::validate_workflow_ast` run before `build_workflow_parts`.

### Finding 5: symbols_count = 0 — ACKNOWLEDGED GAP

`SlotCompiler::build_parts` sets `symbols_count: 0`. This means no symbol table exists in v1 IR. Field-name-based accessors are blocked by design.

**Evidence**: `crates/vb_compile/src/lib.rs:901`.

### Finding 6: Blocker vb-f04l Dependency — BLOCKING

This bead cannot complete lowering infrastructure until `vb-f04l` provides safe primitive source lowering. The `lower_*` functions in `lib.rs` depend on vb-f04l's AST-to-IR lowering entry points.

**Evidence**: `bd show vb-f04l` — DEPENDS ON this bead; BLOCKS vb-ahfl and vb-core-cli-accepted-path.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Slot overflow in deeply nested primitives | Low | High | `checked_add` + `u16::try_from` enforced in `slot_count()` |
| Expression stack overflow | Low | Medium | `MAX_EXPRESSION_STACK` bounded at 255; enforced at `ExprProgram::try_from_ops` |
| Accessor path too deep | Low | Medium | Gate 8 (`gate_08_accessor.rs`) validates depth; `MAX_ACCESSOR_DEPTH` in `vb_core` |
| Constant pool overflow | Medium | Medium | 65536 constant limit; `try_from(u16)` in `push_constant` |
| Taint leak through lowering | Low | Critical | Pipeline ordering (validate-then-lower) prevents this |
| vb-f04l unavailability | High | High | This bead IS the blocker; must unblock first |

## Conclusion

The domain model is sound. Key properties:

1. All numeric handles (`SlotIdx`, `StepIdx`, `ExprIdx`, `ConstIdx`, `ActionId`, `AccessorIdx`) are `u16`-bounded
2. Expression bytecode is provably bounded postfix
3. Taint metadata preservation is order-guaranteed
4. Accessor paths are restricted to numeric indices in v1

**Status**: Domain model is ready for contract verification.
