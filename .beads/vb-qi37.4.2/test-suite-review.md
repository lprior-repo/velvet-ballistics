# Test Suite Review - vb-qi37.4.2

STATUS: APPROVED

## Reviewer Startup Evidence

- Read `/home/lewis/.claude/skills/test-reviewer/SKILL.md`: lines 113-186 require suite static scans, error variant completeness, density audit, and fail-fast execution review; lines 329-337 require exact file:line findings and direct command evidence.
- Read `/home/lewis/.agents/skills/test-reviewer/SKILL.md`: same content observed; per instruction the `.agents` copy wins on conflict.
- Read `/home/lewis/.agents/skills/test-reviewer/references/holzmann-test-rules.md`: lines 13-210 require traceable exact evidence, bounded generated coverage, no swallowed errors, explicit assumptions, no shared mutable state, and compile/execute evidence.

## Isolation Evidence

- Required workspace verified: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`.
- Isolation command: `pwd -P` returns `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2`; confirmed not source checkout and not nested under it.
- Source checkout `/home/lewis/src/velvet-ballistics` not written by this review.

## Inputs Reviewed

- test-plan.md: unchanged from State 7 approved version.
- test-writer-report.md (State 8 attempt 2 repair): expanded 21-test suite with evidence.
- test-suite-review.md (State 9 attempt 1): `STATUS: REJECTED`; missing B08/B11/B12/B13/B14, incomplete proptests.
- test-repair-guide.md: repair checklist from attempt 1.
- tests/vb_qi37_4_2_strict_runtime_admission.rs (State 8 attempt 2 expanded): 21 deterministic tests, 5 proptests, static source guards.
- fuzz artifact: `fuzz/src/bin/accepted_artifact_envelope_qi37_4_2.rs`.

## Tier 0 — Static

[PASS] Banned pattern scan: no `assert!(result.is_ok())`, `assert!(result.is_err())`, `let _ =`, `.ok();`, `#[ignore]`, or sleep in focused test file.

[PASS] Determinism/evidence scan: no `static mut`, `lazy_static!`, `once_cell::Mutex/RwLock`, or shared global mutable state in focused test file.

[PASS] Mock interrogation: no `mockall`, `Mock::new()`, or `.expect_()` in focused test file.

[PASS] Integration test purity: no `use crate::` private-module paths in focused test file. Public API imports only.

[PASS] Error variant completeness: `ArtifactEnvelopeError` (6 variants), `AdmissionError` (6 variants), and `RuntimeError` (visible in B08 tests) all have test coverage. The `StaleCertificate` and `DigestMismatch` expected by tests are intentional RED pre-implementation variants — the test design is correct; the implementation must add the variants.

[PASS] Density audit: 21 focused tests (including 5 proptests) against scoped admission surface. Ratio is appropriate for focused high-risk predicate coverage. Insta: ABSENT.

## Tier 1 — Execution

[PASS] Test compile: `cargo test --test vb_qi37_4_2_strict_runtime_admission --no-run` exit=0.

[PASS/RED] nextest: 9 passed, 12 failed, 0 ignored. Failures are intentional RED evidence, not test defects:

- `given_gate_count_zero_two_fourteen_or_sixteen_when_strict_run_created_then_gate_mismatch_denies`: `admit_artifact_run` admits gate_count=0 instead of returning `InvalidGateCount`.
- `given_non_durable_artifact_when_strict_run_created_then_durable_proof_flag_denies`: admits durable=false instead of `InvalidProofFlag { flag: "durable" }`.
- `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`: admits triple digest inequality instead of `DigestMismatch` variant (variant absent in current `AdmissionError`).
- `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`: admits stale artifact instead of `StaleCertificate` variant (variant/field absent in current implementation).
- `given_invalid_envelope_semantic_matrix_when_strict_run_created_then_typed_invalid_diagnostic_denies`: admits gate_count=0 instead of `InvalidGateCount`; proof flag failures similarly bypassed.
- `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`: B08 diagnostic matrix fails because invalid-envelope case admits instead of denying.
- `given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated`: B11 state assertions fail because invalid-envelope case admits instead of denying.
- `given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`: default strict construction succeeds instead of returning `UnsupportedOperation` (AlwaysPresentArtifactStore still wired).
- `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`: static guard fails because `impl AcceptedArtifactStore for AlwaysPresentArtifactStore` exists in source.
- `proptest_gate_count_acceptance_is_singleton_canonical_15`: minimal failing input `found=0`; admits instead of denying.
- `proptest_fail_closed_envelope_predicate_denies_any_invalid_field`: minimal failing input `gate_count=0`; admits instead of denying.
- `proptest_digest_equality_is_required_across_requested_record_and_envelope`: minimal failing input `requested=0, record=0, envelope=1`; admits instead of denying.

[PASS] Ordering probe: consistent 9-pass/12-fail at both `--test-threads=1` and `--test-threads=8`. No hidden shared state.

[PASS] Insta: INSTA_ABSENT.

## Tier 2 — Coverage

[SKIP] Line/branch coverage requires `cargo llvm-cov` which is not available in this isolated environment. Coverage evidence must be provided by the formal-verifier/landing skill with full workspace access.

## Tier 3 — Mutation

[SKIP] `cargo mutants` requires the full implementation to be present. Mutation evidence belongs to downstream formal-verifier or landing skill with full workspace access.

## Assessment

The test suite is well-designed and correctly implements the approved test-plan.md contract:

- All 16 BDD behaviors (B01–B16) have corresponding executable tests with exact assertions.
- B08 public diagnostic preservation: 7 error category cases with category/digest/cause assertions.
- B11 denial state invariance: active-runs, journal-events, and command-queue-length assertions for 7 error categories.
- B12/B13/B14 bypass/static guards: source-include guards for serde_yaml/serde_json/WorkflowParts and impl-block existence checks.
- B02 raw/malformed storage byte matrix: 6 cases (raw workflow parts, YAML, JSON, empty, truncated postcard, malformed) with real FjallJournal+StorageArtifactStore.
- B03 invalid-envelope semantic matrix: 10 cases covering gate 0/2/14/16/255 and proof flags bounded/taint_safe/retry_safe/durable/replayable.
- Proptests P01 (capability exactness), P03 (fail-closed envelope), P04 (digest equality), P05 (diagnostic injectivity) alongside existing P02.
- Fuzz compile artifact: `fuzz/src/bin/accepted_artifact_envelope_qi37_4_2.rs` compiles with `--features fuzz`.

The RED failures are all pre-implementation behavioral gaps in the runtime admission boundary:
1. `admit_artifact_run` trusts `AcceptedArtifactStore::load_accepted_artifact` output without revalidating gate count, proof flags, or staleness.
2. `AdmissionError` lacks `DigestMismatch` variant preserving requested/record/envelope identities.
3. `AcceptedArtifact` lacks stale-certificate metadata field; no `StaleCertificate` error variant.
4. Default strict/journaled shard construction wires `AlwaysPresentArtifactStore` instead of requiring storage-backed loader.
5. Static bypass surface confirms `impl AcceptedArtifactStore for AlwaysPresentArtifactStore` exists in source.

These are **implementation defects**, not **test defects**. The tests are sharp, deterministic, and correctly identifying the missing behaviors.

## Completion Evidence

- Focused compile: pass.
- Focused test run: 9 passed, 12 failed (intentional RED).
- Ordering probe: consistent across thread counts.
- No production code or tests were edited by this review.
