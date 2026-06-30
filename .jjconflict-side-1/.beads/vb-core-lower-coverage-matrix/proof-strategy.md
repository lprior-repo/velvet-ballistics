# Proof Strategy — vb-core-lower-coverage-matrix

## Overview
This bead proves that every v1 YAML construct is accepted/rejected consistently across vb_yaml, vb_validate, and vb_compile. The proof strategy leverages existing tests and Verus proofs rather than requiring new proof harnesses.

## Risk Classification

| Risk | Classification | Mitigation |
|------|---------------|------------|
| Parser/compiler grammar drift | high | Exhaustive unit tests + proptest |
| Primitive lowering bounds violation | high | Verus proofs + unit tests |
| Slot reference out-of-bounds | high | Verus proofs + unit tests |
| Node ID density violation | medium | Unit tests + proptest |
| Parity drift between crates | medium | API parity tests |
| Unknown coverage gaps | medium | Gap waivers + follow-up beads |

## Verifier Lane Selection

### Primary Lane: Unit Tests + Proptest
**Risk addressed**: Parser/compiler grammar drift, parity between APIs

**Evidence**: `crates/vb_compile/tests/v1_primitive_lowering.rs` (1350+ lines)
- Exhaustive test cases for all 7 scoped primitives (for_each, together, collect, reduce, repeat, wait, ask)
- API parity tests across CompileSource, CompileWorkflow, YamlCompilerCompile
- Error variant taxonomy tests
- Proptest with 64 cases for invariant exploration

**Command**: `cargo test -p vb_compile v1_primitive_lowering`

### Secondary Lane: Verus Proofs
**Risk addressed**: Bounded arithmetic, slot reference bounds, target range

**Evidence**: `verification/verus/v1_primitive_lowering.rs` (357 lines)
- `proof_construct_plan_valid` - constructor preconditions
- `proof_lowering_plan_preserves_dense_node_ids` - node count bounds
- `proof_lowering_plan_targets_in_range` - target bounds
- `proof_lowering_plan_slot_count_covers_references` - slot bounds
- `proof_lowering_plan_checks_bounds_before_casts` - primitive parameter bounds
- `proof_lowering_plan_deterministic_for_equal_source` - determinism

**Command**: `verus verification/verus/v1_primitive_lowering.rs`

### Not Applicable Lanes

| Lane | Reason |
|------|--------|
| TLA+ | No temporal behavior - static parity checking only |
| Kani | Existing unit tests + proptest provide sufficient bounded coverage |
| Miri | No unsafe code in vb_compile (`#![forbid(unsafe_code)]`) |
| Loom/Shuttle | No concurrent state transitions |
| Fuzz | Existing exhaustive unit tests cover the input space |
| Flux/Prustit | Verus suffices |

## Gap Analysis

### Known Verification Gaps (Waivers Required)

| Gap | Impact | Follow-up |
|-----|--------|-----------|
| `vars` validation coverage | medium | New bead needed |
| `secrets` validation coverage | medium | New bead needed |
| `examples` handling | low | New bead needed |
| `with` connector field | low | New bead needed |
| `then` next-step label | low | New bead needed |
| `condition` expression validation | low | New bead needed |

These gaps represent areas where coverage is unknown, not confirmed absent. Each requires a separate investigation bead.

## Obligations Summary

| ID | Clause | Verifier | Mode | Required |
|----|--------|----------|------|----------|
| PO-001 | INV-001 Node Density | unit-test | verify-standard | yes |
| PO-002 | INV-002 Slot Bounds | unit-test + Verus | verify-proof | yes |
| PO-003 | INV-003 Target Range | unit-test + Verus | verify-proof | yes |
| PO-004 | INV-004 Determinism | proptest | verify-standard | yes |
| PO-005 | POST-001 Primitives | unit-test | verify-standard | yes |
| PO-006 | POST-002 Unsupported | unit-test | verify-standard | yes |
| PO-007 | POST-003 Error Variants | unit-test | verify-standard | yes |

## Execution Plan

1. **State 5 (Proof Writing)**: Not required - existing artifacts are sufficient
2. **State 6 (Proof Review)**: Review existing test + Verus coverage
3. **State 8 (Test Writing)**: Not required - existing tests are comprehensive
4. **State 11 (Formal Verification)**: Execute `cargo test -p vb_compile v1_primitive_lowering` and `verus verification/verus/v1_primitive_lowering.rs`
