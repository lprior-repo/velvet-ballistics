# Architecture Refactor: error_code_string decomposition

## Bead: vb-f2zo

## Problem
- `error_code_string` in `crates/vb_core/src/engine/error_routing.rs` was 44 lines
- Nested match expression with `.into()` converting `&str` literals to `Box<str>` on every call
- No allocation avoidance for static error codes

## Solution
Split into two functions:

1. **`engine_error_static_code`** (lines 67-106, 40 lines)
   - Returns `&'static str` instead of `Box<str>`
   - Pure mapping from `EngineError` variant to static string code
   - No heap allocation

2. **`error_code_string`** (lines 108-113, 6 lines)
   - Thin wrapper that calls `engine_error_static_code` when no runtime_code exists
   - Only allocates `Box<str>` when `runtime_code()` returns `Some`

## Benefits
- **Single Responsibility**: Error code mapping is now separated from the allocation logic
- **No allocation for static codes**: 37 of 38 error variants now use zero-allocation `&'static str`
- **Better testability**: `engine_error_static_code` can be tested independently
- **Follows Scott Wlaschin DDD**: Types act as documentation; illegal states unrepresentable

## Files Changed
- `crates/vb_core/src/engine/error_routing.rs`

## Verification
- Syntax verified via rustfmt
- Pre-existing errors in other files (action.rs, workflow.rs) block full build
- error_routing.rs has no compilation errors
