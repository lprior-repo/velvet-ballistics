# vb-yd5x STATE

- Current State: State 8 (Post-review Machine-Gate Repair)
- Title: validate/compile: Prove shared validated IR usage
- Branch/Workspace: `/home/lewis/src/Velvet-ballistics`
- Claim Evidence: `bd update vb-yd5x --claim` succeeded from `/home/lewis/src/Velvet-ballistics`

## State Progression

- State 1.5 (Test Planning): test-plan.md written, covering RED-phase and GREEN-phase scenarios
- State 2 (TDD Red Phase): RED-phase tests written proving contract gap
- State 3 (TDD Green Phase): Implementation complete, tests pass
- State 4 (Refactor): Code reviewed, `lower_steps_to_ir` calls `vb_validate::shared::validate` before `try_from_parts`
- State 5 (Full CI): All tests pass, clippy clean, `moon ci` green
- State 6 (Implementation Review): Implementation matches contract signatures
- State 7 (Black-Hat Review): No unsafe/unwrap/panic, no hot-path YAML/JSON/HTTP
- State 8 (Post-review Machine-Gate Repair): Verification complete

## Implementation Evidence

- `lower_steps_to_ir` (lib.rs:280): `vb_validate::shared::validate(&parts).map_err(|e| CompileErrors(vec![e.into()]))?;` runs BEFORE `CompiledWorkflow::try_from_parts(parts)`
- `validate_ir` (lib.rs:702-705): Same ordering guarantee
- `compile_workflow_with_contracts` (lib.rs:215-225): Compiles, then `validate_with_contracts`, then idempotency gates
- Test file: `crates/vb_compile/src/tests/test_22.rs` + inline in `lib.rs #[cfg(test)]`

## Test Results

```
moon test: 10860 tests, 0 failed (46s)
cargo test -p vb_compile --lib: 246 passed (1 suite, 2.22s)
Key vb-yd5x tests:
- lower_steps_to_ir_bypasses_gate_9_slot_reference_validation: PASS
- validate_ir_orders_shared_validation_before_core: PASS
- validate_ir_output_passes_shared_validation: PASS
- lower_steps_to_ir_output_passes_shared_validation: PASS
- compile_workflow_with_contracts_rejects_missing_action_contract: PASS
- compile_workflow_with_contracts_rejects_orphan_action_contract: PASS
- compile_workflow_with_contracts_accepts_valid_action_contract: PASS
```

## Exit Criteria Status

All exit criteria from test-plan.md are satisfied:
- [x] `lower_steps_to_ir` calls `vb_validate::shared::validate` before `try_from_parts`
- [x] `validate_ir` ordering guaranteed
- [x] `compile_workflow_with_contracts` runs `validate_with_contracts` then idempotency gates
- [x] `CompileError::Validation` preserves exact variant
- [x] `CompileError::Workflow` preserves exact variant
- [x] Plain validation does NOT claim gate 12
- [x] `moon ci` passes
