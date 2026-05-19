# Codebase Map — vb-0sps

Bead: `vb-0sps` — BDD generated-vs-IR parity acceptance scenarios.  
Workspace: `/home/lewis/src/bd-vb-0sps-bdd`; source checkout `/home/lewis/src/velvet-ballistics` was read-only/not edited.

## Normative / planning inputs read

- `/home/lewis/src/bd-vb-0sps-bdd/.beads/vb-0sps/STATE.md` — State 1 complete; baseline `moon ci` exited 0 but ran no affected task pipeline.
- `/home/lewis/src/bd-vb-0sps-bdd/.beads/vb-0sps/baseline-report.md` — baseline is not final quality evidence; State 11 must run scoped/canonical gates.
- `/home/lewis/src/bd-vb-0sps-bdd/velvet-ballistics-MASTER.md`
  - Lines 23-25, 57, 235-248, 660, 1077-1088, 1266: current active milestone is compiled IR interpreter; generated Rust/maxperf are deferred unless reactivated by dedicated contract.
  - Lines 660-664 and 1838-1871: future generated execution must prove exact parity for step states, slot writes, taint, suspension semantics, journal events, typed errors, and result values.
- `/home/lewis/src/bd-vb-0sps-bdd/docs/deferred-codegen-maxperf.md` — exact reactivation/equivalence list: terminal result, error variants/fields, final PC, slots, taints, step states, journal sequence, action tickets, retry counts, wait/ask scheduling, replay behavior.
- `/home/lewis/src/bd-vb-0sps-bdd/docs/generated-workflows.md` — historical generated-mode path and typed unsupported diagnostic contract.
- `/home/lewis/src/bd-vb-0sps-bdd/docs/final-ir-coverage-matrix.md` — existing generated-mode parity matrix from prior work; useful as fixture taxonomy, not fresh evidence for this bead.

## Existing BDD acceptance catalog seam

- `/home/lewis/src/bd-vb-0sps-bdd/crates/workspace_tests/src/acceptance_catalog.rs`
  - `Scenario` model fields include Given/When/Then, public surface, fixture, expected outcome/error, durability profile, related bead, executable target/follow-up.
  - Scenario `VB-BDD-CATALOG-007` already names this bead: generated Rust remains semantically equivalent to IR mode; currently `executable_evidence_target: None`, `deferred_follow_up_bead: Some("vb-0sps")`.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`
  - Verifies the catalog and expects `vb-0sps` as a deferred follow-up.
  - State 3/BDD contract should likely replace the deferred row with an executable test target owned by `vb-0sps`.

## Generated-mode implementation surfaces

- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_codegen/src/lib.rs`
  - Public APIs: `emit_rust_workflow`, `validate_generated_subset`, `compare_generated_to_ir`, `compile_check_generated_rust`, `format_generated_rust`, `emit_trybuild_fixture`.
  - Error contract: `CodegenError::{UnsupportedIr, RustfmtFailed, CompileCheckFailed, SemanticMismatch, TrybuildFixture, Io}`.
  - `emit_rust_workflow` validates first, emits header/IDs/resource/value-store/journal contract/drive API/steps/expressions/actions/finish.
  - Generated runtime API includes `GeneratedRunState`, `drive_with_journal`, `run_until_blocked`, `complete_action`, `answer_ask`, generated `JournalEvent`, `GeneratedRunStatus`, and `DriveOutput` surfaces in emitted source.
  - Unsupported/closed behavior: `validate_generated_subset` checks node, expression, and accessor surfaces before source emission; text helpers `contains`, `starts_with`, `ends_with` reject because they require runtime symbol store.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_codegen/src/tests.rs`
  - Existing unit tests cover emission, generated source lint forbiddance, suspension, journal-ish generated API behavior, UnsupportedIr, and many node/expression cases.
  - Helpers already build `CompiledWorkflow` fixtures using `WorkflowParts` and core engine APIs.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_codegen/src/proptests.rs`
  - Existing proptest compiles generated Rust via `rustc` and compares simple generated trace text to core IR interpreter trace for a fixed small workflow family.
  - This is close to parity acceptance, but not a workspace BDD scenario and only checks a reduced observable set.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_codegen/src/kani_generated_runtime.rs`
  - Kani harness area for generated runtime safety; useful for proof scope if contracts demand no panic/bounds.

## IR interpreter / ground-truth surfaces

- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_core/src/workflow/mod.rs`
  - `CompiledWorkflow::try_from_parts` is the accepted IR constructor.
  - `CompiledNodeKind` final IR variants include linear nodes, expressions, object/list construction, `Do`, `Choose`, `ChooseSlot`, loop/fanout/fanin families, waits, ask/resume, retry, error handlers, jump, finish.
  - Key getters: `node`, `expression`, `accessor`, `constant`, `slot_count`, `symbols_count`, `entry`, `resource_contract`, `to_parts`.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_core/src/engine.rs`
  - Public interpreter exports: `new_run_frame`, `run_until_blocked`, `step_once`, `resume_action_completion`, `resume_action_failure`, `EngineSignal`, `StepBudget`, `ValueStore`.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_core/src/engine/run_loop.rs`
  - `run_until_blocked` / `drive_deterministic` execute until finish, suspension, or budget exhaustion.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_core/src/engine/step.rs`
  - `step_once` updates step state, runs nodes, routes errors, preserves `Finish` result taint, returns suspension signals for `Do`/wait/ask.
  - `resume_action_completion` writes output value/taint, marks suspended step succeeded, advances PC, and returns action completion journal event.
- `/home/lewis/src/bd-vb-0sps-bdd/crates/vb_core/src/frame.rs`
  - `RunFrame` stores pc, executed count, step states, slots, and taints; provides observed state needed for parity assertions.

## Candidate BDD acceptance target

Recommended new executable target under the virtual workspace rule:

- `/home/lewis/src/bd-vb-0sps-bdd/crates/workspace_tests/tests/vb_0sps_generated_ir_parity_bdd.rs`

Use public APIs only from `vb_codegen`, `vb_core`, and possibly `vb_runtime` if action/wait/ask scheduling needs runtime-level evidence. Do not place tests at repo root.

## Scenario families to contract in State 3

1. **Successful deterministic parity**
   - Given a `CompiledWorkflow` accepted by generated subset validation.
   - When IR interpreter and generated runtime execute from identical slots/taints/value-store fixture.
   - Then terminal result, final PC, all slot values, all slot taints, step states, and journal/event sequence match.

2. **Suspension parity**
   - Given generated-supported `Do`, `WaitUntil`, `WaitEvent`, and `Ask` fixtures.
   - When each mode runs until blocked.
   - Then suspension kind, step, resume PC, action id/input/output slots or prompt/deadline/timeout data match; no mode runs past boundary.

3. **Resume parity**
   - Given a blocked action or ask fixture.
   - When both modes receive identical completion/answer value and taint.
   - Then output slot write, taint, journal completion/answer event, step state, PC, and later terminal result match.

4. **Typed failure parity**
   - Given malformed/missing slots, bad program counter, type mismatch, divide-by-zero, missing next step, or budget exhaustion fixtures.
   - When both modes execute.
   - Then typed error category and fields match, or any intentionally different generated `DriveError` is explicitly mapped by contract.

5. **Unsupported generated-mode fail-closed**
   - Given currently unsupported or unsafe-to-generate IR/expression/accessor fixtures.
   - When `validate_generated_subset` or `emit_rust_workflow` runs.
   - Then no source is emitted and `CodegenError::UnsupportedIr { feature }` is returned before execution.

## Main risks for rust-contract

- Generated code is officially deferred in master docs; contract must frame this bead as BDD acceptance for the deferred row, not as making generated/maxperf a release blocker unless explicitly reactivated.
- Existing `vb_codegen` docs and implementation appear ahead of `docs/generated-workflows.md`; State 3 must decide whether acceptance scenarios target current code reality or deferred master scope.
- Existing generated-vs-IR proptests compare text traces over a small workflow family; BDD acceptance must strengthen observable parity to exact structured assertions.
- Journal/event parity has two surfaces: `vb_core` action journal events and generated emitted `JournalEvent`. Contract needs an adapter/comparator instead of raw Debug-string comparison.
- Unsupported/fail-closed tests must assert no source emission/compile attempt, not fallback-to-IR.
- Release-critical status: this bead is a BDD catalog gap (`VB-BDD-CATALOG-007`) and a release-risk evidence gap; not evidence of maxperf acceptance.

## Verification/gate scope suggestion

- Primary: `cargo test -p velvet-ballastics-workspace-tests --test vb_0sps_generated_ir_parity_bdd`.
- Focused dependency tests: `cargo test -p vb_codegen --all-features`; `cargo test -p vb_core --all-features` if fixtures touch core engine behavior.
- BDD catalog update gate: `cargo test -p velvet-ballastics-workspace-tests --test vb_hxm0_acceptance_catalog` if `acceptance_catalog.rs` is updated.
- Optional proof/safety lanes if contract demands: Kani harnesses under `vb_codegen/src/kani_generated_runtime.rs` and `vb_core/src/kani_workflow_arbitrary.rs` scoped to generated runtime bounds; no whole-fleet verifier run.
