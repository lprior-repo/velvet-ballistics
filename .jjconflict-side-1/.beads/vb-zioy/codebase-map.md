# Codebase Map: vb-zioy

**Bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
**Date:** 2026-05-25

## Relevant Files

### Lowering Logic
- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:43-60` — Step primitive dispatch, routes `Collect` to `lower_canonical_collect`
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:167-224` — `lower_canonical_collect` implementation
- `crates/vb_compile/src/mod_compile_lowering/part_04.rs:211-248` — `emit_single_body_set` shared body dispatcher (enforces `body.len() == 1`)

### Error Types
- `crates/vb_compile/src/mod_compile_errors/collection.rs` — `CompileError::StepFieldShape` definition

### Tests
- `crates/vb_compile/tests/v1_primitive_lowering.rs` — Existing test `compile_workflow_rejects_multi_step_body_in_scoped_primitives` covers multi-step rejection for all scoped primitives

## Architecture

The compiler uses a shared `emit_single_body_set` dispatcher for all scoped primitives (repeat, for_each, collect, aggregate/reduce). This function already checks `body.len() != 1` and returns `StepFieldShape { field: "steps", expected: "exactly one set step" }`.

For collect specifically:
1. `lower_canonical_step` dispatches `StepPrimitive::Collect` → `lower_canonical_collect`
2. `lower_canonical_collect` computes synthetic step offsets (body, page, done)
3. It calls `emit_single_body_set(collect.body, body_step, ...)`
4. `emit_single_body_set` validates body length == 1 and that the single step is a `Set` primitive

## Identified Issue

The `body.len() != 1` check in `emit_single_body_set` reports the error at `id.as_usize()` where `id` is the *synthetic* `body_step` offset (original step + 1), not the original collect step index. This means error diagnostics reference the wrong step. The fix should report the error at the original step index (`index` parameter available in `lower_canonical_collect`).

Additionally, `emit_single_body_set` is shared across all scoped primitives, but the error field context should ideally reference the specific primitive ("collect.steps" vs generic "steps").

## Blast Radius

- Localized to `crates/vb_compile/src/mod_compile_lowering/`
- No API changes
- No downstream impact
- Test-only addition for collect-specific coverage
