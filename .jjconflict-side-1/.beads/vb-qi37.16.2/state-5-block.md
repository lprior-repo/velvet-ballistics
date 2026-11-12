bead_id: vb-qi37.16.2
phase: state-5
classification: BLOCK_RESOLVED
owner_state: 5
rerun_from: 5

# Block Resolution

## Original Block

Verification command:

```bash
rtk cargo test --package vb_runtime --test durable_resume_red_phase
```

Result: **24 compile errors** before repair.

Observed local failures:
- `Shard` imported from private `vb_runtime::shard::lifecycle` path.
- `RuntimeError::RunIdNotFound` matched where expression type is `ResumeError`.
- Direct access to private `Shard.runs`.
- `ConstValue::new(0)` does not exist.

## Resolution

All compile errors have been fixed. Tests now compile and run in RED phase.

### Fixes Applied

1. **Import fix:** `use vb_runtime::shard::types::Shard` (was `vb_runtime::shard::lifecycle::Shard`)
2. **Error type fix:** Match `ResumeError::RunIdNotFound` not `RuntimeError::RunIdNotFound`
3. **Private field access:** Moved tests requiring `Shard.runs` to internal `#[cfg(test)]` module in `lifecycle.rs`
4. **ConstValue fix:** Changed `ConstValue::new(0)` to `ConstIdx::new(0)` for `CompiledNodeKind::SetConst`

### Current State

```bash
rtk cargo test --package vb_runtime --test durable_resume_red_phase
```

**Result: 9 passed; 8 failed** — Tests compile and fail for intended behavioral gaps.

Internal tests:
```bash
rtk cargo test --package vb_runtime --lib -- shard::lifecycle::tests::resume_post003
rtk cargo test --package vb_runtime --lib -- shard::lifecycle::tests::resume_inv001
```

**Result: 3 passed** — Private field access tests work correctly.

## Conclusion

Block is RESOLVED. State 5 RED-phase evidence is valid. Tests compile and demonstrate correct RED phase behavior (implementation gaps cause test failures, not compile errors).

Ready for State 6 (implementation).