# Implementation Notes: vb-gvmt

## Source Changes

- `crates/vb_codegen/src/lib.rs`: generated runtime API additions for journaled execution, suspension/resume state, action/ask resume validation, typed errors, and Kani cfg module.
- `crates/vb_codegen/src/tests.rs`: POST-006 through POST-011 tests covering action/ask resume, journal events, invalid resume/no-mutation, overflow prechecks, and generated-vs-IR/runtime semantic comparison fixtures.
- `crates/vb_codegen/src/kani_generated_runtime.rs`: bounded Kani model harnesses for generated runtime invariants.
- `crates/vb_core/src/kani_capability_harnesses.rs` and `crates/vb_runtime/src/kani_capability_harnesses.rs`: capability Kani harness compilation fixes using bound lossy strings instead of borrowing from temporaries.
- `crates/vb_expr/src/eval.rs`: rejects mixed `F64`/`I64` arithmetic/comparison with typed `TypeMismatch`, removes mixed numeric `as f64` coercions, and removes production `unreachable!()` panic surface from numeric evaluation paths.

## Evidence Artifacts

- `.beads/vb-gvmt/tla-report.md`
- `.beads/vb-gvmt/verus-report.md`
- `.beads/vb-gvmt/kani-report.md`
- `.beads/vb-gvmt/parity-test-report.md`
- `.beads/vb-gvmt/mutation-report.md`
- `.beads/vb-gvmt/moon-ci-or-static-scan-report.md`
- `.beads/vb-gvmt/formal-verification-report.md`
- `.beads/vb-gvmt/verification-ledger.jsonl`
