# Verification Layers: vb-qi37.10

## Boundary

- Verus-owned kernel: support/rejection matrix, bounded generated stores, checked arithmetic/indexing, taint/error/outcome pure transitions, if a non-vacuous proof surface is added.
- TLA+ temporal model: bounded generated-vs-IR execution parity, typed Err states, journal-signature order, terminal/suspended/budget/error outcomes.
- Theorem projection: optional finite outcome/taint lattice only, bound to production observations.
- Runtime shell: generated source compilation, rustfmt, clippy, trybuild, generated executable parity harnesses, fuzz/proptest/Miri as applicable.
- External systems excluded from formal proof: actual Fjall persistence, OS process execution details, external action side effects, wall clock. They are covered by executable evidence or owning blocker beads.

## Layer Assignment

- PRE-001 -> executable tests + static scan: only validated `CompiledWorkflow` inputs enter codegen.
- PRE-002 -> code review/static scan: active implementation surface is `crates/vb_codegen/src/lib.rs`.
- PRE-003 / POST-005 / INV-002 -> Verus planned + Kani/proptest/executable capacity tests: bounded generated stores and checked counters.
- PRE-004 / POST-001 -> generated executable parity tests: same workflow/input/resource limits for IR and generated runs.
- PRE-005 / POST-002 / INV-001 -> unit tests + trybuild + TLA+: fail-closed generated admission.
- POST-003 / INV-003 -> generated executable parity tests + proptest: expression/accessor value and taint parity.
- POST-004 -> compile/runtime rejection tests + blocker if not implemented: text helpers.
- POST-006 / INV-006 -> rustfmt/clippy/static forbidden-construct scan over generated source.
- POST-007 / INV-007 -> trybuild compile-fail gate with non-empty fixtures.
- POST-008 / INV-005 -> TLA+ planned + generated executable journal-signature parity tests.
- Error taxonomy -> mutation/negative tests: each error variant or rejection path has a scenario.

## Verus Scope

- Rust targets: `vb_codegen::validate_generated_subset`, generated helper/store abstractions in `crates/vb_codegen/src/generated_storage_helpers.rs.txt`, and any extracted pure support-matrix/helper modules created by implementation.
- Spec/proof functions: blocked until State 4 creates non-vacuous proof targets. Required proof shapes: support matrix totality over final `CompiledNodeKind`, bounded store insert/lookup, checked counter increment, taint join preservation, typed unsupported-feature mapping.
- Invariants: fail-closed support, capacity bounds, no index overflow/wrap, taint preservation, typed error preservation.
- Trusted boundary: validated `CompiledWorkflow`; generated code source string emission; executable parity tests observe emitted behavior.
- Shell exclusions: file I/O, rustc/rustfmt/clippy subprocesses, action/timer/ask external systems, Fjall persistence.
- Evidence command: blocked until proof target exists; candidate command must be exact and artifact-bound in State 4 rather than invented here.

## TLA+ Scope

- Module/model path: planned `verification/tla/VbQi3710GeneratedParity.tla` / `.cfg`.
- Variables: mode, pc, step states, slots, taint, journal signature, budget, generated store state, accepted flag, typed Err state, outcome.
- Actions: validation, IR step, generated step, journal append, store insert/lookup, suspension, budget exhaustion, typed error, finish.
- Safety invariants: fail-closed acceptance, outcome parity, pc/slot/taint/state parity, journal signature parity, bounded stores, no counter wrap.
- Temporal properties: deterministic finite accepted workflows eventually finish or fail typed; external-blocking workflows reach matching suspension; no deadlock outside terminal/suspended/budget/error states.
- Fairness/deadlock stance: weak fairness on enabled step actions while budget remains; no fairness for external completion.
- Refinement boundary: TLA+ `IR` refines `vb_runtime`; TLA+ `GEN` refines emitted generated runtime after validation; `JournalSig` refines normalized observations.
- Evidence command: planned `tlc -config verification/tla/VbQi3710GeneratedParity.cfg verification/tla/VbQi3710GeneratedParity.tla` after State 4 creates files.

## Theorem Scope

- Theorem module: optional `VbQi3710.OutcomeRefinement` / `VbQi3710.TaintJoin`.
- Rust target: production outcome/taint observations from generated and IR parity tests.
- Abstraction relation: executable observations validate into finite theorem model.
- Shell exclusions: source emission, compiler, storage, external action/timer/ask completion.
- Non-goals: no standalone theorem model without production API/test binding.

## Required Executable Evidence Commands

- `cargo test -p vb_codegen --lib`
- `cargo test -p vb_codegen --test trybuild_tests`
- Focused generated-vs-runtime parity tests per added final IR family, using exact test names introduced by State 6 implementation.
- `cargo test -p vb_runtime collect reduce repeat together` only if runtime oracle behavior is touched or used in focused comparisons.
- `cargo test -p vb_core expr` and/or `cargo test -p vb_expr` only if expression oracle/helper semantics are touched.
- `cargo fuzz build generated_compare` if the generated compare fuzz target changes.
- `moon ci` as final repository gate per AGENTS.md after implementation states change code.

## Waivers / Deferred Owners

- `vb-qi37.11`: detailed suspension-error parity for wait/ask/do/retry beyond this bead's generated final-IR support matrix.
- `vb-gvmt`: broad generated semantic parity evidence after coverage exists.
- Phase 33/44: full recovery replay/hydration and deterministic journal evidence chain.
- Phase 37/45: aggregate whole-workflow budget and resource defaults.
- No performance/speed proof is required by this bead unless implementation makes a speed claim.
