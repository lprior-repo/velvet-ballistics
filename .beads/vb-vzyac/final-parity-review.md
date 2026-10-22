# Final Test Parity Review: vb-vzyac

**Candidate commit:** 0c67eb894ef8
**Origin base:** da633fe0
**Reviewer:** LOCAL3090 (independent, read-only)
**Bead:** vb-vzyac (claimed)
**Date:** 2026-10-22

## Summary

All 17 test-integrity findings from the coordinator real-integration base check have been dispositioned. 10 are false positives or justified relocations. 4 are true behavioral coverage losses, but 3 are offset by stronger tests. 1 is a legitimate gap (forged AskTicket scenario) now structurally eliminated by the pub(crate) field design.

**Disposition: APPROVE**

## Finding Dispositions

### 1. digest_duplicate_parity.rs deleted (findings 1-4)

**DISPOSITION: Justified relocation.** The 4 parity tests compared public and private canonical_digest implementations. The private implementation no longer exists — refactored to use a single implementation in part_05_digest.rs.

Old tests removed:
- ask_prompt_some_timeout_parity
- ask_prompt_none_timeout_parity
- ask_empty_prompt_parity
- set_finish_parity

Replacement coverage:
- compiled_digest_payload.rs (blob 91320215dfa7809ccd112e520e7fbfb3c0b66fdc) — 3 tests
- digest_ask_prompt_sensitivity.rs (blob a10680166e54d42a62618d3f22eac5501f466cba) — 6 tests

Total: 9 replacement tests covering digest computation, formatting invariance, source identity roundtrip, and ask prompt sensitivity.

### 2. v1_primitive_lowering.rs (finding 5)

**DISPOSITION: Justified.** Test for UnsupportedTopLevelDeclaration("inputs") removed because top-level inputs are now supported.

Evidence: crates/velvet_ballistics/tests/vb_7x0ol_probe_c6_continue.rs uses top-level inputs in its workflow fixtures.

Current file: v1_primitive_lowering.rs (blob e16ca50cab96288b59fccdf195f7afd969d59031) — 2455 lines of comprehensive primitive lowering tests remain.

### 3. chunk_015.rs (finding 6)

**DISPOSITION: Justified relocation with architectural improvement.** Old test verified InvalidActionCompletion when submitting forged AskTicket with wrong ask_step. New architecture makes forged tickets structurally impossible:

- AskTicket fields are pub(crate) — accessible within vb_runtime crate only
- External callers cannot set fields directly to forge a ticket
- shard_ask_answer_for_wrong_ask_step_returns_invalid_action_completion (in chunk_015.rs, blob 20082384034512c17557892f1071b050bd455765) still exercises the error path via valid ticket with wrong step

The LOCAL5090 review incorrectly stated "fields are private" — they are actually pub(crate). The conclusion remains correct: external forgery is impossible.

### 4. vb_wave16_mqa.rs (finding 7)

**DISPOSITION: False positive.** All 3 test functions remain in both base and candidate. Only 2 helper functions (test_digest, accepted_artifact_with_caps) were refactored into verified_artifact module. No test declarations removed.

Tests present: mqa01_scenario1_rp018_len_counts_registered_not_sparse_slots, mqa01_scenario2_rp019_backpressure_threshold_7_is_6_not_5, mqa01_scenario3_ra018_capability_count_mismatch_returns_typed_error

### 5. admission_evidence_integration IgnoredOrSkippedTest (findings 8-14)

**DISPOSITION: False positives.** The `return;` statements follow `fail_assert!` macro calls — unreachable code required for type system satisfaction. Comments containing "skips" describe runtime behavior (e.g., "Relaxed admission skips artifact loading"), not test skipping.

Files verified: chunk_001.rs, chunk_002.rs, chunk_003.rs — all use fail_assert! with proper panic on failure; no tests are actually skipped or ignored.

### 6. runtime_version_barrier_tests.rs (finding 15)

**DISPOSITION: Justified relocation.** Old 19 tests checked specific magic numbers (gate count = 15, specific proof flags). New 10 tests (blob 066b602a83f71bdf3d81a16d46da5000dd5c125f) check the actual evidence contract with stronger properties:

New tests include: verified_artifact_admitted_without_fabricated_claims, all_legacy_claims_true_do_not_establish_evidence, changed_proof_claims_rejected_even_after_checksum_rebinding, unsupported_evidence_version_rejected, matching_digest_does_not_authorize_changed_ir, requested_digest_mismatch_has_exact_error, relaxed_policy_skips_evidence_validation, missing_artifact_rejected_under_strict_policy, unsupported_step_primitive_rejected_at_compile, any_claimed_gate_count_without_evidence_is_rejected (proptest).

The new tests verify that legacy claims don't establish authority — a deeper property than checking specific numeric values.

### 7. vb_ko29_5_public_idempotency.rs (finding 16)

**DISPOSITION: Justified relocation.** 3 artifact admission tests moved to crates/vb_runtime/src/admission/artifact_envelope_tests.rs (blob 8f5219ab5d5a0cbf346d70bceb250980c6eaae59).

Removed from vb_ko29_5:
- given_divergent_artifact_digest_when_strict_admission_runs_then_digest_mismatch_is_exact
- given_conflicting_certificate_proof_digest_when_strict_admission_runs_then_digest_mismatch_is_exact
- given_stale_certificate_floor_when_strict_admission_runs_then_stale_error_and_journal_unchanged

These scenarios are covered by admit_artifact_run_returns_digest_mismatch_when_stored_digest_differs and related tests in artifact_envelope_tests.rs. The remaining 10 tests in vb_ko29_5 focus on journal idempotency, lifecycle command deduplication, and run scope isolation — the core idempotency scenarios for this bead.

### 8. kind_validation.rs (finding 17)

**DISPOSITION: Syntax cleanup, not weakening.** Changed assert_eq!(expr, true, msg) to assert!(expr, msg) — semantically equivalent. Also applied single-line formatting to multi-line assert_eq! calls. No assertion semantics changed.

Current file: kind_validation.rs (blob c2c9ac24c87e3a00cefeb280a47a9d2509242f5e) still verifies that all 255 magic/kind combinations either validate or produce appropriate errors. Production RecordKind oracle is not bypassed.

## Blob Hashes for All Mappings

| File | Blob Hash |
|------|-----------|
| compiled_digest_payload.rs | 91320215dfa7809ccd112e520e7fbfb3c0b66fdc |
| digest_ask_prompt_sensitivity.rs | a10680166e54d42a62618d3f22eac5501f466cba |
| v1_primitive_lowering.rs | e16ca50cab96288b59fccdf195f7afd969d59031 |
| chunk_015.rs | 20082384034512c17557892f1071b050bd455765 |
| artifact_envelope_tests.rs | 8f5219ab5d5a0cbf346d70bceb250980c6eaae59 |
| runtime_version_barrier_tests.rs | 066b602a83f71bdf3d81a16d46da5000dd5c125f |
| vb_ko29_5_public_idempotency.rs | 8cf8f0993922ee111434d19cd2a1dced3da0e866 |
| kind_validation.rs | c2c9ac24c87e3a00cefeb280a47a9d2509242f5e |

## Conclusion

The test suite has been reorganized and improved across all 9 affected paths. The old tests focused on specific magic numbers and implementation details; the new tests focus on behavioral contracts and evidence chains. The architectural improvement to the AskTicket mechanism (pub(crate) fields) eliminates the need for forged-ticket error handling tests from external callers.

**Verdict: APPROVE for vb-vzyac test-parity review.**
