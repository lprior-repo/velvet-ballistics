# Architecture Refactor Report

## Bead: agent/r7-drift-3
## Task: Continue refactoring vb_yaml/src/ast.rs

## Problem
- `ast.rs` was ~2750 lines, containing:
  - Type definitions (~265 lines)
  - Parsing functions (~596 lines)
  - YAML helpers (~87 lines)
  - Test module (~1883 lines, exempt from limit)
- Total source code (excluding tests): ~948 lines

## Solution
Split `ast.rs` into a modular structure under `ast_parse/`:

### New File Structure
```
crates/vb_yaml/src/
├── ast.rs              # 265 lines - Types only
├── ast_helpers.rs      #  96 lines - YAML lookup/field helpers
├── ast_parse/
│   ├── mod.rs          #  16 lines - Module declarations
│   ├── workflow.rs     #  54 lines - Top-level document parsing
│   ├── trigger.rs      #  46 lines - Trigger declaration parsing
│   ├── fields.rs       #  73 lines - inputs/vars/secrets parsing
│   ├── steps.rs        # 270 lines - Step and primitive parsing
│   └── metadata.rs     #  87 lines - retry/error/result/examples
└── tests.rs           # 1888 lines (EXEMPT - test file)
```

### Module Organization
- **ast.rs**: Contains all AST type definitions (WorkflowSource, TriggerAst, StepAst, StepPrimitive, etc.)
- **ast_helpers.rs**: Shared YAML node lookup and field extraction helpers (lookup, require_str, require_str_in, require_scalar_in, opt_str, opt_u32, require_u16)
- **ast_parse/**: Parsing logic organized by domain
  - `workflow.rs`: Entry point (`parse_workflow_ast`) and top-level document parsing
  - `trigger.rs`: Manual/IPC trigger parsing
  - `fields.rs`: inputs, vars, secrets list parsing
  - `steps.rs`: Step parsing and all primitive variants (Set, Save, Do, Choose, ForEach, Together, Collect, Reduce, Repeat, Wait, Ask, Finish)
  - `metadata.rs`: retry, error_handler, result, examples parsing

### Verification
- `cargo check -p vb_yaml`: ✅ Compiles
- `cargo test -p vb_yaml`: ✅ 190 tests pass
- `cargo clippy -p vb_yaml`: ✅ No issues

### Line Count Summary
All source files now comply with the 300-line limit (test files exempt):

| File | Lines | Limit |
|------|-------|-------|
| ast.rs | 265 | ✅ ≤300 |
| ast_helpers.rs | 96 | ✅ ≤300 |
| ast_parse/mod.rs | 16 | ✅ ≤300 |
| ast_parse/workflow.rs | 54 | ✅ ≤300 |
| ast_parse/trigger.rs | 46 | ✅ ≤300 |
| ast_parse/fields.rs | 73 | ✅ ≤300 |
| ast_parse/steps.rs | 270 | ✅ ≤300 |
| ast_parse/metadata.rs | 87 | ✅ ≤300 |
| tests.rs | 1888 | EXEMPT |
