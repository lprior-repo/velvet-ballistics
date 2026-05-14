# Proof Evidence: vb-qi37.8

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 5 (Proof Writing)
- **dispatch_manifest**: delegate_agent=general (proof writing), isolated_workdir=/home/lewis/src/vb-qi37-ws

---

## Evidence Ledger

| Obligation | Gate | Verifier | Status | Evidence Path | Result |
|------------|------|----------|--------|---------------|--------|
| PO-001 | G7 | Kani | PENDING | evidence/kani-g7-stack.json | PENDING |
| PO-002 | G7 | Miri | PENDING | evidence/miri-g7-stack.log | PENDING |
| PO-003 | G8 | Kani | PENDING | evidence/kani-g8-accessor.json | PENDING |
| PO-004 | G8 | Miri | PENDING | evidence/miri-g8-accessor.log | PENDING |
| PO-005 | G9 | Kani | PENDING | evidence/kani-g9-slot-ref.json | PENDING |
| PO-006 | G9 | Kani | PENDING | evidence/kani-g9-error-slot.json | PENDING |
| PO-007 | G9 | Miri | PENDING | evidence/miri-g9-slot-ops.log | PENDING |
| PO-008 | G10 | Kani | PENDING | evidence/kani-g10-foreach.json | PENDING |
| PO-009 | G10 | Kani | PENDING | evidence/kani-g10-together.json | PENDING |
| PO-010 | G10 | Kani | PENDING | evidence/kani-g10-reduce.json | PENDING |
| PO-011 | G10 | Kani | PENDING | evidence/kani-g10-collect.json | PENDING |
| PO-012 | G10 | Miri | PENDING | evidence/miri-g10-kind.log | PENDING |
| PO-013 | G11 | Kani | PENDING | evidence/kani-g11-foreach-body.json | PENDING |
| PO-014 | G11 | Kani | PENDING | evidence/kani-g11-together-body.json | PENDING |
| PO-015 | G11 | Miri | PENDING | evidence/miri-g11-loop-graph.log | PENDING |
| PO-016 | G12 | Kani | PENDING | evidence/kani-g12-surjection.json | PENDING |
| PO-017 | G12 | Kani | PENDING | evidence/kani-g12-injection.json | PENDING |
| PO-018 | G12 | Proptest | PENDING | evidence/proptest-g12-bijection.log | PENDING |
| PO-019 | G13 | Kani | PENDING | evidence/kani-g13-acyclic.json | PENDING |
| PO-020 | G13 | TLA+ | DEFERRED_GLOBAL | evidence/tla-g13-nocycle.tlc.out | DEFERRED |
| PO-021 | G13 | Miri | PENDING | evidence/miri-g13-cycle-detect.log | PENDING |
| PO-022 | G14 | Kani | PENDING | evidence/kani-g14-type-compat.json | PENDING |
| PO-023 | G14 | Miri | PENDING | evidence/miri-g14-type-check.log | PENDING |
| PO-024 | G15 | Kani | PENDING | evidence/kani-g15-separated.json | PENDING |
| PO-025 | G15 | TLA+ | DEFERRED_GLOBAL | evidence/tla-g15-separated.tlc.out | DEFERRED |
| PO-026 | G15 | Lean | DEFERRED_GLOBAL | evidence/lean-g15-ndnodes.lean.out | DEFERRED |
| PO-027 | G15 | Miri | PENDING | evidence/miri-g15-det-graph.log | PENDING |
| PO-028 | Pipeline | Proptest | PENDING | evidence/proptest-pipeline-det.log | PENDING |
| PO-029 | Pipeline | Miri | PENDING | evidence/miri-pipeline-noeffect.log | PENDING |
| PO-030 | Pipeline | Kani | PENDING | evidence/kani-pipeline-compose.json | PENDING |
| PO-031 | Integration | Test | PENDING | evidence/test-r16-compile.log | PENDING |
| PO-032 | Integration | Test | PENDING | evidence/test-r17-api.log | PENDING |
| PO-033 | Integration | Test | PENDING | evidence/test-r18-schema.log | PENDING |
| PO-034 | Integration | Test | PENDING | evidence/test-r19-types.log | PENDING |
| PO-035 | Integration | Test | PENDING | evidence/test-r20-verify.log | PENDING |
| PO-036 | Integration | Fuzz | PENDING | evidence/fuzz-corpus/ | PENDING |

---

## Evidence Status Summary

| Status | Count | Obligations |
|--------|-------|-------------|
| PASS | 0 | - |
| FAIL_LOCAL | 0 | - |
| FAIL_REGRESSION | 0 | - |
| WAIVED | 0 | - |
| DEFERRED_GLOBAL | 3 | PO-020, PO-025, PO-026 |
| PENDING | 33 | All others |

**Global Debt**: 3 deferred obligations (TLA+/Lean temporal proofs)

---

## Source Code Evidence

### Gate Implementation Files

| Gate | File | Lines | Status |
|------|------|-------|--------|
| G7 | crates/vb_validate/src/gate_07_stack.rs | ~90 | IMPLEMENTED |
| G8 | crates/vb_validate/src/gate_08_accessor.rs | ~40 | IMPLEMENTED |
| G9 | crates/vb_validate/src/gate_09_slots.rs | ~160 | IMPLEMENTED |
| G10 | crates/vb_validate/src/gate_10_node.rs | ~250 | IMPLEMENTED |
| G11 | crates/vb_validate/src/gate_11_loop.rs | ~170 | IMPLEMENTED |
| G12-15 | crates/vb_validate/src/gate_12_14_15.rs | ~380 | IMPLEMENTED |
| G13 | crates/vb_validate/src/gate_13_cycles.rs | ~240 | IMPLEMENTED |

### Key Safety Evidence

1. **#![forbid(unsafe_code)]** - vb_validate/lib.rs:1
2. **No unwrap/expect** - All errors propagated via `?` operator
3. **Checked arithmetic** - gates.rs:72-84 uses checked_sub/checked_add
4. **Bounds validation** - All slot/step refs checked before use

### Test Coverage

| Gate | Unit Tests | Property Tests |
|------|------------|----------------|
| G7 | gate_07_* | - |
| G8 | gate_08_* | - |
| G9 | gate_09_* | - |
| G10 | gate_10_* | - |
| G11 | gate_11_* | - |
| G12 | gate_12_* | proptest |
| G13 | gate_13_* | - |
| G14 | gate_14_* | - |
| G15 | gate_15_* | - |
| Pipeline | gate_tests.rs | proptest |

---

## Acceptance Criteria Mapping

| AC | Criterion | Verification | Status |
|----|-----------|--------------|--------|
| AC1 | All 9 gates compile and pass unit tests | cargo test -p vb_validate | PENDING |
| AC2 | validate() rejects malformed WorkflowParts | Unit test coverage | PENDING |
| AC3 | validate_with_contracts() checks G12 bijection | Integration test | PENDING |
| AC4 | ValidationPipeline::all_gates() enables all gates | Unit test | PENDING |
| AC5 | ValidationPipeline::no_gates() disables all gates | Unit test | PENDING |
| AC6 | Validation is deterministic | Property test | PENDING |
| AC7 | No panic on any input | Miri + Kani | PENDING |
| AC8 | Compilation succeeds --all-features | cargo build --release | PENDING |
| AC9 | Integration with vb_compile succeeds | cargo test -p vb_compile | PENDING |
| AC10 | Fuzz harness exercises validate_with_contracts | cargo fuzz run | PENDING |

---

## Deferred Obligations

### PO-020: TLA+ G13_NoCycle Invariant
- **Deferral Condition**: Run after PO-019 (Kani slot acyclic) passes
- **Rationale**: TLA+ temporal proof requires Kani bounded proof as prerequisite
- **Specification**: tla-spec.md Section 5, G13_NoCycle invariant

### PO-025: TLA+ G15_Separated Temporal Property
- **Deferral Condition**: Run after PO-024 (Kani separation) passes
- **Rationale**: TLA+ temporal proof requires Kani bounded proof as prerequisite
- **Specification**: tla-spec.md Section 6, G15_Separated temporal formula

### PO-026: Lean NDNodesSeparated Theorem
- **Deferral Condition**: Run after PO-025 (TLA+ temporal) passes
- **Rationale**: Lean theorem requires TLA+ model as foundation
- **Specification**: lean-contract.md Section 4, NDNodesSeparated theorem

---

## Evidence Artifacts to be Created

### Miri UB Check Evidence (9 files)
```
evidence/miri-g7-stack.log
evidence/miri-g8-accessor.log
evidence/miri-g9-slot-ops.log
evidence/miri-g10-kind.log
evidence/miri-g11-loop-graph.log
evidence/miri-g13-cycle-detect.log
evidence/miri-g14-type-check.log
evidence/miri-g15-det-graph.log
evidence/miri-pipeline-noeffect.log
```

### Kani Bounded Proof Evidence (16 files)
```
evidence/kani-g7-stack.json
evidence/kani-g8-accessor.json
evidence/kani-g9-slot-ref.json
evidence/kani-g9-error-slot.json
evidence/kani-g10-foreach.json
evidence/kani-g10-together.json
evidence/kani-g10-reduce.json
evidence/kani-g10-collect.json
evidence/kani-g11-foreach-body.json
evidence/kani-g11-together-body.json
evidence/kani-g12-surjection.json
evidence/kani-g12-injection.json
evidence/kani-g13-acyclic.json
evidence/kani-g14-type-compat.json
evidence/kani-g15-separated.json
evidence/kani-pipeline-compose.json
```

### Proptest Property Evidence (2 files)
```
evidence/proptest-g12-bijection.log
evidence/proptest-pipeline-det.log
```

### TLA+ Model Checking Evidence (2 files - deferred)
```
evidence/tla-g13-nocycle.tlc.out
evidence/tla-g15-separated.tlc.out
```

### Lean Theorem Proving Evidence (1 file - deferred)
```
evidence/lean-g15-ndnodes.lean.out
```

### Integration Test Evidence (6 files)
```
evidence/test-r16-compile.log
evidence/test-r17-api.log
evidence/test-r18-schema.log
evidence/test-r19-types.log
evidence/test-r20-verify.log
```

### Fuzz Corpus
```
evidence/fuzz-corpus/
```

---

**Evidence Report Generated**: State 5 - Proof Writing
**Next State**: 6 (Proof Review)
