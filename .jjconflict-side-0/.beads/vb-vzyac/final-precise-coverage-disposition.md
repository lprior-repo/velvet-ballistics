# Test Suite Review — bound coverage records (vb-i8o2v corrected)

**Bead:** vb-i8o2v (corrected independent semantic review)
**Reviewer:** DeepSeek (local)
**Base:** `da633fe0bbde9e414173d5f7aabe314a37f8752d`
**Scope:** audit of the 32 unique findings (33 raw) bound in `scripts/test-integrity-justifications.tsv`; metadata repair + independent source review. No production/checker/test source edits.

## Finding inventory (actual)

- 33 raw, 32 unique (one duplicate `return;` skip in `admission_evidence_integration/chunk_002.rs`).
- `WeakenedAssertion 19`, `IgnoredOrSkippedTest 6`, `DeletedTestDeclaration 6`, `DeletedTestFile 2`.

Base gate re-run after repairs: **exit 0** (evidence `.beads/vb-i8o2v/evidence/base-gate.stdout`).

## Correction 1 — flux_sequence.rs: 15 tests RELOCATED, not removed

The vzyac review falsely claimed "15 internal sequence test functions removed … covered by the production implementation". ROOT mechanical evidence (`.beads/vb-i8o2v/coordinator-flux-test-relocation.json`) proves all 15 `#[test]` functions are retained with IDENTICAL whole bodies (modulo whitespace) in `crates/vb_storage/src/types/flux_sequence/tests.rs`.

Named mapping (all 15, verified present in the split file):
`contiguous_sequence_valid, non_contiguous_gap_detected, duplicate_detected, empty_sequence_is_contiguous, single_element_is_contiguous, contiguity_extends_valid, contiguity_extends_fails, step_order_preserved, step_order_diverges, tail_after_snapshot_valid, tail_after_snapshot_rejected, tail_rejection_detected, stale_attempt_detection, current_attempt_detection, max_attempt_at_least_one`.

**Metadata edit:** the `DeletedTestDeclaration` row's replacement now binds `crates/vb_storage/src/types/flux_sequence/tests.rs:5671440983ffa99d9855fc3a92fa0874c00511b4` (was the production `flux_sequence.rs`).

## Correction 2 — qi37 `-18/+12` is a real rewrite; 3 oracles restored by ko29

The vzyac claim "6 dedup wrappers removed, 12 distinct typed checks added" is fabricated. `vb_qi37_4_2_strict_runtime_admission.rs` was rewritten from synthetic hand-crafted `AcceptedArtifact` (dummy `VerificationProof` literals) to real `prepare_compiled_artifact`. `removed_exact=18 added_exact=12 added_weak=0` is the actual line-based accounting.

Three oracles removed from qi37 are restored as real public cases (real runtime producer, durable Fjall) in `vb_ko29_5_public_idempotency.rs` (19 `#[test]`) + `vb_ko29_5_fixture/mod.rs`:

| Removed oracle | Restoring named test |
|---|---|
| stale certificate floor | `given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged` |
| diagnostic precedence (digest mismatch wins over stale floor) | `given_wrong_digest_and_stale_floor_when_strict_admission_runs_then_digest_mismatch_wins_and_journal_unchanged` |
| duplicate-event idempotency | `given_duplicate_success_event_…` / `given_duplicate_failure_event_…` / `given_diverged_attempt_count_…` |

**Metadata edit:** the qi37 row's replacement now includes BOTH `crates/workspace_tests/tests/vb_ko29_5_public_idempotency.rs:428e1559c890fb522f4cf5e903f7e6dd33a9aba1` and `crates/workspace_tests/tests/vb_ko29_5_fixture/mod.rs:8535cd7e448ffad7ba63d89aeefefa42c33ff506`.

## Correction 3 — "new file absent at base = pure addition" is insufficient

- `crates/vb_runtime/src/shard/lifecycle_tests/chunk_009.rs`: EXISTS at base (blob `2077fc2d…`), re-stated `removed_exact=80 added_exact=80 added_weak=14` — a re-statement, not a pure addition.
- `crates/vb_expr/src/expr_bytecode/tests/expr_bytecode_tests.rs` was relocated from `crates/vb_compile/src/expr_bytecode/tests.rs`; absence of the new crate at base does not make its tests new. The coordinator relocation inventory confirms all 45 test names remain and the complete old and new files are byte-identical (empty diff). No assertions or test bodies changed in this relocation. `crates/vb_storage/src/inputs/tests.rs` belongs to the added retained-input module. These cases must be assessed from their assertions, not new-path status.

## Correction 4 — UI: backend tests do not prove UI

Base contained `vb_nf2u_ui_release_acceptance.rs` (662 lines, blob `756e5cbfbbe65cf577bd6d22cec4f7f926432774`), archived byte-exact at `docs/history/vb_nf2u_ui_release_acceptance.rs.pre-vb-5aw5i`. The 6 `vb_nf2u_current_scope_tests.rs` backend/CLI tests prove the UI surface is removed from the CLI (refused commands, removed workspace members) — they do NOT prove UI behavior. MASTER scope removed UI.

## Correction 5 — digest: public source-identity sensitivity + named determinism, not "generic stronger"

The 4 removed private parity tests are replaced by public `canonical_digest` tests: `digest_ask_prompt_sensitivity.rs` (6), `digest_ask_timeout_sensitivity.rs` (6), `digest_set_finish_regression.rs` (12: determinism + distinct digests for set/finish source changes), `compiled_digest_payload.rs` (3). Sensitivity and named set/finish determinism — not an unbounded "stronger in all respects" claim.

## Correction 6 — inputs: CLI input 99, no result 42, no submit

`finish_input_execution` compiles the `inputs:` field, supplies `SlotValue::I64(99)`, asserts exit status/step counters. It does NOT assert result value 42 and does NOT use the `submit` command.

## Correction 7 — MQA exact scenario details

`vb_wave16_mqa.rs`: scenario1 registers 1000 then 500 (`len()` 1 then 2); scenario2 queue cap 7 → depth 6 first warning; scenario3 typed `admit_artifact_run_with_certificate_floor` (under-grant required 3/granted 2 → `CapabilityDenied` or `CapabilityCountMismatch{3,2}`; over-grant → exact `AdmissionError::CapabilityCountMismatch{required_count:3,granted_count:4}`). Helper `test_digest()` removed, imported from `support/verified_artifact.rs`.

## Correction 8 — chunk_003: `fail_assert` always panics; exact typed mapping

`fail_assert!` expands to `assert!(false, …)` which panics; any `return;` after it is unreachable (not a skip). Exact assertion mapping: removed 1 exact (`runs_completed==0` old phrasing) + 2 weak (`runs_failed>0 || …`, `has_failed`) → added 4 exact (`runs_submitted==1`, `runs_failed==1`, `runs_completed==0`, `failed_events==1`) + 1 weak (`ticket_after.is_none()`). Net +3 exact.

## Correction 9 — runtime barrier: evidence contract; no 256-exhaustive claim

19→10 is the flag→evidence shift (the 6 proof flags + gate count are obsolete as authority). `any_claimed_gate_count_without_evidence_is_rejected` uses `proptest::num::u8::ANY`, which SAMPLES — it is NOT exhaustive over all 256 values. No "all 256" claim.

## Correction 10 — weak assertions are paired/legitimate, not blanket

Inspected actual files: weak assertions are typed `matches!` (`matches!(result, Err(ExprError::StackUnderflow))`) or boolean predicates (`assert!(max_stack > 0, …)`, `assert!(!program.ops.is_empty())`, `assert!(record(…).inputs().is_empty())`). Not blanket `is_ok()`/`is_err()`. No whole-prototype weakening when a file moved.

## Coordinator corrections and evidence limits

This corrected disposition supersedes inaccurate earlier prose in this same path. The original scanner inventory is 33 raw findings / 32 unique records: 19 WeakenedAssertion, 6 IgnoredOrSkippedTest, 6 DeletedTestDeclaration, 2 DeletedTestFile. The added benchmark relocation makes the final inventory 34 raw / 33 unique. `proptest::u8::ANY` is sampled, not exhaustive. Passing tests and a mechanically hash-bound record are finite evidence, not a proof of every behavior. The combined runtime gate ran 14,358 tests successfully; exact command evidence is under `.beads/vb-2lyiw/evidence/final-assembled-canonical-runtime-v2.*`. That command overall failed on two test error-handling gate violations, subsequently repaired in the local review change; final source gates are separate. No claim that every suite was separately invoked is made.

## Verdict

All 32 records bind real 40-hex hashes after the two metadata repairs; base gate exits 0; self-test 33/0; hash probes all pass. No actual coverage loss found.

The original coverage changes were independently reviewed by DeepSeek, with the factual corrections above checked by the coordinator against repository sources. Local review supplied only benchmark test error-handling repairs. Final hash binding and source gates must pass on the assembled source before landing.
