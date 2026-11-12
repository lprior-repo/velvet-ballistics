# vb-vt2f State 2 codebase map: Direct Rust API BDD acceptance scope

## Isolation and bead evidence

- Isolated workspace verified by `pwd -P`: `/home/lewis/src/femdation-vb-vt2f-bdd`.
- Forbidden source checkout for writes: `/home/lewis/src/velvet-ballistics`.
- `bd show vb-vt2f --json` in the isolated workspace currently fails because local `.beads` server tables are unavailable (`table not found: issues`); the controller-captured successful `bd show` payload is embedded in `.beads/vb-vt2f/STATE.md` lines 23-90.
- Bead title: `bdd: Direct Rust API acceptance scenarios`; parent epic `vb-hjvq`; dependency `vb-hxm0` closed; dependent `vb-oewy` open.

## Master-document clauses that bound this bead

- `velvet-ballistics-MASTER.md` lines 21-24: runtime is direct Rust API plus binary IPC; runtime uses numeric state machines and deterministic synchronous execution until suspension.
- Lines 41-58: Direct Rust API ingress is mandatory, typed failures are mandatory, and AI changes need executable evidence.
- Lines 344-346: `manual` trigger means direct Rust API submission via `Runtime::submit`; IPC is separate ingress.
- Lines 596-609: engine signals include `Finished`, budget exhaustion, and action/wait/ask suspension; `Finish` must preserve result taint.
- Lines 987-1012: shard-owned state, bounded queues, commands include `Submit`, `Resume`, `ActionCompleted`, `TimerFired`, `Cancel`, `Inspect`, `Shutdown`.
- Lines 3310-3345: accepted-artifact admission must gate production submit; legacy `submit_direct` remains for testing/internal use, and strict policy rejection with `AdmissionRequired` is release-relevant.

## Existing direct API/runtime surface map

- `crates/vb_runtime/src/runtime.rs`: public `Runtime` facade.
  - submit surfaces: `submit_direct`, `submit_direct_with_grants`, `submit_direct_with_grants_and_contracts`, `submit_direct_with_inputs_grants_and_contracts`, `submit_compiled`, `submit_compiled_with_grants`, `submit_compiled_with_inputs`, `submit_compiled_with_inputs_and_grants`.
  - control/inspection surfaces: `cancel_run`, `resume_run`, `inspect_run`, `snapshot_run`, `tick_all`, `tick_shard`, `take_inspect_response`, `list_active_runs`, `collect_metrics`, `counters_snapshot`.
  - completion/suspension surfaces: `complete_action`, `complete_action_with_output`, `fail_action`, `answer_ask`, `timer_fired`.
  - trace/shutdown surfaces: `list_events`, `drain_trace`, `shutdown_graceful`.
- `crates/vb_runtime/src/shard/types.rs`: public `ShardCommand`, `AskTicket`, `AskAnswer`, `InspectSnapshot`, `InspectResponse`, `ShardConfig`, `ShardStatus` model the behavior observable through the facade.
- `crates/vb_runtime/src/admission.rs`: public `RunAdmission`, `AdmissionError`, `ArtifactEnvelopeError`, `AcceptedArtifactStore`, `admit_artifact_run`, `admit_artifact_run_with_budget` define admission behavior for direct API acceptance.
- `crates/vb_runtime/src/trace.rs`: public `TraceEvent`/`TraceRing` support direct API trace assertions.
- `crates/vb_runtime/src/journal.rs`: `SharedRuntimeJournal`, `VolatileRuntimeJournal`, and journal events provide evidence for submit/run/finish/cancel/shutdown scenarios.
- `crates/vb_core/src/workflow.rs`, `frame.rs`, `engine.rs`, `nodes.rs`, `action.rs`, `policy.rs`, `capability.rs`, `ids.rs`, `value.rs`: core fixtures and typed values needed by direct API tests.

## Existing acceptance and workspace-test coverage

- `crates/workspace_tests/src/acceptance_catalog.rs` has `VB-BDD-CATALOG-004` for this bead: direct API must expose submit, inspect, cancel, trace, and shutdown; currently `executable_evidence_target: None`, `deferred_follow_up_bead: Some("vb-vt2f")`.
- `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` asserts `vb-vt2f` is one of five deferred release acceptance gaps.
- `crates/workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs` covers accepted-artifact storage/admission/recovery status, not direct runtime facade BDD.
- `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs` covers strict admission diagnostics with Given/When/Then comments, but does not drive the runtime facade through submit/tick/inspect/trace/shutdown as vb-vt2f requires.
- Existing runtime crate unit tests in `crates/vb_runtime/src/runtime.rs` and `crates/vb_runtime/src/shard/tests/**` cover many API pieces, but they are crate-local unit tests, not release-cataloged workspace acceptance scenarios for `VB-BDD-CATALOG-004`.

## Required BDD scenario scope for State 8+

Likely single workspace acceptance test file: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.

Minimum scenarios should use public `vb_runtime`/`vb_core` APIs only and explicit Given/When/Then comments:

1. Given a relaxed in-memory runtime and a deterministic finish workflow, when submitted via `Runtime::submit_direct` and driven with `tick_all`, then `snapshot_run`/trace/journal evidence shows expected terminal result and taint semantics.
2. Given an active or suspended run, when `inspect_run`/`snapshot_run` is used, then `InspectResponse` carries correlation id, pc, and executed count or typed not-found response.
3. Given a submitted run, when `cancel_run` is called before/at suspension, then trace/journal/counter evidence records cancellation and later snapshot is not active.
4. Given a `Do` suspension, when `complete_action_with_output` or `fail_action` is supplied with typed ticket/output/failure, then the run resumes or fails with exact typed state/trace evidence; legacy `complete_action` should be covered only if still public release surface.
5. Given an `Ask` suspension, when `answer_ask` is supplied with an `AskAnswer`, then the answer slot/taint resumes deterministically.
6. Given trace events for multiple runs, when `list_events` and `drain_trace` are called, then per-run filtering is non-destructive and drain is destructive/aggregated.
7. Given queued or active runs, when `shutdown_graceful` runs, then shards drain before journal shutdown and subsequent ticks report shutdown.
8. Given strict/admission-required policy, when legacy direct submit is attempted or accepted-artifact admission is absent, then exact `AdmissionRequired`/`AdmissionError` behavior is asserted or documented as current implementation gap.

## Gaps and risks

- Release gap: catalog row `VB-BDD-CATALOG-004` is deferred to this bead, so no workspace-level executable evidence currently counts for direct Rust API acceptance.
- API naming drift risk: master says `Runtime::submit`, but current facade exposes `submit_direct`/`submit_compiled`; acceptance tests must either bind to current public names or expose a compatibility API in a later implementation state.
- Admission drift risk: master Phase 39 text describes `RuntimePolicy { require_accepted_artifact, strict_admission }`, while code uses enum-like `RuntimePolicy::{Relaxed, Journaled, Strict}`; BDD must assert current code behavior and flag master/code mismatch if strict direct submit is not actually gated.
- Fixture risk: runtime unit-test helpers are private inside `runtime.rs`; workspace tests may need their own public-fixture builders using `vb_core::WorkflowParts` and `CompiledWorkflow::try_from_parts`.
- Determinism risk: BDD must drive ticks explicitly and avoid sleeping, async, external files, or global state.
- Durability/evidence risk: relaxed/noop runtime evidence is insufficient for scenarios claiming journal durability; use `VolatileRuntimeJournal` or storage-backed journal only where the direct API contract requires event evidence.
- Scope risk: do not test binary IPC, YAML CLI, generated Rust parity, or Fjall crash recovery here except where direct runtime admission depends on accepted artifacts.

## Likely touched files/globs

- Primary test artifact: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Catalog update: `crates/workspace_tests/src/acceptance_catalog.rs` set `VB-BDD-CATALOG-004.executable_evidence_target` to the new test file and clear its deferred follow-up.
- Catalog expectation update: `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` expected executable/deferred counts and lists.
- Possible public fixture/API adjustments only if tests cannot be written through public surfaces: `crates/vb_runtime/src/runtime.rs`, `crates/vb_runtime/src/lib.rs`, `crates/vb_runtime/src/admission.rs`, `crates/vb_runtime/src/journal.rs`, `crates/vb_runtime/src/shard/types.rs`.
- Core fixture dependencies: `crates/vb_core/src/{workflow.rs,ids.rs,value.rs,action.rs,policy.rs,capability.rs}`.

## Required verifier modes for later states

- State 8/11 scoped test lane: `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` or repository-equivalent Moon task once represented.
- Catalog regression lane: `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog`.
- Runtime focused lane if runtime code changes: `cargo nextest run -p vb_runtime` plus clippy for touched crates.
- Canonical gate at landing: `moon ci`, unless classified as pre-existing global debt with raw evidence.
- No Kani/TLA/Verus lane is required for BDD-only workspace tests unless runtime/admission semantics are changed.
