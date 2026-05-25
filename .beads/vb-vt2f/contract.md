# vb-vt2f Contract: Direct Rust API BDD Acceptance Scenarios

## Context

- Bead: `vb-vt2f` - `bdd: Direct Rust API acceptance scenarios`.
- Scope: executable Given/When/Then acceptance scenarios for direct Rust public API behavior only.
- Public surface: `vb_runtime::runtime::Runtime` facade plus public `vb_runtime::shard`, `vb_runtime::admission`, `vb_runtime::trace`, and `vb_core` types listed in `.beads/vb-vt2f/delivery-scope.jsonl`.
- Source-of-truth clauses read: master lines 21-24, 41-58, 344-346, 596-609, 987-1012, and 3310-3345.
- Skill rules applied: `/home/lewis/.claude/skills/rust-contract/SKILL.md` and `/home/lewis/.agents/skills/rust-contract/SKILL.md` lines 12-26 require contract-first, verification-first, executable Fowler/Given-When-Then scenarios, railway-oriented fallible surfaces, and no implementation/test/proof code in this state. The files match; no conflict observed.

## Assumptions

- `vb-hxm0` has established the release acceptance catalog and currently defers direct API evidence to `vb-vt2f`.
- This state writes contracts only. Test implementation belongs to later states.
- Workspace BDD scenarios should live in `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` unless later implementation discovery proves a different public acceptance target is required.
- Scenarios must use public APIs only; private runtime unit-test helpers are not allowed as the primary behavior surface.
- Direct API tests may use isolated in-memory/volatile fixtures where durability is not the asserted behavior. If a scenario asserts journal evidence, it must use public journal/trace surfaces that produce observable events.
- The current code map shows API-name drift: master says `Runtime::submit`, current facade exposes `submit_direct` and `submit_compiled` variants. Contract binds to current public names while requiring evidence of the drift in scenario metadata.

## Open Questions For Later States

- Which public constructor path is shortest for a deterministic finish workflow fixture without private helpers?
- Does current strict admission policy reject `submit_direct` with an exact `AdmissionRequired`-equivalent typed error, or is that still a master/code drift gap that the BDD must record?
- Which public event variants are stable enough for exact trace assertions versus snapshot/counter assertions?

## Contract Clauses

### Preconditions

- PRE-001 Public-surface-only: Each scenario must drive behavior through exported `vb_runtime`/`vb_core` APIs, not private modules or crate-local test helpers.
- PRE-002 Isolated fixture: Each scenario must construct a fresh runtime, workflow, run id, trace/journal state, and inputs so no state leaks across scenarios.
- PRE-003 Deterministic drive: Scenarios must advance runtime progress by explicit public calls such as `tick_all`, `tick_shard`, action completion, ask answer, timer firing, or shutdown; no sleeps, timing races, network, IPC, YAML, JSON, or HTTP.
- PRE-004 Exact assertion target: Every scenario must name the scenario id, Given/When/Then clauses, public API surface, expected result or exact typed error, and evidence path/runner output.
- PRE-005 Admission policy declaration: Every submit scenario must state whether it uses relaxed/direct submission or admission-aware compiled-artifact submission and why that policy is valid for the asserted behavior.

### Postconditions

- POST-001 Runnable BDD target: The direct API acceptance group is runnable by `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance`.
- POST-002 Catalog closure: `VB-BDD-CATALOG-004` points at executable evidence and is no longer deferred to `vb-vt2f`.
- POST-003 Failure locality: A failing scenario reports the exact scenario/test name and observable mismatch.
- POST-004 Submit-to-finish evidence: A finish workflow submitted through the direct API reaches a terminal result; snapshot/trace/journal evidence preserves result value and taint.
- POST-005 Inspect evidence: Inspect/snapshot behavior returns active-run state with correlation/program-counter/executed-count evidence or an exact typed not-found result for absent runs.
- POST-006 Cancel evidence: Canceling a known run records observable cancellation and the run is no longer listed as active after deterministic draining.
- POST-007 Action completion evidence: Completing an action through `complete_action_with_output` resumes the correct run and records exact output value/taint or terminal state.
- POST-008 Action failure evidence: Failing an action through `fail_action` records typed failure behavior and resumes retry/error/terminal failure semantics without completing the wrong run.
- POST-009 Ask answer evidence: Answering an ask through `answer_ask` resumes the suspended run with exact answer value/taint semantics.
- POST-010 Trace evidence: `list_events` is non-destructive and per-run filterable; `drain_trace` is destructive/aggregated as documented by public API behavior.
- POST-011 Health/shutdown equivalent evidence: Public metrics/counters/status plus `shutdown_graceful` demonstrate healthy runtime before shutdown and no further deterministic execution after shutdown.
- POST-012 Admission rejection evidence: Under strict/admission-required policy, direct submission without accepted artifact is rejected with the current exact typed error or recorded as an implementation gap linked to master lines 3330-3345.

### Invariants

- INV-001 Determinism: Re-running a scenario with the same fixture and explicit ticks yields the same terminal state, event sequence class, and typed errors.
- INV-002 Public API fidelity: Acceptance evidence must not pass by observing or mutating private shard internals.
- INV-003 Exact typed failures: Error scenarios assert specific error variants/typed responses, not string fragments unless the public type only exposes a stable diagnostic string.
- INV-004 No weak assertions: A scenario cannot count as release evidence if it only checks that a call returned `Ok(())`; it must assert state, trace, journal, counter, snapshot, or exact typed error.
- INV-005 Scenario independence: Scenario order does not affect pass/fail outcome.
- INV-006 Runtime-core exclusions: Direct API BDD must not introduce runtime YAML/JSON/HTTP behavior or private helper reliance.

## Error Taxonomy For Acceptance Scenarios

- ERR-001 UnknownRun: cancel/inspect/complete/fail/answer operations for an absent run return the current public typed not-found/unknown-run response.
- ERR-002 AdmissionRequired: strict/admission-required policy rejects raw direct submission without accepted artifact.
- ERR-003 InvalidActionTicket: action completion/failure for a mismatched run/ticket/step/action returns exact public typed error and does not mutate another run.
- ERR-004 InvalidAskTicket: ask answer for a mismatched or stale ask ticket returns exact public typed error and does not mutate another run.
- ERR-005 ShutdownRejectedOrNoop: post-shutdown tick/submit/control operations expose exact public behavior: typed rejection, no-op, or inactive status, never panic.
- ERR-006 WeakScenarioRejected: catalog/runner metadata rejects scenarios missing Given, When, Then, public surface, or exact assertion evidence.

## Contracted Public Behaviors And BDD Scenarios

### SCN-VT2F-001 Submit direct finish preserves result and taint

- Given: a fresh relaxed in-memory runtime, a public `CompiledWorkflow` fixture that deterministically finishes with a known `SlotValue` and `Taint`, and a unique `RunId`.
- When: the scenario submits via `Runtime::submit_direct` or current public equivalent and drives progress with explicit `tick_all` until completion.
- Then: public snapshot/trace/journal/counter evidence shows the run finished with exact result value and result taint; no private helper is required.
- Covers: POST-004, INV-001, INV-002, INV-004.

### SCN-VT2F-002 Inspect active and unknown runs

- Given: a fresh runtime with one submitted active/suspended run and one absent `RunId`.
- When: the scenario calls `inspect_run`, `snapshot_run`, and `take_inspect_response` through public APIs.
- Then: known run evidence includes correlation id, program counter or snapshot state, and executed count; absent run produces exact typed not-found behavior.
- Covers: POST-005, ERR-001.

### SCN-VT2F-003 Cancel known run

- Given: a submitted known run before terminal completion or at a documented suspension point.
- When: the scenario calls `cancel_run` and drains deterministic work with public tick/drain APIs.
- Then: cancellation is visible via trace/journal/counters/snapshot and `list_active_runs` no longer reports the run as active.
- Covers: POST-006.

### SCN-VT2F-004 Complete action resumes only the matching run

- Given: a workflow suspended on `Do` with a public action ticket/output slot and a second unrelated run.
- When: the scenario calls `complete_action_with_output` with the matching ticket and output.
- Then: only the matching run resumes; output value and taint are observable; unrelated run state is unchanged.
- Covers: POST-007, ERR-003.

### SCN-VT2F-005 Fail action records typed failure

- Given: a workflow suspended on `Do` with retry/error/terminal failure path and a public action ticket.
- When: the scenario calls `fail_action` with a typed `ActionFailure`.
- Then: public state/trace evidence records the failure and the runtime follows retry/error/terminal semantics exactly.
- Covers: POST-008, ERR-003.

### SCN-VT2F-006 Answer ask resumes suspended run

- Given: a workflow suspended on `Ask` with a public `AskTicket` and expected answer slot.
- When: the scenario calls `answer_ask` with an `AskAnswer`.
- Then: public state/trace evidence shows deterministic resumption and exact answer value/taint behavior.
- Covers: POST-009, ERR-004.

### SCN-VT2F-007 Trace list and drain semantics

- Given: two runs that have emitted trace events.
- When: the scenario calls `list_events` for each run and then `drain_trace`.
- Then: list is non-destructive and filterable; drain returns aggregated events and subsequent drain/list behavior matches documented public API semantics.
- Covers: POST-010.

### SCN-VT2F-008 Health and graceful shutdown equivalent

- Given: a runtime with queued or active work and public metrics/status/counter access.
- When: the scenario collects health-equivalent evidence, calls `shutdown_graceful`, and attempts deterministic ticks/control operations after shutdown.
- Then: shutdown drains or rejects according to public contract; subsequent operations expose exact typed behavior and never panic.
- Covers: POST-011, ERR-005.

### SCN-VT2F-009 Strict admission rejects raw direct submit

- Given: a runtime configured with current strict/admission-required policy and no accepted artifact admission record.
- When: the scenario attempts legacy/raw direct submission.
- Then: the API returns the exact current typed rejection (`AdmissionRequired` or implementation-equivalent) or the test documents a current master/code drift gap with failure evidence.
- Covers: POST-012, ERR-002.

### SCN-VT2F-010 Acceptance catalog closes direct API gap

- Given: the acceptance catalog row `VB-BDD-CATALOG-004`.
- When: the catalog tests run after the direct API acceptance target exists.
- Then: the row points at the executable target and no longer lists `vb-vt2f` as deferred.
- Covers: POST-002, ERR-006.

## Contract Signatures / Surfaces Under Test

- Submit: `Runtime::submit_direct`, `Runtime::submit_compiled`, and current accepted-artifact admission APIs where policy requires.
- Inspect: `Runtime::inspect_run`, `Runtime::snapshot_run`, `Runtime::take_inspect_response`, `Runtime::list_active_runs`.
- Cancel/resume: `Runtime::cancel_run`, `Runtime::resume_run`.
- Action completion/failure: `Runtime::complete_action_with_output`, `Runtime::complete_action`, `Runtime::fail_action`.
- Ask/wait: `Runtime::answer_ask`, `Runtime::timer_fired`.
- Trace: `Runtime::list_events`, `Runtime::drain_trace`.
- Health/shutdown equivalents: `Runtime::collect_metrics`, `Runtime::counters_snapshot`, `Runtime::shutdown_graceful`.

## Verification Ownership

- BDD acceptance: required for every PRE/POST/INV/ERR clause in this bead; owned by later test-writing/implementation/formal execution states and directly traced in `proof-obligations.jsonl` plus `traceability-matrix.jsonl`.
- Public-surface audit: review-artifact obligation limited to public API import/use violations and private runtime-core access; it must not reject test helper style or local test structure.
- TLA+ runtime lifecycle: `TLA-VT2F-LIFECYCLE-001` is required for `POST-004` through `POST-011` and `INV-001`; earlier BDD-only lifecycle waivers are superseded and not approval paths. State 5 evidence records TLC PASS for the lifecycle model.
- TLA+ strict admission: `TLA-VT2F-STRICT-ADMISSION-001` is required for `POST-012`/`ERR-002`/`PRE-005`/`INV-006`; earlier strict-admission TLA waivers are superseded and not approval paths. State 5 evidence records TLC PASS for the strict-admission model.
- Kani runtime facade/lower shard: `KANI-VT2F-RUNTIME-FACADE-001` and `KANI-VT2F-SHARD-LOWER-001` are required compensating executable Rust obligations over owner-authorized projection proof kernels, not over the full concrete `Runtime`/`Shard`/admission/store implementations. The proof targets are `crates/vb_runtime/src/kani_vt2f_runtime_facade.rs::vt2f_runtime_facade_semantics` and `crates/vb_runtime/src/kani_vt2f_shard_lower_semantics.rs::vt2f_shard_lower_semantics` under `#[cfg(kani)]`.
- Kani projection trusted boundary: `KernelRuntimeError`, `KernelInspectResponse`, `FacadeKernelState`, `ShardKernelState`, `StoreMode`, `TicketShape`, and `AskKernelFrame` are trusted manual projections of the concrete public runtime/shard/admission/ask behavior listed in `.beads/vb-vt2f/proof-architecture-report.md`. These Kani PASS results prove only the projected bead-local semantics and must not be cited as concrete-runtime Kani equivalence, full store/Fjall behavior, scheduler fairness, or public API execution proof.
- Projection equivalence: `PROJ-EQ-VT2F-001` is a required manual review/waiver obligation mapping each projection type/action to the concrete runtime/shard/admission/ask code. It records residual risk, owner authorization, expiry before any semantic edit to runtime/shard/admission/ask/action/journal/trace/store-selection behavior, and the non-reuse caveat for future beads.
- Verus/Lean: Lean remains waived for no theorem kernel. Verus is not blanket-approved; `WAIVER-VERUS-VT2F-002` is candidate-only until State 6 reviewer approval after accepted TLA PASS, owner-authorized Kani projection-kernel PASS, BDD/catalog/CI evidence, and explicit acceptance or rejection of the `PROJ-EQ-VT2F-001` trusted projection risk. If a pure runtime/core transition kernel is extracted or semantics change again, executable non-vacuum Verus obligations must replace the candidate waiver.

## Non-goals

- No binary IPC BDD scenarios; those belong to IPC acceptance beads.
- No YAML/CLI authoring scenarios.
- No generated-vs-IR parity scenarios.
- No Fjall crash recovery scenarios except direct admission/journal evidence needed by direct API behavior.
- No performance or speed claims.
