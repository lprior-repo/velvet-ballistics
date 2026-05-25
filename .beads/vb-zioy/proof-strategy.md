# Proof Strategy: vb-zioy

## Bead Summary

**Bead:** vb-zioy — fix: enforce body.len() == 1 in collect body lowering (vb-xi2f.23)
**State:** 4 (Proof Planning)
**Scope:** Localized diagnostic fix in `vb_compile` lowering phase.

## Nature of the Change

This is a **diagnostic fidelity fix** with zero runtime behavior change on success paths. The shared `emit_single_body_set` dispatcher incorrectly reports synthetic compiled step indices (`id: StepIdx`) in user-facing `CompileError` variants. The fix adds a `diagnostic_step: usize` parameter and ensures all callers pass the original source AST step index.

## Proof Architecture

### Formal Verification Stance

**No formal verification (TLA+, Verus, Kani, Flux, Loom, Miri) is applicable** for this bead. Rationale:

1. **No temporal properties** — The fix does not involve state machines, protocols, queues, retries, leases, or distributed state.
2. **No arithmetic/index proofs** — The change is pure parameter plumbing: a `usize` value is threaded from callers to error constructors. No new arithmetic, indexing, or bounds-checking logic is introduced.
3. **No unsafe code** — The affected functions (`emit_single_body_set`, `lower_canonical_*`) contain no `unsafe` blocks, raw pointers, FFI, or layout concerns.
4. **No concurrency** — The lowering phase is single-threaded; no `Arc`, `Mutex`, channels, or async boundaries are involved.
5. **No refinement types** — The domain uses `usize` and `StepIdx` (both plain numeric types). No illegal state is representable through refinement types today.

### Primary Verification Layers

1. **Compile-Time Enforcement** (SEED-002): The Rust type system guarantees that all call sites of `emit_single_body_set` are updated to pass the new `diagnostic_step` argument. A `cargo check` gate proves this.
2. **Unit/Integration Tests** (SEED-004, SEED-005): Existing test `compile_workflow_rejects_multi_step_body_in_scoped_primitives` must be updated to assert the correct source step index (not synthetic) in `StepFieldShape` errors. New parameterized tests across all scoped primitives (collect, for_each, aggregate, repeat, parallel) verify caller correctness.
3. **Property Tests** (SEED-001, SEED-003): Existing proptest harnesses (`proptest_body_dispatcher.rs`, `proptest_error_parity.rs`) must be updated to pass `diagnostic_step` and assert the error's `step` field matches it, not the compiled `id`.

### Defense-in-Depth Summary

| Layer | Seed Coverage | Verifier | Status |
|-------|--------------|----------|--------|
| Compile-time | SEED-002 | Rust type system | Automatic |
| Unit test | SEED-001, 003 | `cargo test` | Planned |
| Integration test | SEED-004, 005 | `cargo test --test v1_primitive_lowering` | Planned |
| Property test | SEED-001, 003, 005 | proptest | Planned |
| Code review | SEED-002 | Human reviewer | Planned |

## Trusted Base

- **Caller obligation**: Each `lower_canonical_*` function must pass its `index: usize` as `diagnostic_step`. This is enforced by code review and grep verification, not by the type system (both `usize` and `StepIdx` are valid numeric types).
- **No model reduction**: The fix does not introduce model abstractions or stub boundaries.

## Waiver Stance

No waiver candidates. All behavior-affecting obligations must be proven through tests. No non-behavior exceptions exist.

## Risk Residuals

- **H4 (Index Namespace Confusion)**: Accepted as residual risk per contract. A future bead may introduce `SourceStepIdx` newtype.
- **H3 (Parallel Branch Ambiguity)**: Deferred to implementation; parallel branch diagnostic step choice is a caller-side decision.
