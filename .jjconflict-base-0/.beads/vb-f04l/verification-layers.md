# Verification Layers: vb-f04l

## Boundary

- Verus-owned kernel: abstract lowering plan for dense node IDs, in-range targets, slot coverage, bounded conversions, deterministic output, and primitive-specific local shape preservation.
- TLA+ temporal model: bounded primitive lifecycle model for `ForEach`, `Together`, `Collect`, `Reduce`, `Repeat`, `Wait`, and `Ask` in `verification/tla/V1PrimitiveLowering.tla`.
- Theorem projection: optional dense recursive expansion theorem only if Verus cannot express recursive body allocation without excessive trusted assumptions.
- Runtime shell: YAML parser entry, mutable builder implementation, validation calls, workflow construction, runtime execution, wall-clock/event/human answer services.
- External systems excluded from formal proof: filesystem, storage, action services, event sources, human answer services, wall-clock scheduler.

## Layer Assignment

- PRE-001 -> focused test/property evidence for empty canonical source rejection.
- PRE-002 -> focused test evidence for v1 version and trigger admission.
- PRE-003 -> focused test evidence for unsupported top-level declarations/result.
- PRE-004 -> focused test plus property/Kani evidence for duplicate IDs across nested scopes.
- PRE-005 -> focused test evidence for unsupported control fields.
- PRE-006 -> focused test evidence for empty primitive source fields.
- PRE-007 -> Verus + Kani/proptest for checked bounds before narrowing.
- POST-001 -> integration tests and `moon ci` proving scoped primitives no longer return `UnsupportedStepPrimitive`.
- POST-002 -> focused tests and review proving `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts` remain on the success path.
- POST-003 -> deterministic digest/IR property tests plus Verus abstract deterministic-plan obligation.
- POST-004 -> Verus + Kani/proptest + validation evidence for dense/in-range targets.
- POST-005 -> Verus + Kani/proptest slot coverage checks.
- POST-006 -> TLA+ lifecycle + Verus primitive shape preservation + implementation tests for `ForEach`.
- POST-007 -> TLA+ lifecycle + Verus primitive shape preservation + implementation tests for `Together`.
- POST-008 -> TLA+ lifecycle + Verus primitive shape preservation + implementation tests for `Collect`.
- POST-009 -> TLA+ lifecycle + Verus primitive shape preservation + implementation tests for `Reduce`.
- POST-010 -> TLA+ lifecycle + Verus primitive shape preservation + implementation tests for `Repeat`.
- POST-011 -> TLA+ suspend/resume + Verus primitive shape preservation + implementation tests for `Wait` legal/illegal shapes.
- POST-012 -> TLA+ suspend/resume + Verus primitive shape preservation + implementation tests for `Ask`/`AskResume` pairing.
- POST-013 -> regression tests for existing `Set`/`Finish` behavior.
- POST-014 -> static scan/review inventory proving legacy compiler files/tests are not deleted to hide regressions.
- INV-001 -> Verus + Kani/proptest for dense node IDs.
- INV-002 -> TLA+ lifecycle target range + Verus/Kani/proptest concrete target range + `vb_validate` gate evidence.
- INV-003 -> Verus + Kani/proptest for slot coverage.
- INV-004 -> Verus + static scan for checked conversions and bounded allocation.
- INV-005 -> Verus abstract deterministic plan + proptest deterministic lowering.
- INV-006 -> acceptance tests assert no `UnsupportedStepPrimitive` for scoped primitives.
- INV-007 -> coverage matrix/test inventory for every newly accepted primitive.
- INV-008 -> static-scan/source lint through `moon ci`.
- INV-009 -> dependency-boundary scan through `moon ci`.
- INV-010 -> review + regression inventory for legacy compiler files/tests.
- ERR-001 through ERR-011 -> focused error scenarios + mutation/coverage evidence where applicable.

## Verus Scope

- Rust target: proof-facing abstract lowering plan for `crates/vb_compile/src/lib.rs::compile_source`, with a later bridge from emitted production nodes/targets/slots to the abstract plan.
- Current proof file: `verification/verus/v1_primitive_lowering.rs`.
- Evidence command: `verus verification/verus/v1_primitive_lowering.rs`.
- Required proof surfaces: `proof_lowering_plan_preserves_dense_node_ids`, `proof_lowering_plan_targets_in_range`, `proof_lowering_plan_slot_count_covers_references`, `proof_lowering_plan_checks_bounds_before_casts`, `proof_lowering_plan_deterministic_for_equal_source`, and `proof_lowering_plan_preserves_primitive_shapes`.
- Non-vacuity requirement: proof functions must derive claims from abstract constructors/transitions or bridge invariants; they must not require the exact property being ensured as an input predicate without construction evidence.
- Invariants: dense nodes, in-range targets, slot coverage, checked primitive bounds, deterministic plan, primitive-specific local shape preservation.
- Trusted boundary: validated `WorkflowSource` accessors; `StepIdx`, `SlotIdx`, and `ConstIdx` constructors; production bridge from `compile_source` emitted nodes to abstract plan; final runtime validators are compensating checks, not proof substitutes.
- Shell exclusions: YAML parsing, hashing internals, heap allocation, runtime event delivery, storage, generated Rust, wall-clock time.

## TLA+ Scope

- Module/model path: `verification/tla/V1PrimitiveLowering.tla`.
- Config path: `verification/tla/V1PrimitiveLowering.cfg`.
- Variables: primitive kind, lifecycle phase, target/done/join nodes, branch completion counters, attempts, pages, suspend/delivery/answer/timeout flags, finished flags.
- Actions: start, body completion, loop advance, branch completion, join, page/reduce/retry advance, suspend, delivery, answer, timeout, finish.
- Safety invariants: targets in range, no premature join, bounded counters, bounded branches, single completion.
- Temporal properties: eventual done/join/resume under finite bounds and weak fairness for all seven primitive families, including `Collect`, `Reduce`, `Wait`, and `Ask`.
- Fairness/deadlock stance: weak fairness as encoded by `WF_vars` in `Spec`; `CHECK_DEADLOCK TRUE` in config.
- Refinement boundary: emitted `CompiledNodeKind` family maps to the model primitive lifecycle only after Verus/Kani/proptest/test evidence establishes concrete emitted graph and slot shape.
- Evidence command: `tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`.

## Theorem Scope

- Theorem module: none required now.
- Rust target: optional abstract body expansion only.
- Abstraction relation: source primitive tree -> dense numeric graph.
- Shell exclusions: I/O, YAML parser, runtime, storage, wall-clock.
- Non-goals: no theorem proof of the async/runtime scheduler or actual event/human services.

## Second-Ring And CI Evidence

- `moon ci` is the canonical repository quality gate after implementation.
- Focused bead commands may use `cargo test -p vb_compile <filter>` where filters are exact obligations in `proof-obligations.jsonl`.
- Static source scan must cover `unsafe`, unwrap/expect/panic/todo/unimplemented/dbg, unchecked indexing/slicing, unchecked casts, and unchecked arithmetic in modified production paths.
- Performance claim is boundedness only: lowering must be linear or clearly bounded by source AST size and configured primitive limits; no speedup claim is made.

## Waivers

- LEAN-WAIVER-001: Lean/Aeneas/Hax waived unless Verus cannot express dense recursive expansion. Owner: State4 proof planner. Expiry: before implementation approval if Verus proof remains insufficient. Compensating evidence: strengthened Verus plus Kani/proptest.
- LOOM-WAIVER-001: Loom not required for compiler lowering because implementation is synchronous cold-path graph construction. Owner: State3 contract. Expiry: if implementation introduces concurrency. Compensating evidence: TLA+ lifecycle model and Rust graph-shape tests.
