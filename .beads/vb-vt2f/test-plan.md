# Test Plan: vb-vt2f Direct Rust API BDD Acceptance Scenarios

## Startup and Scope Evidence

- Read `/home/lewis/.claude/skills/test-planner/SKILL.md`: lines 8-10 require planning only, not test or implementation code; lines 75-171 require Given/When/Then scenarios, exact assertions, proptest/fuzz/Kani consideration, mutation checkpoints, and no bare `is_ok()`/`is_err()` assertions.
- Read `/home/lewis/.agents/skills/test-planner/SKILL.md`: same content; no conflict observed, so the agents copy wins trivially.
- Read `/home/lewis/.claude/skills/test-planner/references/testing-philosophy.md`: lines 5-16 require behavior/public-API/state testing; lines 82-86 require automated hermetic evidence for cared-about behavior; lines 106-116 reject weak assertions, private-state coupling, sleeps, and ambiguous test names.
- Bead scope: `vb-vt2f` only, State 7 `test-planning` only.
- Oracle repair evidence for State 7 attempt 2: read current isolated checkout public APIs in `crates/vb_runtime/src/runtime.rs` and exact `RuntimeError` variants in `crates/vb_runtime/src/error/mod.rs`; read catalog validator variants in `crates/workspace_tests/src/acceptance_catalog.rs` and State 9 rejection artifacts.
- Required State 8 implementation paths:
  - Direct API suite: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`
  - Catalog data update: `crates/workspace_tests/src/acceptance_catalog.rs`
  - Catalog regression update: `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`
- Do not write production code in State 8 unless a red acceptance scenario proves a public API gap and the controller explicitly routes implementation work. If runtime/shard/admission semantics change, TLA/Verus waivers reopen.

## Resolved Public Oracles For State 8

These are no longer open questions; State 8 must bind tests to these public names and exact outcomes from the isolated checkout under review.

| Obligation | Exact public API under test | Exact oracle |
|---|---|---|
| Submit relaxed direct | `Runtime::submit_direct`; action fixtures use `Runtime::submit_direct_with_inputs_grants_and_contracts` | `Ok(())` at submit boundary, followed by exact trace/journal/counter/snapshot assertions after `tick_all` |
| Inspect known run | `Runtime::snapshot_run`, `Runtime::inspect_run`, `Runtime::take_inspect_response`, `Runtime::list_active_runs` | `Ok(InspectResponse::Found(InspectSnapshot { run, correlation, pc: StepIdx::ZERO, executed: 0 }))` for suspended action fixture; active list equals `vec![run]` |
| Inspect absent run | `Runtime::snapshot_run(absent, correlation)` | `Ok(InspectResponse::NotFound { run: absent, correlation })` |
| Action completion invalid/mismatched ticket | `Runtime::complete_action_with_output`, then `Runtime::tick_all` | enqueue returns `Ok(())`; deterministic tick returns `Err(RuntimeError::InvalidActionCompletion)`; unrelated run snapshot/events equal pre-image |
| Action failure invalid/wrong-run ticket | `Runtime::fail_action`, then `Runtime::tick_all` | enqueue returns `Ok(())`; deterministic tick returns `Err(RuntimeError::InvalidActionCompletion)`; unrelated run snapshot/events/counters equal pre-image |
| Ask stale ticket | `Runtime::answer_ask`, then `Runtime::tick_all` on a ticket for a run that is no longer active | enqueue returns `Ok(())`; deterministic tick returns `Err(RuntimeError::RunNotFound)`; active list, unrelated snapshot, unrelated trace, and counters equal pre-image |
| Ask mismatched/wrong-run ticket | `Runtime::answer_ask`, then `Runtime::tick_all` with `ticket.run` set to an absent/wrong run id while an unrelated run remains active | enqueue returns `Ok(())`, tick returns `Err(RuntimeError::RunNotFound)`; unrelated active run snapshot/events/counters equal pre-image. If implementation later adds a dedicated ask-ticket variant, tests may be updated only through contract repair. |
| Trace list/drain | `Runtime::list_events`, `Runtime::drain_trace` | repeated `list_events(run)` returns identical per-run sequence; `drain_trace()` returns aggregate events once; second `drain_trace()` returns `Vec::new()` |
| Shutdown post-operation behavior | `Runtime::shutdown_graceful`, `Runtime::tick_all`, `Runtime::submit_direct`, `Runtime::counters_snapshot` | `shutdown_graceful() == Ok(())`; queued work is drained exactly once; first and repeated post-shutdown `tick_all() == Ok(false)`; post-shutdown `submit_direct(...) == Ok(())` queues but does not progress; subsequent `tick_all() == Ok(false)`; `runs_submitted` remains the pre-shutdown completed count |
| Strict/admission-required raw direct submit | `Runtime::submit_direct` with `ShardConfig { policy: RuntimePolicy::Strict, .. }` | expected contract oracle is `Err(RuntimeError::AdmissionArtifactNotFound { digest })`; current observed permissive `Ok(())` is implementation drift and must stay red, with no active run fabricated as accepted evidence |
| Catalog validation variants | `validate_catalog` | exact `CatalogValidationError::{EmptyCatalog, MissingGivenWhenThen, MissingExactAssertion, MissingEvidenceDisposition, ConflictingEvidenceDisposition, InvalidExecutableEvidenceTarget, InvalidDeferredFollowUpBead, PrivateSurface, SharedFixture, DuplicateScenarioId}` as applicable |

## Summary

- Behaviors identified: 10 acceptance behaviors.
- Trophy allocation: 0 calc-unit / 10 integration-acceptance / 0 e2e / 2 static-review gates. The suite is deliberately integration-heavy because the contracted surface is the public `vb_runtime::runtime::Runtime` facade plus catalog closure, not isolated pure functions.
- Proptest invariants: 0 new required for this bead; no new pure multi-input calc function is introduced by the planned direct API acceptance tests/catalog edit.
- Fuzz targets: 0 new required; this bead forbids YAML/JSON/HTTP/IPC/parser boundaries.
- Kani harnesses: 0 new required; no production transition logic should change. Existing proof waivers remain valid only under that constraint.
- Mutation threshold: scoped `cargo-mutants` checkpoint target >=90% kill rate for changed test/catalog helper logic if mutants is run; critical listed mutants must be killed by named tests.

## 1. Behavior Inventory

1. `Runtime` preserves terminal result and taint when a deterministic workflow is submitted through the direct public API.
2. `Runtime` exposes exact active-run inspection and exact absent-run typed behavior when asked about known and unknown runs.
3. `Runtime` records cancellation and removes a run from active listings when a known run is canceled and deterministically drained.
4. `Runtime` resumes only the matching run and preserves action output value/taint when an action ticket is completed.
5. `Runtime` records typed action failure and follows retry/error/terminal semantics when an action ticket is failed.
6. `Runtime` resumes a suspended ask workflow with exact answer value/taint when an ask ticket is answered.
7. `Runtime` trace APIs preserve non-destructive filtered listing and destructive aggregate draining semantics when multiple runs emit events.
8. `Runtime` health/shutdown APIs expose pre-shutdown status and exact post-shutdown behavior without panics.
9. Strict/admission-required policy rejects raw direct submission without an accepted artifact, or the scenario fails red with explicit master/code drift evidence.
10. The acceptance catalog closes `VB-BDD-CATALOG-004` by pointing at executable direct API evidence and clearing `vb-vt2f` deferral.

## 2. Trophy Allocation

| Behavior | Primary layer | State 8 file path | Rationale |
|---|---:|---|---|
| 1 submit-to-finish | Integration acceptance | `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` | Exercises real public runtime facade and real value/taint/trace state. |
| 2 inspect known/unknown | Integration acceptance | same | Public API behavior crosses runtime state, inspect response, snapshot/listing surfaces. |
| 3 cancel known run | Integration acceptance | same | Requires state transition plus trace/journal/counter/list evidence. |
| 4 complete action | Integration acceptance | same | Validates ticket/run correlation and wrong-run non-mutation through real facade. |
| 5 fail action | Integration acceptance | same | Validates typed failure path and retry/error/terminal behavior through public API. |
| 6 answer ask | Integration acceptance | same | Validates ask suspension/resumption and taint propagation through public API. |
| 7 list/drain trace | Integration acceptance | same | Validates multi-run trace storage semantics with real event APIs. |
| 8 health/shutdown | Integration acceptance | same | Validates runtime status/counters and post-shutdown public behavior. |
| 9 strict admission rejection | Integration acceptance | same | Validates policy boundary; red failure is acceptable evidence only if it names drift. |
| 10 catalog closure | Integration/catalog regression | `crates/workspace_tests/src/acceptance_catalog.rs`, `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs` | Ensures release catalog no longer defers direct API evidence. |
| PRE-001 / INV-002 public-surface audit | Static review artifact | `.beads/vb-vt2f/test-review.md` in State 9, against the State 8 direct API test file | Review must audit private implementation imports only; helper style is out of scope. |
| INV-006 / GATE-VT2F-001 | Static/release gate | workspace via `moon ci` in State 11 | Only after State 8/9/10 evidence exists; classify unrelated global failures separately. |

## 3. BDD Scenarios and Exact Assertions

### SCN-VT2F-001 — submit direct finish preserves result and taint

- Test name: `test_direct_api_submit_to_finish_returns_result_and_taint`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh relaxed in-memory runtime, a public deterministic `CompiledWorkflow` fixture, a unique `RunId`, a known expected `SlotValue`, and a known expected `Taint`.
- When: submit via `Runtime::submit_direct`; drive only with explicit public `tick_all` until terminal.
- Then assert exact:
  - terminal snapshot/result equals expected `SlotValue`;
  - terminal result taint equals expected `Taint`;
  - trace/journal/counter evidence contains the submitted run id and terminal finished event/class;
  - active run list excludes the terminal run;
  - no private shard/internal modules are imported as primary evidence.
- Red/failing-first expectation before State 8: `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` fails with missing test target/file. After file creation, first red may be compile/API drift or assertion mismatch; output must name this test.
- Covers: PRE-001..PRE-005, POST-001, POST-003, POST-004, INV-001, INV-002, INV-004, INV-005, INV-006, WAIVER-TLA-VT2F-001, WAIVER-VERUS-VT2F-001, GATE-VT2F-001.

### SCN-VT2F-002 — inspect active and unknown runs

- Test name: `test_direct_api_inspect_known_and_unknown_run_returns_exact_state`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh runtime with one submitted active or suspended run and one absent `RunId`.
- When: call public `inspect_run`, `snapshot_run`, `take_inspect_response`, and `list_active_runs` for the known and absent ids.
- Then assert exact:
   - known run correlation id equals submitted id/correlation;
   - known suspended action fixture snapshot equals `Ok(InspectResponse::Found(InspectSnapshot { run, correlation, pc: StepIdx::ZERO, executed: 0 }))` after explicit ticks;
   - active list contains the known active/suspended id;
   - absent id returns exactly `Ok(InspectResponse::NotFound { run: absent, correlation })` from `Runtime::snapshot_run(absent, correlation)`, not a string-only or boolean error check.
- Red/failing-first expectation: absent-run handling fails if the assertion only checks `is_err()` or if the public API returns an undocumented error; output must name expected vs observed typed response.
- Covers: POST-005, ERR-001, INV-003, INV-004.

### SCN-VT2F-003 — cancel known run

- Test name: `test_direct_api_cancel_known_run_records_cancellation`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh runtime with a submitted known run before terminal completion or at a documented suspension point.
- When: call public `cancel_run` and drain deterministic work with public tick/drain APIs.
- Then assert exact:
  - cancellation event/state/counter/journal evidence references the run id;
  - `list_active_runs` no longer contains the canceled run;
   - follow-up `snapshot_run(run, 55)` returns exactly `Ok(InspectResponse::NotFound { run, correlation: 55 })`;
  - repeated deterministic execution yields the same event class and final active-list state.
- Red/failing-first expectation: if cancellation returns `Ok(())` but no observable cancellation state is asserted, the scenario is invalid; if runtime behavior lacks observable cancellation, the red failure must show missing evidence.
- Covers: POST-006, INV-001, INV-004, INV-005.

### SCN-VT2F-004 — complete action resumes only the matching run

- Test name: `test_direct_api_action_completion_resumes_correct_run`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: two fresh submitted runs; one is suspended on `Do` with a public action ticket/output slot, and the second is unrelated.
- When: call `complete_action_with_output` with the matching ticket and exact output value/taint; also exercise a mismatched/wrong-run ticket path.
- Then assert exact:
  - matching run resumes and records output value exactly;
  - matching run records output taint exactly;
  - unrelated run snapshot/trace/event count remains unchanged from before completion;
   - invalid/mismatched ticket enqueue returns `Ok(())`, the following `Runtime::tick_all()` returns exactly `Err(RuntimeError::InvalidActionCompletion)`, and no run other than the intended target mutates.
- Red/failing-first expectation: scenario fails if ticket correlation is not enforced, wrong run changes, taint is absent, or invalid ticket assertion is weaker than exact variant.
- Covers: POST-007, ERR-003, INV-003, INV-004.

### SCN-VT2F-005 — fail action records typed failure

- Test name: `test_direct_api_action_failure_records_typed_failure`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh workflow suspended on `Do` with public action ticket and documented retry/error/terminal failure path; also a second unrelated suspended run for non-mutation evidence.
- When: call `fail_action` with a typed `ActionFailure`; then call `fail_action` with an invalid/wrong-run action ticket and drive one public `tick_all`.
- Then assert exact:
   - trace/state/journal records the failure type/reason/code exposed by public API;
   - runtime follows the non-retryable fixture's exact terminal failure state: `counters_snapshot().runs_failed == 1` and `counters_snapshot().runs_completed == 0`;
   - run does not incorrectly complete with success value;
   - valid non-retryable failure uses `ActionFailure { code: ActionFailureCode::Timeout, retry_policy: RetryPolicy::NonRetryable, taint: Taint::Clean, detail: None, encoded_len: 0 }` and emits `TraceEvent::ActionFailed { run, step: StepIdx::ZERO, code: ActionFailureCode::Timeout }`;
   - invalid/wrong-run ticket enqueue returns `Ok(())`, the following `Runtime::tick_all()` returns `Err(RuntimeError::InvalidActionCompletion)`, and the unrelated run `snapshot_run`, `list_events`, and counters equal the pre-invalid-ticket snapshot.
- Red/failing-first expectation: failure is red if action failure is swallowed, converted to a success, lacks typed public evidence, or only asserts `is_err()`.
- Covers: POST-008, ERR-003, INV-003, INV-004.

### SCN-VT2F-006 — answer ask resumes suspended run

- Test name: `test_direct_api_answer_ask_resumes_suspended_run`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh workflow suspended on `Ask` with public `AskTicket { run, ask_step: StepIdx::new(2), resume_step: StepIdx::new(3) }`, expected answer slot `SlotIdx::new(2)`, expected answer `SlotValue::I64(6060)`, and expected `Taint::DerivedFromSecret`; also a second unrelated active/suspended run for non-mutation evidence.
- When: call `answer_ask` with the matching `AskAnswer`; then separately exercise stale and mismatched/wrong-run ask tickets and drive each queued answer with one public `tick_all`.
- Then assert exact:
   - resumed run observes the exact answer value and taint;
   - trace/snapshot state moves from ask-suspended to exact terminal not-found snapshot after completion: `snapshot_run(run, 66) == Ok(InspectResponse::NotFound { run, correlation: 66 })`;
   - matching answer emits `TraceEvent::AskAnswered { run, step: StepIdx::new(2), slot: SlotIdx::new(2) }` and journal `RuntimeJournalEvent::SlotWritten { run, slot: SlotIdx::new(2), value: postcard(SlotValue::I64(6060)), taint: Taint::DerivedFromSecret, extra: None }`;
   - after terminal completion, reusing the stale `AskTicket` enqueues as `Ok(())` and the next `Runtime::tick_all()` returns `Err(RuntimeError::RunNotFound)`;
   - a mismatched/wrong-run `AskTicket` whose `ticket.run` is an absent run id enqueues as `Ok(())` and the next `Runtime::tick_all()` returns `Err(RuntimeError::RunNotFound)`;
   - before/after `snapshot_run`, `list_events`, active-list membership, and counters for an unrelated run are exactly unchanged across stale/wrong-run ask attempts.
- Red/failing-first expectation: scenario fails if answer taint is not observable, stale tickets are accepted, or wrong-run mutation occurs.
- Covers: POST-009, ERR-004, INV-003, INV-004.

### SCN-VT2F-007 — trace list and drain semantics

- Test name: `test_direct_api_list_events_and_drain_trace_have_exact_semantics`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: two fresh runs that have each emitted distinguishable trace events.
- When: call `list_events` filtered per run before and after another `list_events`, then call `drain_trace`, then call post-drain list/drain again.
- Then assert exact:
  - first and second filtered `list_events(run_a)` return the same event sequence/class and include only run A;
  - `list_events(run_b)` includes only run B;
  - `drain_trace` returns aggregate events containing exactly `first_a.len()` events for run A and `first_b.len()` events for run B by run-id count;
  - immediate second `drain_trace()` returns exactly `Vec::new()`.
- Red/failing-first expectation: scenario fails if list consumes events, drain is non-destructive when documented destructive, filtering leaks events across runs, or assertions only count `> 0` without event identity/class.
- Covers: POST-010, INV-001, INV-004.

### SCN-VT2F-008 — health and graceful shutdown equivalent

- Test name: `test_direct_api_health_and_shutdown_equivalent_behavior`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a fresh runtime with queued or active work and public metrics/status/counter access.
- When: collect `collect_metrics` and `counters_snapshot`, call `shutdown_graceful`, then attempt post-shutdown `tick_all`, repeated `tick_all`, `submit_direct`, and another `tick_all`.
- Then assert exact:
   - pre-shutdown metrics expose `runs_active == 0` and first shard `command_queue_depth == 1` for the queued finish fixture;
   - `shutdown_graceful() == Ok(())`;
   - queued work is drained exactly once, so `counters_snapshot().runs_completed == 1` for the one queued finish fixture;
   - first and repeated post-shutdown `tick_all()` calls return exactly `Ok(false)`;
   - post-shutdown `submit_direct(RunId::new(9008), finished_workflow)` returns exactly `Ok(())`, but the following `tick_all()` returns `Ok(false)` and `counters_snapshot().runs_submitted` remains exactly `1`;
   - no operation panics.
- Red/failing-first expectation: scenario fails if shutdown permits further progress contrary to docs, post-shutdown behavior is not typed/documented, or only no-panic is asserted without exact state.
- Covers: POST-011, ERR-005, INV-001, INV-003, INV-004.

### SCN-VT2F-009 — strict admission rejects raw direct submit

- Test name: `test_direct_api_rejects_submission_when_accepted_artifact_required`.
- State 8 path: `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs`.
- Given: a runtime configured with current strict/admission-required policy and no accepted artifact admission record; scenario metadata declares strict policy.
- When: attempt raw/direct submission without accepted artifact.
- Then assert exact:
   - `Runtime::submit_direct(run, workflow)` under `RuntimePolicy::Strict` and no accepted artifact returns `Err(RuntimeError::AdmissionArtifactNotFound { digest })`, where `digest == workflow.digest()` captured before submission;
   - current observed `Ok(())` is acceptable only as red implementation-drift evidence, not as a passing oracle; failure message must name master/code drift for master lines 3310-3345 and expected `RuntimeError::AdmissionArtifactNotFound { digest }` behavior;
   - no run is active, no terminal success event is emitted, and no accepted-artifact state is fabricated.
- Red/failing-first expectation: this scenario is expected to expose drift if strict admission is not implemented; it must not be weakened to pass by accepting raw direct submission.
- Covers: POST-012, ERR-002, WAIVER-TLA-VT2F-002, PRE-005, INV-003, INV-004.

### SCN-VT2F-010 — acceptance catalog closes direct API gap

- Test names:
  - Update existing `test_catalog_maps_existing_tests_to_covered_scenarios`.
  - Add or update a catalog-specific assertion named `test_catalog_direct_runtime_api_row_points_to_executable_evidence_when_vt2f_is_done`.
- State 8 paths:
  - `crates/workspace_tests/src/acceptance_catalog.rs`
  - `crates/workspace_tests/tests/vb_hxm0_acceptance_catalog.rs`
- Given: catalog row `VB-BDD-CATALOG-004` for related bead `vb-vt2f`.
- When: the catalog regression inspects executable targets and deferred follow-up beads.
- Then assert exact:
  - `VB-BDD-CATALOG-004.executable_evidence_target == Some("crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs")`;
  - `VB-BDD-CATALOG-004.deferred_follow_up_bead == None`;
  - executable target count increases from 5 to 6;
  - deferred follow-up count decreases from 5 to 4;
  - follow-up bead list excludes `"vb-vt2f"` and remains exactly `["vb-te1i", "vb-rpch", "vb-0sps", "vb-ssei"]` unless unrelated catalog work lands first;
   - replace weak boolean behavior checks with localized collection equality: collect `(master_behavior, id)` pairs from `catalog()` and assert they equal the exact ten required behavior/id pairs; failure output must print the full actual vector;
   - replace weak related-bead boolean checks with exact sorted/vector equality over actual related beads, or with an exact offender vector equal to `Vec::<&str>::new()` for empty related beads;
   - catalog validation rejects each negative fixture with exact variants:
     - empty catalog -> `Err(CatalogValidationError::EmptyCatalog)`;
     - missing `given`/`when`/`then` -> `Err(CatalogValidationError::MissingGivenWhenThen { scenario_id })`;
     - both `expected_outcome == None` and `expected_error == None` -> `Err(CatalogValidationError::MissingExactAssertion { scenario_id })`;
     - no executable target and no deferred bead -> `Err(CatalogValidationError::MissingEvidenceDisposition { scenario_id })`;
     - both executable target and deferred bead set -> `Err(CatalogValidationError::ConflictingEvidenceDisposition { scenario_id })`;
     - executable target not under `crates/workspace_tests/tests/*.rs` -> `Err(CatalogValidationError::InvalidExecutableEvidenceTarget { scenario_id })`;
     - deferred follow-up bead not equal to `related_bead` or not prefixed `vb-` -> `Err(CatalogValidationError::InvalidDeferredFollowUpBead { scenario_id })`;
     - `public_surface` containing `private` or `helper` -> `Err(CatalogValidationError::PrivateSurface { scenario_id })`;
     - `fixture` lacking `isolated` -> `Err(CatalogValidationError::SharedFixture { scenario_id })`;
     - duplicate `id` -> `Err(CatalogValidationError::DuplicateScenarioId { scenario_id })`.
- Red/failing-first expectation before update: existing catalog has `executable_evidence_target: None` and `deferred_follow_up_bead: Some("vb-vt2f")`; catalog regression should fail after expectations are changed until catalog row is updated.
- Covers: POST-002, ERR-006.

## 4. Proptest Invariants

No new proptest target is required for State 8 if it only adds public API BDD tests and catalog row/test updates. State 8 must not add pure multi-input production functions. If State 8 introduces any pure fixture builder/catalog helper with branching over multiple inputs, add a proptest in `crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs` or an appropriate workspace test module with these invariants:

Concrete review checkpoint: State 9 must inspect State 8 diff for new pure helper branching. If no new pure helper branching exists, record `HELPER_BRANCHING_PROPTEST: NOT_REQUIRED — no new pure multi-input helper branch introduced`. If any new branching helper exists, State 9 must reject unless the corresponding proptest or explicit bounded table covers all branch outcomes and exact error variants.

| Candidate | Invariant | Strategy | Anti-invariant |
|---|---|---|---|
| catalog row validation, if new helper added | Exactly one evidence disposition is required: executable target xor deferred bead | generated `Scenario` fields with target/deferred combinations | both missing and both present return exact `CatalogValidationError` variants |
| run id/ticket fixture correlation, if pure public helper added | ticket accepted only for originating run id | generated distinct valid run ids/tickets within bounded fixture model | mismatched ticket returns exact invalid-ticket variant and preserves state |

## 5. Fuzz Targets

No fuzz target is required or allowed by current bead scope. The contract excludes runtime YAML/JSON/HTTP/IPC behavior, and the direct Rust API suite should build typed fixtures, not parse untrusted bytes or strings. If State 8 adds any parsing/deserialization boundary, stop and route scope change back to the controller.

## 6. Kani Harnesses

No new Kani harness is required while State 8 only writes tests/catalog updates. The proof obligations deliberately route runtime semantics through BDD evidence and waivers. If State 8 changes runtime/core transition logic, add proof planning before implementation; do not hardcode Kani shapes. Candidate reopened properties would be:

| Trigger | Required Kani property | Bound | Rationale |
|---|---|---|---|
| ticket/run correlation production change | mismatched action/ask tickets cannot mutate another run | bounded 2-run fixture with arbitrary ids/tickets | ERR-003/ERR-004 are state isolation invariants |
| shutdown semantics production change | post-shutdown transitions cannot advance execution or panic | bounded lifecycle states and operations | ERR-005/POST-011 are transition safety obligations |
| admission production change | strict policy rejects raw direct submit without accepted artifact | bounded policy/artifact states | POST-012/ERR-002 strict-admission obligation |

## 7. Mutation Checkpoints

Minimum threshold: >=90% scoped mutation kill rate for changed workspace test/catalog helper code if `cargo-mutants` is run. Critical mutants that must be killed by named tests:

- Flip `VB-BDD-CATALOG-004.executable_evidence_target` back to `None` -> killed by `test_catalog_direct_runtime_api_row_points_to_executable_evidence_when_vt2f_is_done`.
- Restore `VB-BDD-CATALOG-004.deferred_follow_up_bead` to `Some("vb-vt2f")` -> killed by catalog direct-row test and updated `test_catalog_maps_existing_tests_to_covered_scenarios`.
- Change expected executable target string to another path -> killed by exact equality assertion.
- Remove exact result value assertion in SCN-VT2F-001 -> killed by mutation review/weak assertion policy; scenario must fail if implementation returns wrong value.
- Remove exact taint assertion in SCN-VT2F-001/004/006 -> killed by those scenario assertions.
- Change absent-run expected error in SCN-VT2F-002 -> killed by exact typed error equality.
- Allow wrong-run action ticket mutation in SCN-VT2F-004/005 -> killed by before/after unrelated-run snapshot equality.
- Allow stale/mismatched ask ticket mutation in SCN-VT2F-006 -> killed by `RuntimeError::RunNotFound` stale/wrong-run tick assertion plus before/after unrelated-run snapshot/event/counter equality.
- Accept invalid/wrong-run `fail_action` or mutate another run -> killed by SCN-VT2F-005 `RuntimeError::InvalidActionCompletion` assertion plus unrelated-run non-mutation equality.
- Remove catalog negative branch for `MissingGivenWhenThen`, `MissingExactAssertion`, `ConflictingEvidenceDisposition`, `PrivateSurface`, `SharedFixture`, or `DuplicateScenarioId` -> killed by exact `CatalogValidationError` variant tests in SCN-VT2F-010.
- Weaken catalog behavior/related-bead checks back to boolean `any`/`all` -> killed by required exact vector/set equality review in SCN-VT2F-010.
- Make `list_events` destructive or `drain_trace` non-destructive -> killed by SCN-VT2F-007 repeated-list and post-drain assertions.
- Permit post-shutdown progress or submit success -> killed by SCN-VT2F-008 exact post-shutdown state/error assertions.
- Permit raw strict submit without accepted artifact -> killed or exposed as required red drift by SCN-VT2F-009.

## 8. Combinatorial Coverage Matrix

| Clause/obligation | Scenario/test | Input class | Exact expected output/evidence | Layer | State 8 path |
|---|---|---|---|---|---|
| PRE-001 | all SCN-VT2F-001..009 | public runtime/core imports | no private shard/internal primary evidence; State 9 `PUBLIC_SURFACE_AUDIT: PASS` | static review | `.beads/vb-vt2f/test-review.md` after direct test exists |
| PRE-002 | all SCN-VT2F-001..009 | fresh runtime/workflow/run ids | nextest passes under isolated fixtures; no order dependence | integration | direct API test file |
| PRE-003 | SCN-VT2F-001/003/004/005/006/008 | explicit ticks/action/ask/timer/shutdown | deterministic progress; no sleeps/network/IPC/YAML/JSON/HTTP | integration | direct API test file |
| PRE-004 | all SCN-VT2F-001..010 | scenario metadata | test names/GWT/evidence target visible; failure output names mismatch | integration/catalog | direct API + catalog tests |
| PRE-005 | SCN-VT2F-001/009 | relaxed vs strict submit policy | policy declaration matches asserted behavior | integration | direct API test file |
| POST-001 | direct API suite | test target discovery | `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` exits 0 after implementation | integration | direct API test file |
| POST-002 | SCN-VT2F-010 | catalog row | target set to direct test file and deferral cleared | catalog integration | catalog src + catalog test |
| POST-003 | all failure paths | intentionally failing mismatch or compile/API drift | nextest output names exact scenario/test and expected-vs-observed | integration | direct API test file |
| POST-004 | SCN-VT2F-001 | deterministic finish workflow | exact terminal value, taint, terminal event/counter/snapshot | integration | direct API test file |
| POST-005 / ERR-001 | SCN-VT2F-002 | known active run + absent run | known correlation/pc/executed count; absent exact typed not-found | integration | direct API test file |
| POST-006 | SCN-VT2F-003 | known cancelable run | cancellation evidence; run absent from active list | integration | direct API test file |
| POST-007 / ERR-003 | SCN-VT2F-004 | matching and mismatched action tickets | exact output value/taint; invalid/mismatched completion tick returns `Err(RuntimeError::InvalidActionCompletion)`; wrong-run snapshot/events unchanged | integration | direct API test file |
| POST-008 / ERR-003 | SCN-VT2F-005 | action failure and invalid/wrong-run ticket | valid failure emits `TraceEvent::ActionFailed` with `ActionFailureCode::Timeout`; invalid/wrong-run `fail_action` tick returns `Err(RuntimeError::InvalidActionCompletion)`; unrelated snapshot/events/counters unchanged | integration | direct API test file |
| POST-009 / ERR-004 | SCN-VT2F-006 | valid, stale, mismatched/wrong-run ask tickets | exact answer value/taint and `TraceEvent::AskAnswered`; stale/wrong-run ask tick returns `Err(RuntimeError::RunNotFound)`; unrelated snapshot/events/counters unchanged | integration | direct API test file |
| POST-010 | SCN-VT2F-007 | two runs with events | filtered non-destructive list; destructive aggregate drain semantics | integration | direct API test file |
| POST-011 / ERR-005 | SCN-VT2F-008 | active runtime then shutdown | `shutdown_graceful()==Ok(())`; queued work drains once; repeated `tick_all()==Ok(false)`; post-shutdown `submit_direct()==Ok(())` but no progress and submitted counter remains `1`; no panic | integration | direct API test file |
| POST-012 / ERR-002 | SCN-VT2F-009 | strict policy without accepted artifact | expected `Err(RuntimeError::AdmissionArtifactNotFound { digest })`; current `Ok(())` remains red drift evidence, never a passing oracle | integration | direct API test file |
| INV-001 | SCN-VT2F-001/003/007/008 | repeated deterministic fixtures | same terminal state/event class/typed errors on rerun | integration | direct API test file |
| INV-002 | all SCN-VT2F-001..009 | public API fidelity | no private internals observed/mutated as primary evidence | static review | State 9 review artifact |
| INV-003 | SCN-VT2F-002/004/005/006/008/009 | error scenarios | exact typed variants: `InspectResponse::NotFound`, `RuntimeError::InvalidActionCompletion`, `RuntimeError::RunNotFound`, `Ok(false)` post-shutdown inactive ticks, and expected `RuntimeError::AdmissionArtifactNotFound { digest }` strict rejection | integration | direct API test file |
| INV-004 | all SCN-VT2F-001..009 | weak assertion risk | every scenario asserts state/trace/journal/counter/snapshot/error beyond `Ok(())` | integration/review | direct API test file |
| INV-005 | all SCN-VT2F-001..009 | nextest scheduling | tests pass in any order/parallel scheduling | integration | direct API test file |
| INV-006 | source exclusion | no runtime-core YAML/JSON/HTTP/private-helper reliance | `moon ci` in State 11 or scoped classification; no production touch unless routed | release/static | workspace |
| ERR-006 | SCN-VT2F-010 | weak/deferred catalog metadata | catalog rejects exact variants: `EmptyCatalog`, `MissingGivenWhenThen`, `MissingExactAssertion`, `MissingEvidenceDisposition`, `ConflictingEvidenceDisposition`, `InvalidExecutableEvidenceTarget`, `InvalidDeferredFollowUpBead`, `PrivateSurface`, `SharedFixture`, `DuplicateScenarioId`; vt2f no longer deferred; catalog positive assertions use exact vectors/sets, not booleans | catalog integration | catalog src + catalog test |
| WAIVER-TLA-VT2F-001 | scenarios 001/003/007/008 and lifecycle behaviors | BDD-only no semantic runtime change | waiver remains valid only if no runtime/shard/admission semantics edited | proof linkage | no State 8 proof file edit needed |
| WAIVER-TLA-VT2F-002 | SCN-VT2F-009 | strict admission BDD only | waiver remains valid only if strict admission behavior is not implemented/edited | proof linkage | no State 8 proof file edit needed |
| WAIVER-VERUS-VT2F-001 | invariants | no pure/core logic change | waiver remains valid only if no pure/runtime/core transition logic changed | proof linkage | no State 8 proof file edit needed |
| WAIVER-LEAN-VT2F-001 | theorem none | no theorem kernel introduced | no Lean/Aeneas/Hax target needed | proof linkage | no State 8 proof file edit needed |
| GATE-VT2F-001 | release gate | all local evidence after State 8/9/10 | `moon ci` exits 0 or unrelated failures classified `DEFERRED_GLOBAL` | release | workspace |

## 9. State 8 Red/Failing-First Evidence Expectations

State 8 must capture red evidence before weakening or repairing assertions:

1. Initial missing direct target evidence: `cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance` currently fails because no such test target exists. This is expected pre-State-8 red evidence.
2. Catalog red after expectation update: update catalog regression expectations first so `VB-BDD-CATALOG-004` still deferred causes failure, then update catalog row to make it green.
3. API drift red: public names are resolved in this plan; if the checkout changes again, preserve compile failure or assertion failure evidence with exact missing symbol/behavior and route plan repair before weakening any oracle.
4. Strict admission red: SCN-VT2F-009 must fail if strict raw direct submission succeeds. Do not invert the expectation to match permissive behavior; record master/code drift.
5. Weak assertion rejection: any scenario with only `is_ok()`, `is_err()`, `len() > 0`, or no exact event/value/error assertion is not acceptable evidence even if nextest passes.

## 10. Required Commands for State 8 Evidence

Run from isolated workspace only:

```bash
cargo nextest run -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance
cargo nextest run -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog
```

State 8 should not claim `moon ci` closure unless explicitly routed; State 11 owns `GATE-VT2F-001`. State 9 owns `.beads/vb-vt2f/test-review.md` public-surface audit after the State 8 file exists.

## Open Questions

None for State 8 test writing. Exact public API names, typed error variants, stale/wrong-run ticket behavior, post-shutdown behavior, trace drain behavior, catalog validation variants, and catalog assertion strength requirements are resolved in the oracle table above. Any implementation drift must remain red evidence or be routed to an implementation bead; State 8 must not weaken these oracles to make tests pass.
