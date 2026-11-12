# Trusted Base Plan: vb-xi2f.4

## Trusted Assumptions

### TB-001: try_from_parts Validation Correctness
- **Scope**: `CompiledWorkflow::try_from_parts` in `vb_core`
- **Kind**: external_verified
- **Reason**: The validation logic (`validate_parts`, `validate_budget`) is owned by `vb_core` and has extensive existing test coverage (`section36_mandatory_coverage.rs` with 30+ validation cases) plus Kani/Verus artifacts from prior beads.
- **Impact**: If `try_from_parts` has a bug that fails to catch invalidity, the compiler could emit corrupt IR. Mitigated by existing `vb_core` verification artifacts.
- **Compensating Evidence**: `crates/vb_core/tests/section36_mandatory_coverage.rs`, `verification/kani/collect_try_from_parts.rs`, `verification/verus/try_from_parts.rs`
- **Behavior Affecting**: true

### TB-002: Kani Harness AST Construction
- **Scope**: Kani harnesses PO-002 and PO-006
- **Kind**: model_reduction
- **Reason**: Kani harnesses construct `WorkflowParts` from AST manually. We trust that the harness construction is representative of actual compiler behavior.
- **Impact**: Harness may not cover all compiler-emitted shapes. Mitigated by proptest using real YAML parser.
- **Compensating Evidence**: PO-003 proptest with real YAML input
- **Behavior Affecting**: true

### TB-003: YAML Parser Correctness
- **Scope**: `vb_yaml::parse_workflow_source`
- **Kind**: external_verified
- **Reason**: Proptest inputs rely on the YAML parser producing valid AST. Parser correctness is verified by separate parser tests.
- **Impact**: Parser bug could generate invalid AST that bypasses compile-time checks. Mitigated by parser test suite.
- **Compensating Evidence**: `crates/vb_yaml/tests/`
- **Behavior Affecting**: true

### TB-004: Flux Type System Soundness
- **Scope**: Flux refinement type checker
- **Kind": tool_soundness
- **Reason**: Flux proofs rely on the soundness of the refinement type system implementation.
- **Impact**: Unsoundness could allow invalid refinements to pass. Standard risk for any automated theorem prover.
- **Compensating Evidence**: Flux project test suite, peer review
- **Behavior Affecting**: false

### TB-005: From Impl Correctness
- **Scope**: `CompileError::Workflow(#[from] WorkflowError)`
- **Kind": type_system
- **Reason**: The `#[from]` derive for `thiserror` is trusted to produce a correct `From` implementation.
- **Impact**: Incorrect mapping would break error typing. Mitigated by Rust type system and `thiserror` crate maturity.
- **Compensating Evidence**: `thiserror` crate tests, Rust compiler type checking
- **Behavior Affecting**: false

### TB-006: proptest Arbitrary Implementations
- **Scope**: `WorkflowParts` and related types in proptest
- **Kind": model_reduction
- **Reason**: Proptest coverage depends on `Arbitrary` implementations generating sufficiently diverse inputs.
- **Impact**: Poor arbitrary impls may miss edge cases. Mitigated by explicit edge-case unit tests.
- **Compensating Evidence**: `crates/vb_core/tests/section36_mandatory_coverage.rs` explicit boundary cases
- **Behavior Affecting": true

## Static Analysis Compensating Evidence

### CI Lint for from_parts_unchecked
- **Script**: `grep -r "from_parts_unchecked" crates/vb_compile/src/ | grep -v "test" | grep -v "\.rs:.*//.*test"`
- **Expected Result**: Zero matches in production code after bead implementation.
- **Rationale**: Formal verifiers prove behavioral properties; static reachability of the unchecked function is best proven by absence in source.
- **Integration**: Add to `moon ci` lint pipeline or `xtask` lint task.

## Summary

| ID | Kind | Behavior Affecting | Compensating Evidence |
|----|------|-------------------|----------------------|
| TB-001 | external_verified | true | section36 tests, prior Kani/Verus |
| TB-002 | model_reduction | true | PO-003 proptest |
| TB-003 | external_verified | true | vb_yaml tests |
| TB-004 | tool_soundness | false | Flux tests |
| TB-005 | type_system | false | thiserror tests |
| TB-006 | model_reduction | true | explicit unit tests |
