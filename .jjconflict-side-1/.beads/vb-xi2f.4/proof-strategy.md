# Proof Strategy: vb-xi2f.4 — Route Compiler Emission through try_from_parts

## Goal

Eliminate the single unchecked `CompiledWorkflow` construction site in the compiler lowering pipeline and ensure the `test-util` feature is not leaked into production dependencies. All emitted `CompiledWorkflow` instances must pass `try_from_parts` validation.

## Proof Seeds

| Seed | Requirement | Domain Claim | Risk Tags |
|------|-------------|--------------|-----------|
| seed-001 | REQ-001 | All public compile APIs use `try_from_parts` | api-safety, validation-bypass |
| seed-002 | REQ-002 | `try_from_parts` rejects invalid IR with correct error variant | error-handling, validation |

## Strategy Overview

### REQ-001: No Unchecked Construction Reachable

1. **Kani (bounded model checking)**: Prove panic-freedom of `compile_source` after the `try_from_parts` integration. Verify that for bounded valid `WorkflowSource` AST inputs, `compile_source` returns `Ok(validated)` or `Err(typed)` and never panics.
2. **Verus (spec binding)**: Write a boundary spec proving the postcondition of `compile_source` — that any `Ok(CompiledWorkflow)` returned was constructed via `try_from_parts` (i.e., validated).
3. **proptest (property testing)**: Generate arbitrary valid YAML workflows, compile them, and assert the output is either `Ok` (validated workflow) or `Err` (typed compile errors).
4. **Flux (refinement types)**: Add refinement annotations ensuring `CompiledWorkflow` values in `vb_compile` originate only from `try_from_parts`.
5. **Static analysis (compensating)**: Grep/CI lint to confirm zero `from_parts_unchecked` occurrences in `crates/vb_compile/src/` outside test modules.

### REQ-002: Typed Validation Errors

1. **Kani**: Harnesses with `kani::any()` generated invalid `WorkflowParts` verify that each invalidity class produces the expected `WorkflowError` variant.
2. **Verus**: Spec proving the error mapping `WorkflowError → CompileError::Workflow` via `#[from]` preserves variant information.
3. **proptest**: Generate invalid `WorkflowParts` (out-of-bounds indices, backward edges, empty nodes, etc.) and assert `try_from_parts` returns the correct error variant.
4. **Flux**: Refinement annotations on `try_from_parts` return type encoding the error typing invariant.

## Non-Applicable Verifiers

| Verifier | Rationale |
|----------|-----------|
| TLA+ | No temporal protocol, distributed state, or lifecycle state machine involved. This is a static functional invariant. |
| Loom | Compile pipeline is single-threaded synchronous code. No concurrency, async, locks, or interleaving. |
| Miri | Both `vb_compile` and `vb_core` declare `#![forbid(unsafe_code)]`. No raw pointers, FFI, or layout concerns. |
| cargo-fuzz | Change is a call-site replacement in an internal lowering pipeline, not a parser/codec. Input is structured AST, not hostile bytes. `try_from_parts` fuzzing is owned by `vb_core`. |

## Trusted Base

- `try_from_parts` validation correctness is trusted from `vb_core` (extensively tested in `section36_mandatory_coverage.rs` and other beads).
- `#[from] WorkflowError → CompileError::Workflow` mapping is trusted from the type system.
- YAML parser (`vb_yaml`) correctness is trusted for proptest inputs.

## Blockers

None. All required tools (`cargo kani`, `cargo verus`, `cargo flux`, `proptest`) are present in the workspace.
