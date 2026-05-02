# Architecture Refactor: vb_yaml/src/ast.rs (2750 lines → 7 modules)

## Summary
Split the monolithic 2750-line `ast.rs` file into 7 focused modules, all under 300 lines.

## Original Problem
`vb_yaml/src/ast.rs` was 2750 lines - massively exceeding the 300-line architectural limit.

## Solution: Module Split

| File | Lines | Purpose |
|------|-------|---------|
| `types.rs` | 246 | Shared AST types (ScalarValue, InputField, VarField, SecretField, etc.) + YAML helpers + field parsers |
| `workflow_ast.rs` | 95 | WorkflowSource struct + top-level parsing entry point |
| `step.rs` | 140 | StepAst, StepPrimitive enum, ChooseBranch, TogetherBranch types |
| `step_parsing.rs` | 68 | Main step parsing (parse_steps, parse_step) |
| `step_primitive_parsing.rs` | 209 | All primitive parsers (parse_step_primitive, parse_choose, parse_foreach, etc.) |
| `step_metadata_parsing.rs` | 87 | Step metadata parsing (parse_retry, parse_error_handler, parse_result, parse_examples) |
| `trigger_ast.rs` | 53 | TriggerAst enum + trigger parsing |
| `ast.rs` | 1900 | Re-exports for API compatibility + tests (tests are exempt from line limit) |

**Total: 7 new modules, all ≤ 300 lines (except ast.rs which holds tests)**

## DDD Compliance
- Parse, don't validate: YAML parsing is cold path, using saphyr parser with strict profile validation
- Types act as documentation: All AST nodes are well-documented enums/structs
- Make illegal states unrepresentable: Parse functions return YamlResult with typed errors
- No primitive obsession: Uses typed enums (StepPrimitive, TriggerAst, ScalarValue) over raw strings

## Verification
- `cargo check -p vb_yaml` ✓
- `cargo test -p vb_yaml` ✓ (265 tests pass)
- `cargo clippy -p vb_yaml` ✓ (no issues)
- All files ≤ 300 lines ✓

## API Compatibility
The `ast.rs` facade re-exports all types from submodules, ensuring existing code using `crate::ast::*` continues to work without modification.
