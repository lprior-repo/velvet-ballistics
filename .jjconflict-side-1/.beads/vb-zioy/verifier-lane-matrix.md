# Verifier Lane Matrix: vb-zioy

## Legend
- **R** = Required (planned obligation exists)
- **NA** = Not Applicable (concrete evidence cited)
- **BT** = Blocked Tooling

## Matrix

| Proof Seed | Requirement | TLA+ | Verus | Kani | Flux | Loom | Miri | proptest | cargo-fuzz |
|-----------|-------------|------|-------|------|------|------|------|----------|------------|
| SEED-001 | REQ-001: StepFieldShape uses source step | NA | NA | NA | NA | NA | NA | **R** | NA |
| SEED-002 | REQ-003: Signature accepts diagnostic_step | NA | NA | NA | NA | NA | NA | NA | NA |
| SEED-003 | REQ-002: UnsupportedStepPrimitive uses source step | NA | NA | NA | NA | NA | NA | **R** | NA |
| SEED-004 | REQ-004: collect passes original index | NA | NA | NA | NA | NA | NA | **R** | NA |
| SEED-005 | REQ-005: for_each passes original index (all callers) | NA | NA | NA | NA | NA | NA | **R** | NA |

## Required Obligations Summary

| ID | Seed | Verifier | Obligation |
|----|------|----------|------------|
| PO-001 | SEED-001 | proptest | `proptest_body_dispatcher.rs` updated: empty/multi-step body returns `StepFieldShape` with `step == diagnostic_step` |
| PO-002 | SEED-003 | proptest | `proptest_error_parity.rs` updated: non-Set body returns `UnsupportedStepPrimitive` with `step == diagnostic_step` |
| PO-003 | SEED-004 | proptest + integration | Collect-specific integration test: multi-step collect body reports `step == 0` (source), not `1` (synthetic) |
| PO-004 | SEED-005 | proptest + integration | Parameterized test across all scoped primitives: body validation errors report source step index |

## Not-Applicable Rationale Summary

| Verifier | Count | Primary Rationale |
|----------|-------|-------------------|
| TLA+ | 5/5 | No temporal, protocol, or distributed state properties |
| Verus | 5/5 | No arithmetic, index, or typestate invariants to prove |
| Kani | 5/5 | No panic/overflow/index risk; function returns `Result` on all error paths |
| Flux | 5/5 | No illegal state representable as refinement type |
| Loom | 5/5 | No concurrency, atomics, or interleaving |
| Miri | 5/5 | No `unsafe`, FFI, raw pointers, or layout concerns |
| cargo-fuzz | 5/5 | No parsing or codec change; input grammar is unchanged |
