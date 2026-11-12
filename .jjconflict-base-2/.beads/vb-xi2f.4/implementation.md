# Implementation: vb-xi2f.4

## Changes
1. crates/vb_compile/src/mod_compile_lowering/part_01.rs:57
   - `CompiledWorkflow::from_parts_unchecked(parts)` → `CompiledWorkflow::try_from_parts(parts)`
2. crates/vb_compile/Cargo.toml
   - Removed `features = ["test-util"]` from vb_core production dependency
   - Added `vb_core = { path = "../vb_core", features = ["test-util"] }` to dev-dependencies

## Compiler Bug Fixes Exposed by Validation
3. crates/vb_compile/src/mod_compile_lowering/part_03.rs
   - `lower_canonical_parallel`: added `next` parameter, set on TogetherJoin
   - `lower_canonical_collect`: added `next` parameter, set on CollectFinish, link body to CollectPage
   - Removed `builder.max_slot = Some(...)` resets that corrupted slot_count
4. crates/vb_compile/src/mod_compile_lowering/part_04.rs
   - `lower_canonical_aggregate`: added `next` parameter, set on ReduceFinish, link body to ReduceNext
   - `lower_canonical_repeat`: added `next` parameter, set on RepeatFinish, link body to RepeatAttempt

## Safety
- No unsafe code
- No unwrap/expect/panic
- All paths return typed errors


## Source Coverage Matrix

| Requirement | File | Line | Status |
|---|---|---|---|
| REQ-001 | part_01.rs | 57 | covered |
| REQ-002 | workflow/mod.rs | try_from_parts | covered |
