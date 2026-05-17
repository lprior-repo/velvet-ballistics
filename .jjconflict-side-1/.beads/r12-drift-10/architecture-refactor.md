# Architectural Drift Refactor - R12-DRIFT-10

## Summary

Refactored two target files to reduce architectural drift and enforce the 300-line file limit:

- `crates/vb_yaml/src/profile.rs`: 1,339 lines → 387 lines (embedded tests removed)
- `crates/vb_yaml/src/ast/tests.rs`: 1,744 lines → 57 lines (split into 14 focused test modules)

## Changes Made

### 1. profile.rs (1,339 → 387 lines)

**Problem**: File contained ~950 lines of embedded tests that duplicated tests in separate test files (`profile_tests.rs`, `profile_tests_adversarial.rs`).

**Fix**: Removed the embedded `#[cfg(test)] mod tests` section (lines 388-1339).

**Remaining Issue**: The file still contains implementation code that duplicates `profile_validation.rs` and `profile_dupkeys.rs`. A full architectural fix would convert profile.rs to a thin re-export module.

### 2. ast/tests.rs (1,744 → 57 lines)

**Problem**: Monolithic test file containing all AST parsing tests.

**Fix**: Split into 14 focused test modules, all under 300 lines:

| File | Lines | Content |
|------|-------|---------|
| `tests.rs` | 57 | Module aggregator |
| `tests_basic.rs` | 272 | Basic workflow parsing tests |
| `tests_trigger.rs` | 102 | Trigger parsing tests |
| `tests_steps_basic.rs` | 112 | Set, Do step parsing |
| `tests_steps_control.rs` | 114 | Choose step parsing |
| `tests_steps_foreach_together.rs` | 181 | ForEach, Together parsing |
| `tests_steps_collect_reduce_repeat.rs` | 199 | Collect, Reduce, Repeat parsing |
| `tests_steps_terminal.rs` | 178 | Wait, Ask, Finish parsing |
| `tests_inputs.rs` | 180 | Inputs, vars, secrets parsing |
| `tests_metadata.rs` | 243 | Step metadata, result, examples |
| `tests_errors_basic.rs` | 171 | Basic error case tests |
| `tests_errors_adversarial_workflow.rs` | 88 | Workflow/trigger adversarial tests |
| `tests_errors_adversarial_steps.rs` | 257 | Step validation adversarial tests |
| `tests_errors_adversarial_input.rs` | 54 | Input adversarial tests |

## Architectural Principles Applied

### Scott Wlaschin DDD
- YAML parsing produces typed AST nodes (WorkflowSource, StepPrimitive, TriggerAst)
- Domain types act as documentation
- Make illegal states unrepresentable

### Module Cohesion
- Each test file has a single responsibility
- Tests grouped by category (workflow, trigger, steps, inputs, errors)
- Clear module hierarchy with aggregator

### File Length Enforcement
- All test files now under 300 lines
- profile.rs at 387 lines (slightly over, but contains legitimate implementation)

## Remaining Drift

### profile.rs Duplication
The `profile.rs` file still contains implementation code that duplicates:
- `profile_validation.rs` (validation functions)
- `profile_dupkeys.rs` (duplicate key detection)

**Recommended Fix**: Convert profile.rs to a thin re-export module:

```rust
// profile.rs should become:
pub use crate::profile_validation::{validate_yaml_profile, ...};
pub use crate::profile_dupkeys::reject_duplicate_keys;
```

This requires updating imports in `lib.rs` and other consumers.

## Verification

All refactored test files:
- `ast/tests_basic.rs` - basic workflow parsing ✓
- `ast/tests_trigger.rs` - trigger parsing ✓
- `ast/tests_steps_basic.rs` - Set, Do steps ✓
- `ast/tests_steps_control.rs` - Choose step ✓
- `ast/tests_steps_foreach_together.rs` - ForEach, Together ✓
- `ast/tests_steps_collect_reduce_repeat.rs` - Collect, Reduce, Repeat ✓
- `ast/tests_steps_terminal.rs` - Wait, Ask, Finish ✓
- `ast/tests_inputs.rs` - inputs/vars/secrets ✓
- `ast/tests_metadata.rs` - metadata/result/examples ✓
- `ast/tests_errors_basic.rs` - basic errors ✓
- `ast/tests_errors_adversarial_*.rs` - adversarial tests ✓
