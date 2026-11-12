# Contract Specification: vb-qi37.10

## Context

- Bead: `vb-qi37.10` - codegen: Complete remaining final IR coverage and parity.
- State: 3 contract/verification planning only.
- Scope boundary: generated final IR coverage/parity in `vb_codegen`, with `vb_runtime` and `vb_core` as semantic oracles. This contract does not implement production code, tests, proof code, or proof harnesses.
- Startup rule source: `/home/lewis/.agents/skills/rust-contract/SKILL.md` wins over `/home/lewis/.claude/skills/rust-contract/SKILL.md`; both state that contract-first artifacts are required, no production/proof/test code is written, TLA+ owns temporal behavior, and Verus owns Rust-local pure/core obligations.

## Authority and Scope Inputs

- Master final IR contract: `velvet-ballistics-MASTER.md` lines 613-660 require all `CompiledNodeKind` variants and generated Rust parity for step states, slot writes, taint, suspension, journal events, typed errors, and result values.
- Generated Rust mode: lines 1071-1107 require pinned-nightly compilation, rustfmt, clippy denies, no forbidden constructs, exact IR semantics, no hidden hot-step allocation except explicit resource contract allowance, equivalent journal/slot/taint/error/result behavior, equivalence tests, and compile-fail tests.
- Mandatory `vb_codegen` surface: lines 1304-1315 require `emit_rust_workflow`, step/expression/action/finish/resource-contract emission, `compare_generated_to_ir`, `validate_generated_subset`, compile-check generated Rust, and trybuild fixture emission.
- Implementation phase: line 1484 makes Phase 32 generated Rust mode responsible for codegen, compile checks, equivalence tests, and compile-fail tests.
- AI acceptance/final DoD: lines 1978-1988 require generated Rust semantic equivalence and actual evidence, not represented tasks.
- Normative parity oracle: line 1994 requires IR interpreter and generated Rust to match terminal result, typed error variant and fields, final pc, slot values, slot taints, step states, journal event sequence, action tickets, retry counts, wait/ask scheduling, and replay behavior.
- Recovery/journal context: lines 748-853 and 3386-3476 define journal sequence, envelope, typed storage/decode errors, and recovery gaps. This bead can require journal signature parity, but Phase 44 owns full recovery evidence chain/hydration.

## Narrow Bead Scope

In scope:

- `vb_codegen` final IR support/rejection matrix for all `CompiledNodeKind` variants in the final IR contract.
- Generated expression/accessor parity for active generated helpers and fail-closed rejection for unsupported final expression helpers unless implemented with runtime-oracle parity.
- Typed outcome/error parity between generated execution and the IR/runtime oracle for bead-local final IR families.
- Taint parity for slot writes, expression/accessor results, list/object construction, branch/join/reducer/repeat/collect materialization, and `Finish` result taint.
- Bounded generated stores for generated list/object/helper state and any added primitive side state.
- Compile-fail contract: trybuild compile-fail fixtures must be non-empty and must fail for real generated-code contract violations.
- Journal-signature parity: generated lightweight journal observations must match runtime/storage semantic event signatures for equivalent workflows.

Out of scope:

- `vb-qi37.11`: owns detailed suspension-error parity expansion for wait/ask/do/retry edge cases beyond the bead-local generated final-IR families.
- `vb-gvmt`: owns broader generated-mode semantic parity evidence after this coverage bead unblocks it.
- Phase 33 full recovery/replay and Phase 44 recovery evidence chain: own full live-frame hydration, deterministic step `SlotWritten` journal completion, and crash recovery proof.
- Phase 37/45 whole-workflow boundedness/resource defaults: own aggregate budget analysis and global arena caps. This bead still requires generated stores to be locally bounded and to return typed errors on local capacity exhaustion.
- Storage envelope implementation changes: storage remains an oracle; this bead should not change `vb_storage` unless a real oracle bug is found and separately scoped.

## Domain Terms

- Final IR: the `CompiledNodeKind` variants listed in the master final IR contract.
- Generated mode: Rust emitted by `vb_codegen::emit_rust_workflow` for maxperf execution.
- IR oracle: runtime execution semantics from `vb_runtime::engine::execute::execute_node_full` and primitive modules.
- Journal signature: normalized semantic sequence of event kinds and essential fields needed for parity: step lifecycle, slot write slot/value-kind/taint, action/wait/ask/retry scheduling fields, terminal event, sequence order. It is not byte-for-byte equality with storage envelopes.
- Typed outcome: success, suspension, budget exhaustion, or failure with exact public error variant and fields.
- Bounded generated store: generated in-memory helper state that has explicit capacity checks and typed capacity errors; no unbounded hot-step allocation.

## Preconditions

- PRE-001: The input workflow is a validated `CompiledWorkflow` whose final IR nodes pass `CompiledWorkflow::try_from_parts` and existing validation before entering `emit_rust_workflow`.
- PRE-002: `vb_codegen` must target the active source surface `crates/vb_codegen/src/lib.rs`, not the duplicate unreferenced `src/codegen/mod.rs`.
- PRE-003: Resource contract limits used by generated code are finite and explicit for lists, objects, primitive state, journal buffers, fanout/iteration/page/retry bounds, and step budget.
- PRE-004: Runtime oracle comparisons use the same compiled workflow digest, const pools, slot layout, action IDs, resource limits, inputs, and deterministic seeds as generated execution.
- PRE-005: Unsupported generated features must be rejected by `validate_generated_subset` before source emission or compile-check; late panics or generated code with unreachable placeholders are forbidden.

## Postconditions

- POST-001: For every final IR family implemented by generated mode, generated execution matches the IR/runtime oracle on terminal result, typed error variant and fields, final pc, slot values, slot taints, step states, and step budget consumption.
- POST-002: For `Together*`, `Reduce*`, `Repeat*`, and `Collect*`, generated mode either implements executable parity evidence or returns an exact typed unsupported-feature error from validation with a named blocker. `vb-qi37.10` must not be closed as complete while any required final IR family remains unsupported without an approved scope/blocker decision.
- POST-003: For generated expression/accessor helpers, `Append`, `AppendIf`, `Merge`, `Sum`, `Count`, `Unique`, `Has`, `Exists`, `Length`, `Empty`, and accessor traversal preserve value and taint parity against the expression/runtime oracle.
- POST-004: `Contains`, `StartsWith`, and `EndsWith` either gain symbol/text-store parity evidence or remain fail-closed with exact rejection tests and a recorded blocker; they must not silently compile to partial semantics.
- POST-005: Generated list/object/primitive side stores reject capacity exhaustion with typed generated/runtime-compatible errors before allocation or unchecked index arithmetic.
- POST-006: Generated source compiles under the pinned toolchain, formats with rustfmt, passes clippy deny settings for generated code, and contains no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, runtime YAML/JSON/HTTP, or runtime string action/reference lookup.
- POST-007: Trybuild compile-fail coverage is non-empty and proves generated-code contract violations fail compilation rather than becoming runtime partial behavior.
- POST-008: Generated journal signature equals the IR/runtime semantic journal signature for bead-local workflows, while acknowledging byte envelope persistence and full recovery hydration are owned by Phase 33/44.

## Invariants

- INV-001: Generated final-IR acceptance is fail-closed: every node/expression is either supported with parity evidence or rejected before source emission with a typed unsupported-feature error.
- INV-002: Generated stores are bounded by finite resource-contract limits and never rely on unbounded `Vec` growth or unchecked handle arithmetic after run admission.
- INV-003: Taint is never dropped, weakened, or invented. Generated joins must match runtime `Clean`, `DerivedFromSecret`, and `Secret` semantics, including `Finish` result taint.
- INV-004: Typed errors are semantically stable. Generated execution must preserve error category and fields observable by callers; lossy string-only errors are forbidden.
- INV-005: Journal sequence/signature is monotonic and parity-observable for generated versus IR execution over the same bounded workflow and inputs.
- INV-006: Generated source remains first-party-code compliant with all forbidden-construct rules.
- INV-007: Compile-fail fixtures are part of the contract gate; a trybuild run with zero compile-fail fixtures is a failed acceptance condition for this bead.

## Error Taxonomy

- `CodegenError::UnsupportedNodeFeature { node, feature }`: generated mode does not support a final IR node family; must identify exact family such as `Collect*`, `Together*`, `Reduce*`, or `Repeat*`.
- `CodegenError::UnsupportedExprFeature { op, feature }`: generated mode does not support an expression helper such as text/symbol helpers.
- `CodegenError::GeneratedStoreCapacityExceeded { store, len, max }`: generated bounded list/object/primitive store would exceed capacity.
- `CodegenError::GeneratedIndexOutOfBounds { index, len, context }`: generated helper/store access would be out of bounds.
- `CodegenError::GeneratedArithmeticOverflow { op, context }`: generated checked arithmetic failed.
- `CodegenError::GeneratedUnsupportedTaintShape { context }`: generated taint state cannot be represented without loss.
- `CodegenError::GeneratedJournalMismatch { expected, actual }`: parity harness observed journal-signature mismatch.
- `CodegenError::GeneratedSemanticMismatch { dimension, expected, actual }`: parity harness observed result/error/pc/slot/taint/state mismatch.
- `CodegenError::GeneratedCompileCheckFailed { tool, status }`: emitted source failed rustc/rustfmt/clippy/trybuild gate.
- Runtime/storage oracle errors remain typed by their owning crates; generated mode may map them only if variant and fields remain parity-observable.

## Contract Signatures

- `vb_codegen::emit_rust_workflow(workflow: &CompiledWorkflow) -> CodegenResult<String>`
- `vb_codegen::validate_generated_subset(workflow: &CompiledWorkflow) -> CodegenResult<()>`
- `vb_codegen::unsupported_node_feature(kind: &CompiledNodeKind) -> Option<&'static str>`
- `vb_codegen::unsupported_expr_feature(op: ExprOp) -> Option<&'static str>`
- `vb_codegen::compare_generated_to_ir(source: &str, workflow: &CompiledWorkflow) -> CodegenResult<()>`
- Generated runtime API emitted by `emit_generated_runtime_api` must expose fallible railway-oriented outcomes for drive/resume paths; no panic-only path may represent contract failure.

## Support/Rejection Rules

- All final IR families listed in master lines 619-653 are mandatory decision points for this bead.
- Existing supported families remain supported only if executable parity evidence covers success and representative error/edge cases.
- `Together*`, `Reduce*`, `Repeat*`, and `Collect*` are the highest-priority remaining node families. If any is deferred, `vb-qi37.10` stays non-closable unless a blocker bead explicitly owns the gap and acceptance is revised.
- `Contains`, `StartsWith`, and `EndsWith` are expression helper gaps. They may be rejected only with exact compile/runtime validation tests and a blocker because final generated expression coverage cannot be claimed otherwise.
- Rejection must occur in `validate_generated_subset` or an equivalent pre-emission gate. Emitted Rust must not contain stub branches that compile but return generic errors for final IR shapes accepted by validation.

## Verus-Owned Clauses

- PRE-003, POST-005, INV-001, INV-002, INV-003, INV-004, INV-006: Rust-local pure generated-admission and helper-store invariants should be specified/proven in Verus if/when a Verus proof surface exists for `vb_codegen`. Until such a surface exists, obligations are blocked on State 4 proof planning/writing to create a non-vacuous proof target bound to production APIs.

## TLA+-Owned Clauses

- POST-001, POST-002, POST-008, INV-001, INV-005: bounded temporal/state-machine parity for generated versus IR execution, including accept/reject, step transitions, typed Err states, journal-signature order, and terminal/suspended outcomes.

## Theorem-Owned Clauses

- Optional tiny theorem kernel only for finite outcome/taint/error lattice refinement if Verus cannot express it cleanly. The theorem must bind to production API observations and executable parity tests; standalone algebra with no Rust/refinement boundary is rejected.

## Non-Goals and Owning Blockers

- Full suspension-error parity for wait/ask/do/retry edge cases: blocker `vb-qi37.11`.
- Broad generated-mode semantic parity campaign after all coverage exists: blocker `vb-gvmt`.
- Full crash recovery/hydration and deterministic step evidence chain: Phase 33/44 recovery beads.
- Whole-workflow aggregate boundedness and global arena cap policy: Phase 37/45 beads.
- Performance superiority or maxperf benchmark claims: Phase 34/35 benchmark/maxperf beads; this bead may require generated code to compile but must not claim speed without benchmark evidence.
