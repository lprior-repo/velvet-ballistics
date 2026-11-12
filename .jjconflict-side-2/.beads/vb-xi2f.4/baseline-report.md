# Baseline Report: vb-xi2f.4

## Baseline Commit
main@a97b03fb

## Scope
Compiler emission path in vb_compile crate.

## Current State
- One unchecked emission site: `crates/vb_compile/src/mod_compile_lowering/part_01.rs:57`
- `from_parts_unchecked` exposed via `test-util` feature in production dependency
- `try_from_parts` already exists and is used in other compile paths
- Validation infrastructure complete (WorkflowError, vb_validate gates)

## Tests
- moon ci passes at baseline
- 7900+ tests green
