# Architectural Drift Hammer - Master Summary

## Overview

5 rounds × 20 agents = 100 agent invocations across 96 unique files.
JJ Workspace: `arch-drift-hammer` at `/home/lewis/src/velvet-ballistics/arch-drift-hammer`

## The Catastrophe

**EVERY SINGLE PRODUCTION FILE >300 LINES IS A VIOLATION**

### Top 20 Worst Offenders (by line count)

| File | Lines | Ratio to 300 |
|------|-------|--------------|
| vb_cli/src/app_impl.rs | 6296 | 20.9x |
| vb_ipc/src/server/handlers.rs | 3998 | 13.3x |
| vb_cli/src/args.rs | 2969 | 9.9x |
| vb_validate/src/gates.rs | 2894 | 9.6x |
| vb_runtime/src/runtime.rs | 2718 | 9.1x |
| vb_core/src/budget.rs | 2716 | 9.1x |
| vb_core/src/value_store.rs | 2552 | 8.5x |
| vb_compile/src/expression_bytecode.rs | 2533 | 8.4x |
| vb_runtime/src/shard/helpers.rs | 2492 | 8.3x |
| vb_core/src/diagnostic.rs | 2445 | 8.2x |
| vb_core/src/action.rs | 2287 | 7.6x |
| vb_validate/src/schema.rs | 2195 | 7.3x |
| vb_core/src/replay/ops.rs | 2101 | 7.0x |
| vb_core/src/frame.rs | 2081 | 6.9x |
| vb_core/src/errors.rs | 2055 | 6.9x |
| vb_runtime/src/admission.rs | 1970 | 6.6x |
| vb_runtime/src/engine/execute.rs | 1910 | 6.4x |
| vb_core/src/workflow/mod.rs | 1909 | 6.4x |
| vb_runtime/src/primitives/retry.rs | 1686 | 5.6x |
| vb_storage/src/recovery/replay/summary.rs | 1576 | 5.3x |

## Recurring Violations

### 1. LINE COUNT (>300)
**Every single production file analyzed violated this rule.** The largest ratio was 20.9x (app_impl.rs).

### 2. Primitive Obsession (HIGH)
- `u64` used for `RunId`, `TicketId`, `SequenceNumber` without newtype wrappers
- `u16` used for `StepIdx`, `SlotIdx`, `ActionId` without validation
- `&str` used for field names, error messages, enum variants
- `[u8; 32]` used for digests without `Digest` type
- Raw `usize` for indices, offsets, capacities

### 3. Test Pollution (CRITICAL)
**Most files had 60-80% of their lines as inline `#[cfg(test)]` modules.**
These should all be moved to `workspace_tests/` or `tests/` directories.

### 4. "helpers.rs" DDD Smell
`vb_runtime/src/shard/helpers.rs` (2492 lines) - the name "helpers" indicates missing domain concepts.

### 5. Parse, Don't Validate Violations
Multiple `is_valid_*` functions that validate after parsing instead of failing at parse site.

## Priority Refactoring Targets

### P0 (CATASTROPHIC - Do First)
1. **vb_cli/src/app_impl.rs** - Split into command modules (validate/, run/, compile/, ipc/, etc.)
2. **vb_cli/src/args.rs** - Split by command group with typed args
3. **vb_ipc/src/server/handlers.rs** - Extract by handler type
4. **vb_validate/src/gates.rs** - Split by gate number (gate_7, gate_8, etc.)

### P1 (CRITICAL)
5. **vb_runtime/src/runtime.rs** - Extract scheduling, lifecycle, timer
6. **vb_core/src/budget.rs** - Split by budget computation phase
7. **vb_core/src/diagnostic.rs** - Extract code registry
8. **vb_core/src/value_store.rs** - Extract tests, split domain

### P2 (HIGH)
All files with >500 lines - split tests out, extract domain modules.

## Common Patterns Found

### Pattern 1: Inline Tests
```rust
// VIOLATION: 800 lines of tests inline in 200 line production file
#[cfg(test)]
mod tests { ... }
```
**Fix**: Move to `workspace_tests/` or `tests/` directory.

### Pattern 2: God Functions
```rust
// VIOLATION: 300 line function doing 7 different things
fn god_function(...) { ... }
```
**Fix**: Split by responsibility, use router pattern.

### Pattern 3: Primitive Collections
```rust
// VIOLATION: Vec<(u16, String, u64)> instead of typed structs
struct Data(Vec<(u16, String, u64)>);
```
**Fix**: Create domain types with named fields.

## Newtypes Required (Common)

- `RunId(u64)` - already exists in some places
- `StepIdx(u16)` - already exists in some places
- `SlotIdx(u16)` - already exists in some places
- `ActionId(u16)` - already exists in some places
- `TicketId(u64)` - MISSING
- `SequenceNumber(u64)` - MISSING
- `Digest([u8; 32])` - MISSING
- `TimestampMs(u64)` - MISSING
- `ShardIndex(u32)` - MISSING
- `AttemptCount(u16)` - MISSING

## Reports Generated

96 detailed reports in `arch-drift-reports/`:
- `app_impl_hammer.md` through `workflow_mod_hammer.md`
- Each report contains: line count violation, primitive obsession violations, recommended split, DDD violations

## Next Steps

1. **Move all tests** from production files to `workspace_tests/`
2. **Extract god functions** into domain-specific modules
3. **Create newtypes** for all primitive obsessisons
4. **Split files >300 lines** into focused modules
5. **Enforce <300 line rule** in CI

## Status

**ARCHITECTURAL DRIFT CONFIRMED - ZERO EXCEPTIONS REQUIRED**

Generated: 2026-05-29
Agents: 100 (5 rounds × 20 agents)
Reports: 96
Files Analyzed: 96 unique production files
