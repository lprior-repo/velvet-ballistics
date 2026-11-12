# Theorem Kernel Projection: vb-qi37.10

## Boundary

- TLA+-owned temporal model: generated-vs-IR step/outcome/journal temporal parity over bounded finite workflows.
- Verus-owned Rust core: generated admission/support matrix, bounded generated stores, taint/error/outcome preservation for pure helper transitions, and no unchecked arithmetic/indexing in proofable kernels.
- Theorem-owned kernel: optional tiny finite lattice/refinement lemmas if Verus cannot express a compact proof for outcome/taint/error equivalence.
- Rust/runtime shell: source emission, subprocess compilation, rustfmt/clippy/trybuild, file I/O, action execution, timers, ask answers, Fjall persistence, and recovery hydration.
- External systems excluded from theorem proof: storage byte envelopes, OS process execution, compiler implementation, and wall-clock/external action completion.

## Theorem-Owned Clauses

No mandatory Lean/Aeneas/Hax theorem is required at State 3. The preferred path is Verus plus executable parity tests because the core claims bind directly to Rust APIs.

Optional theorem kernel if later proof planning finds Verus insufficient:

- THM-LAT-001: finite outcome/error refinement lattice preserves typed parity.
- THM-TAINT-001: finite taint join/refinement used by generated helpers is associative, monotone, and equal to runtime-observed lattice behavior.

## Non-Vacuity Requirements

Any Lean/Aeneas/Hax work must bind to production APIs and executable tests:

- Rust/spec targets must include `vb_codegen::validate_generated_subset`, `vb_codegen::emit_rust_workflow`, emitted generated runtime outcome API observations, and runtime oracle observations from `vb_runtime`.
- The theorem model must be generated from or checked against finite observations produced by executable generated-vs-IR parity tests.
- A standalone Lean enum and proof with no relation to `CompiledNodeKind`, `ExprOp`, `Taint`, generated outcomes, or runtime oracle signatures is rejected as a vacuum proof.

## Optional Theorem Obligations

### THM-LAT-001

- Contract clauses: POST-001, POST-002, INV-001, INV-004.
- Rust/spec target: generated and IR observable outcome records from parity harnesses.
- Lean module if created: `VbQi3710.OutcomeRefinement`.
- Theorem shape: `generated_refines_ir_outcome_or_typed_rejection`.
- Model: finite `Outcome`, `ErrState`, and support/rejection decision sets matching the TLA+ finite model.
- Refinement: each executable parity test emits an observation that validates into the Lean finite model; generated accepted outcomes must equal IR outcomes, rejected outcomes must carry typed unsupported errors.
- Shell exclusions: Rust source emission, compiler behavior, storage I/O, action/timer/ask external completion.
- Evidence command: blocked until State 4 creates a theorem artifact; then `lake build` or repository proof lane if configured.

### THM-TAINT-001

- Contract clauses: POST-003, INV-003.
- Rust/spec target: taint observations from generated helper/accessor parity tests and runtime oracle.
- Lean module if created: `VbQi3710.TaintJoin`.
- Theorem shape: `generated_taint_join_matches_runtime_lattice`.
- Model: finite taint set `{Clean, DerivedFromSecret, Secret}` and helper contribution sets.
- Refinement: generated and runtime helper observations map into the same finite taint result.
- Shell exclusions: concrete slot storage, object/list allocation, source emission, compile process.
- Evidence command: blocked until State 4 creates a theorem artifact; then `lake build` or repository proof lane if configured.

## Waivers

- Mandatory Lean proof is waived for State 3 because Verus/executable parity/TLA+ are better-fitting and no theorem target currently exists. This waiver expires if State 4 proof planning identifies a non-trivial lattice/refinement property that Verus cannot express without excessive trusted code.
