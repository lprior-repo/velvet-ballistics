# Test Plan: vb-kyyf Cross-Run Determinism and Reproducibility

## Summary
- Bead: `vb-kyyf`.
- Status gates confirmed before planning: `proof-review.md` says `STATUS: APPROVED`; `contract-verification-review.md` says `STATUS: APPROVED`.
- Startup doctrine cited: `/home/lewis/.claude/skills/test-planner/SKILL.md` and `/home/lewis/.agents/skills/test-planner/SKILL.md` both require behavior-first public-API plans, BDD scenarios, property/fuzz/Kani/mutation coverage, exact value/error assertions, and `test-plan.md` output; `/home/lewis/.agents/skills/test-planner/SKILL.md` wins on conflict. `references/testing-philosophy.md` requires public APIs, state-not-interaction assertions, real deps/fakes over mocks, hermetic deterministic tests, DAMP names, and rejection of bare `is_ok()`/`is_err()`.
- Behaviors identified: 7 BDD release behaviors plus 5 pure comparison-kernel behaviors.
- Trophy allocation: 5 unit/pure, 7 integration/BDD, 1 E2E/catalog/CLI acceptance, 1 static gate.
- Proptest invariants: 10.
- Fuzz targets: 5.
- Kani harnesses: 3.
- Mutation threshold: >= 90% kill rate, with listed critical mutants mandatory-killed.

## 1. Behavior Inventory

### Release BDD behaviors
1. `BDD-KYYF-001`: Identical accepted workflow runs produce identical normalized terminal observations when executed in two fresh isolated stores through public runtime/CLI surfaces.
2. `BDD-KYYF-002`: Persisted replay reports remain identical when the store is dropped/reopened and replay/events/inspect surfaces are invoked repeatedly.
3. `BDD-KYYF-003`: Recovery blocks non-replay-safe external actions without re-dispatching side effects when replay crosses a scheduled external-action boundary.
4. `BDD-KYYF-004`: Corrupt, gapped, duplicate, or digest-mismatched replay evidence fails with the same typed storage/replay error on repeated public reads.
5. `BDD-KYYF-005`: Generated mode and IR mode are observationally equivalent for workflows accepted by `validate_generated_subset`.
6. `BDD-KYYF-006`: Unsupported generated-mode IR families fail closed with `UnsupportedGeneratedSubset` or the existing typed generated-subset rejection and never count as parity evidence.
7. `BDD-KYYF-007`: Acceptance runner output is traceable and strong for every `vb-kyyf` pass/fail result.

### Pure kernel behaviors
8. `normalize_observation` strips only allowed cold metadata and preserves every semantic field when normalizing a public observation.
9. `compare_cross_run` returns `Ok(())` only for exact normalized equality and `NondeterministicObservation` for any semantic delta.
10. `compare_replay` prioritizes digest mismatch, replay-policy blocked, sequence violation, then normalized mismatch when comparing replay observations.
11. `compare_generated_ir` returns `UnsupportedGeneratedSubset` before divergence checks and returns `GeneratedIrDivergence` for supported semantic deltas.
12. `DigestStatus::all_match` returns true only when workflow-source, compiled-IR, action-ABI, and policy digests all match.

## 2. Trophy Allocation

| Layer | Count | Behaviors | Planned artifacts / commands | Rationale |
|---|---:|---|---|---|
| Static / proof gate | 1 | PO-010 | `moon ci` plus scoped source-lint/proof gates | Release closure must reject missing evidence or bead-local failures. |
| Unit / Calc | 5 | Pure kernel behaviors 8-12 | `cargo test -p vb_proof_kernels vb_kyyf_normalization --all-features` | Pure comparison algebra is small, deterministic, and must exhaust exact variants. |
| Integration / BDD | 7 | BDD-KYYF-001..007 | `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism -- --test-threads=1`; `cargo test -p vb_storage --test replay_resume`; `cargo test -p vb_storage --test recovery_bdd_tests`; `cargo test -p vb_codegen`; `cargo test -p workspace_tests --test vb_hxm0_acceptance_catalog` | Widest layer because contract is public-surface runtime/storage/codegen/CLI behavior with real stores and durable evidence. |
| E2E / CLI acceptance | 1 | CLI portions of BDD-KYYF-002 and BDD-KYYF-007 | CLI `velvet-ballastics replay/events/inspect` via existing binary harness conventions | Narrow layer validates user-facing replay/event/inspect reports and catalog evidence shape. If CLI missing, assert typed `ScenarioSurfaceUnavailable`. |

Deviation from 60/30/5/5: integration is intentionally dominant because this bead certifies cross-run/replay behavior across runtime, storage, codegen, and CLI boundaries; unit tests cover only the pure normalization kernel.

## 3. BDD Scenarios

### BDD-KYYF-001 — isolated identical runs are deterministic
- Test name: `given_same_accepted_artifact_when_run_twice_then_observations_match`.
- Given: Same accepted compiled artifact, same binary input, same durability profile, and two fresh isolated stores.
- When: Execute the workflow through `Runtime::submit_compiled_with_inputs` or CLI public surface, inspect with `Runtime::inspect_run`, collect journal observations, and normalize.
- Then: Assert exact equality of terminal result, taint, event kind/order signature, significant payload digest, digest status, typed diagnostics, semantic slot/action/suspension/taint signatures.
- And: Assert allowed differences are only temp path, process id, wall-clock timestamp, and generated run id.
- Error variant: if the public runtime or CLI surface is unavailable, assert exact `ScenarioSurfaceUnavailable` diagnostic linked to `BDD-KYYF-001`, not a private-helper fallback.

### BDD-KYYF-002 — persisted replay is reproducible after reopen
- Test name: `given_persisted_run_when_reopened_and_replayed_twice_then_reports_match`.
- Given: Strict or journaled persisted run with durable evidence in an isolated Fjall store.
- When: Drop/reopen the store and invoke `FjallJournal::events_for_run`, `recover_full_journal`, `recover_runtime_summary`, `recover_runtime_frame_seed`, and CLI `replay/events/inspect` twice.
- Then: Assert exact normalized equality of both replay reports, `events_for_run` outputs, recovery summaries, and frame seeds.
- And: Assert sequence numbers are contiguous and monotonic, not merely sorted.
- Error variant: absent/corrupt events must assert exact `RecoveryError`/storage error variant mapped to `ReplaySequenceViolation` where applicable.

### BDD-KYYF-003 — non-replay-safe side effects are not re-executed
- Test name: `given_non_replay_safe_action_when_recovered_twice_then_side_effect_not_reexecuted`.
- Given: A run journal containing a scheduled external action boundary classified as `AtLeastOnceExternal` or non-idempotent.
- When: Recovery/replay is attempted twice through public recovery/runtime surfaces.
- Then: Assert the side-effect dispatch count is unchanged across both attempts.
- And: Assert every attempt returns exact `ReplayPolicyBlocked` typed outcome, including same diagnostic code and normalized report fields.
- Error variant: a second dispatch is a hard failure; do not accept eventual consistency or interaction-only mock verification.

### BDD-KYYF-004 — corrupt replay evidence fails deterministically
- Test name: `given_bad_replay_evidence_when_recovered_twice_then_same_typed_error`.
- Given: Isolated journal/snapshot records covering corruption, sequence gap, duplicate sequence, out-of-order sequence, workflow-source digest mismatch, compiled-IR digest mismatch, action-ABI digest mismatch, and policy digest mismatch.
- When: Replay/recovery is invoked twice through public storage/runtime/CLI surfaces.
- Then: Assert both attempts return the same exact typed error: `ReplayDigestMismatch` for digest mismatch or `ReplaySequenceViolation` for sequence/corruption cases.
- And: Assert no silent continuation and no terminal success event is produced.

### BDD-KYYF-005 — generated mode and IR mode are observationally equivalent for supported workflows
- Test name: `given_generated_supported_workflow_when_ir_and_generated_run_then_observations_match`.
- Given: A workflow accepted by `validate_generated_subset` and equivalent IR/generated fixtures.
- When: IR mode and generated mode execute, persist evidence, replay, and compare via `compare_generated_to_ir` plus normalized observation comparison.
- Then: Assert exact equality of terminal result, taint, journal signature, significant payload digest, suspension state, typed errors, and semantic signatures.
- And: Assert evidence is semantic execution/replay evidence, not only generated-source pattern checks.
- Error variant: supported-workflow divergence asserts exact `GeneratedIrDivergence`.

### BDD-KYYF-006 — unsupported generated subset fails closed
- Test name: `given_unsupported_generated_workflow_when_compared_then_fails_closed`.
- Given: A workflow containing an IR family not accepted by generated mode.
- When: Generated parity certification is requested.
- Then: Assert exact `UnsupportedGeneratedSubset` or existing typed generated-subset rejection.
- And: Assert the scenario is marked as fail-closed, not counted as generated/IR parity evidence.

### BDD-KYYF-007 — evidence artifacts are traceable and strong
- Test name: `given_vb_kyyf_scenario_finishes_when_runner_reports_then_evidence_path_is_traceable`.
- Given: The release acceptance suite runs the `vb-kyyf` group.
- When: Any `BDD-KYYF-*` scenario passes or fails.
- Then: Assert runner output includes bead id, scenario id, Given/When/Then text, public surface, exact mismatch or normalized observation digest, and evidence artifact path.
- And: Assert missing evidence path returns exact `EvidenceArtifactMissing`; private-helper primary paths return exact `ScenarioUsesPrivateSurface`.

## 4. Proptest Invariants

1. `normalize_observation` metadata erasure: for any two observations differing only in temp path/process id/wall-clock/generated-run signatures, normalized observations are equal.
2. `normalize_observation` semantic preservation: for every semantic field, mutating only that field changes the normalized observation exactly in that field.
3. `compare_cross_run` reflexivity: any valid public observation compares equal with itself after normalization.
4. `compare_cross_run` semantic delta rejection: any single semantic delta returns exact `NondeterministicObservation`.
5. `compare_replay` digest priority: any false digest bit in either input returns exact `ReplayDigestMismatch` regardless of later semantic equality.
6. `compare_replay` policy priority: when digests match and either input is replay-policy blocked, result is exact `ReplayPolicyBlocked`.
7. `compare_replay` sequence rejection: when digests match and neither input is policy-blocked, differing event signatures return exact `ReplaySequenceViolation`.
8. `compare_generated_ir` unsupported priority: any unsupported-generated flag returns exact `UnsupportedGeneratedSubset` regardless of other fields.
9. `compare_generated_ir` divergence rejection: supported inputs with any semantic delta return exact `GeneratedIrDivergence`.
10. Journal replay sequence property: generated valid event sequences remain contiguous/monotonic after repeated `events_for_run`/recovery observations; generated gaps/duplicates/out-of-order records map to exact sequence-violation outcomes.

Strategies must generate typed public observations and journal event sequences. Do not hardcode one dummy shape. Shrinking must preserve which field was intentionally mutated so exact error assertions remain meaningful.

## 5. Fuzz Targets

1. `FjallJournal::events_for_run` / journal record decoding: input bytes representing stored events; risk = panic, OOM, silent record skip, unstable corrupt-error mapping; seeds = empty store, single event, duplicate seq, gap, out-of-order, malformed payload, max seq.
2. `recover_full_journal` / recovery event stream: arbitrary bounded `JournalEvent` vectors; risk = side-effect re-dispatch, nonterminal oscillation, wrong latest-attempt filtering, sequence acceptance bug; seeds = strict run, journaled run, scheduled external action, digest mismatch.
3. `recover_runtime_frame_seed` and `recover_runtime_summary`: arbitrary event lists; risk = unchecked indexing/order assumptions, inconsistent summary/frame seed across replay; seeds = no events, admission-only, terminal-only, suspended, failed, blocked.
4. CLI `velvet-ballastics replay/events/inspect`: arbitrary invalid paths/run ids/report bytes where harness supports CLI invocation; risk = panic, nondeterministic diagnostics, untraceable evidence; seeds = missing store, corrupt store, wrong run id, unsupported command.
5. Generated-subset validation/parity input: arbitrary supported/unsupported IR-family fixtures; risk = unsupported workflow accepted as parity evidence or supported workflow rejected nondeterministically; seeds = minimal supported workflow, unsupported IR family, mixed supported/unsupported graph.

## 6. Kani Harnesses

1. `kani_vb_kyyf_normalized_equality_is_metadata_insensitive`.
   - Property: normalized equality ignores only allowed cold metadata and no semantic field.
   - Bound: all enum variants and bounded scalar signatures.
   - Rationale: This is the safety kernel for PRE-004/INV-002 and must satisfy the repository GOD RULE against hardcoded Kani shapes by using `kani::Arbitrary` or exhaustive safe generators for core observation structs.
2. `kani_vb_kyyf_replay_error_priority_is_total`.
   - Property: `compare_replay` priority order is total and deterministic: digest mismatch > policy blocked > sequence violation > normalized mismatch/equality.
   - Bound: all digest booleans, policy booleans, terminal/taint variants, bounded signatures.
   - Rationale: Error taxonomy must not oscillate under repeated replay attempts.
3. `kani_vb_kyyf_generated_ir_error_priority_is_total`.
   - Property: unsupported generated subset is returned before divergence; supported semantic deltas return `GeneratedIrDivergence`; equal supported observations return `Ok(())`.
   - Bound: all enum variants and bounded observation signatures.
   - Rationale: Prevents unsupported workflows from being counted as parity evidence.

## 7. Mutation Checkpoints

Threshold: `cargo-mutants` kill rate >= 90% for scoped touched crates/modules, with all critical mutants below killed. Surviving critical mutants block release even if aggregate threshold passes.

Critical mutants to kill:
- Remove any semantic field from `normalized_observations_equal`: killed by `given_unlisted_difference_when_normalized_then_comparison_fails` and proptest semantic delta cases.
- Include temp path/process id/wall-clock/generated-run metadata in normalized equality: killed by metadata-erasure property and BDD-KYYF-001 two-store run.
- Change digest `&&` to `||` in `DigestStatus::all_match`: killed by per-digest mismatch cases in BDD-KYYF-004 and proptest digest priority.
- Swap `ReplayDigestMismatch` and `ReplayPolicyBlocked` priority: killed by `compare_replay` priority unit/proptest cases.
- Remove event-signature check in `compare_replay`: killed by corrupt/gapped/duplicate sequence scenarios and sequence proptest.
- Return `GeneratedIrDivergence` before `UnsupportedGeneratedSubset`: killed by BDD-KYYF-006 and generated priority proptest/Kani.
- Treat generated source-pattern check as parity success without execution/replay evidence: killed by BDD-KYYF-005 evidence assertion.
- Allow private helper as primary surface: killed by BDD-KYYF-007 catalog validation.
- Omit evidence artifact path from runner output: killed by BDD-KYYF-007 `EvidenceArtifactMissing` scenario.
- Continue replay after digest mismatch: killed by BDD-KYYF-004 no-silent-continuation assertion.

## 8. Combinatorial Coverage Matrix

| Scenario / group | Input class | Expected exact output | Layer |
|---|---|---|---|
| Cross-run identical | same accepted artifact/input/durability, two isolated stores | equal normalized terminal/result/taint/events/digests/diagnostics | integration BDD |
| Cross-run semantic terminal delta | one run terminal differs | `NondeterministicObservation` with mismatch evidence | integration + unit |
| Cross-run taint delta | one run taint differs | `NondeterministicObservation` | integration + unit |
| Cross-run event-order delta | event kind/order differs | `NondeterministicObservation` or sequence violation as public surface maps | integration + property |
| Allowed cold metadata delta | path/pid/wall-clock/run-id differ only | `Ok(())` normalized equality | unit + proptest |
| Persisted replay same store reopened | strict/journaled durable run | identical events, recovery summary, frame seed, CLI reports | integration BDD |
| Empty/missing journal | no events for run | exact no-recovery-data/storage typed error, not success | integration |
| Sequence gap | missing seq N | exact `ReplaySequenceViolation`/mapped storage error | integration + fuzz |
| Duplicate sequence | duplicate seq N | exact `ReplaySequenceViolation`/mapped storage error | integration + fuzz |
| Out-of-order sequence | decreasing seq | exact `ReplaySequenceViolation`/mapped storage error | integration + fuzz |
| Malformed record | corrupt bytes | exact corrupt recovery/storage typed error stable across attempts | integration + fuzz |
| Workflow-source digest mismatch | only source digest false | exact `ReplayDigestMismatch` | integration + unit |
| Compiled-IR digest mismatch | only compiled digest false | exact `ReplayDigestMismatch` | integration + unit |
| Action-ABI digest mismatch | only ABI digest false | exact `ReplayDigestMismatch` | integration + unit |
| Policy digest mismatch | only policy digest false | exact `ReplayDigestMismatch` | integration + unit |
| Non-idempotent action boundary | scheduled non-replay-safe action | exact `ReplayPolicyBlocked`; dispatch count unchanged | integration BDD |
| At-least-once action boundary | scheduled at-least-once action | exact stable blocked/policy outcome; dispatch count unchanged | integration BDD |
| Deterministic pure action | replay-safe pure action | replay succeeds with equal normalized observation | integration |
| Generated supported equal | `validate_generated_subset` accepts and observations equal | `Ok(())`; parity evidence recorded | integration + unit |
| Generated supported divergent terminal | supported but terminal differs | exact `GeneratedIrDivergence` | integration + unit |
| Generated supported divergent journal | supported but event signature differs | exact `GeneratedIrDivergence` | integration + unit |
| Generated unsupported family | unsupported IR family | exact `UnsupportedGeneratedSubset`/typed subset rejection; no parity credit | BDD + unit |
| CLI replay/events/inspect present | valid persisted run | stable report with scenario id/evidence path | E2E |
| CLI replay/events/inspect absent | command unavailable | exact `ScenarioSurfaceUnavailable` gap diagnostic | E2E |
| Catalog evidence missing | scenario output lacks artifact path | exact `EvidenceArtifactMissing` | integration/catalog |
| Private helper primary path | scenario row names private helper only | exact `ScenarioUsesPrivateSurface` | integration/catalog |

## 9. Required Exact Commands / Evidence Mapping

| Requirement | Planned command | Evidence required |
|---|---|---|
| BDD-KYYF-001 | `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism -- --test-threads=1` | `.evidence/vb-kyyf/bdd-cross-run-determinism.md` with identical normalized digests. |
| BDD-KYYF-002 | `cargo test -p vb_storage --test replay_resume` | `.evidence/vb-kyyf/storage-replay-resume.md` with repeated reopen/replay equality and sequence proof. |
| BDD-KYYF-003 | `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism` | `.evidence/vb-kyyf/non-replay-safe-actions.md` with unchanged dispatch count and stable `ReplayPolicyBlocked`. |
| BDD-KYYF-004 | `cargo test -p vb_storage --test recovery_bdd_tests` | `.evidence/vb-kyyf/recovery-bdd-errors.md` with stable typed corrupt/digest errors. |
| BDD-KYYF-005 | `cargo test -p vb_codegen` | `.evidence/vb-kyyf/generated-ir-parity.md` with semantic execution/replay parity, not pattern-only evidence. |
| BDD-KYYF-006 | `cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism` | `.evidence/vb-kyyf/generated-subset-fail-closed.md` with fail-closed unsupported subset result. |
| BDD-KYYF-007 | `cargo test -p workspace_tests --test vb_hxm0_acceptance_catalog` | `.evidence/vb-kyyf/acceptance-catalog-traceability.md` with scenario id, GWT text, public surface, evidence path. |
| VERUS-KYYF-001 regression | `verus verification/verus/vb_kyyf_normalization.rs` | `42 verified, 0 errors` or updated reviewed proof evidence. |
| Pure kernel cargo regression | `cargo test -p vb_proof_kernels vb_kyyf_normalization --all-features` | all focused tests pass with exact error/value assertions. |
| Release gate | `moon ci` | exit 0 or only approved unrelated `DEFERRED_GLOBAL`; no bead-local failures. |

## Open Questions

1. The exact CLI binary test harness shape remains unconfirmed by the contract. Test writer must discover existing CLI conventions before implementation; absence must become `ScenarioSurfaceUnavailable`, not a private helper fallback.
2. Generated semantic parity API completeness remains unconfirmed. Test writer must require execution/replay observations and must not accept `compare_generated_to_ir` source-pattern checks alone as BDD-KYYF-005 evidence.
3. If exact public diagnostic enum names differ from the contract taxonomy, tests must assert the existing typed error plus a documented mapping to ERR-001..ERR-009; bare string matching is insufficient unless no typed surface exists.
