# vb-my3b3 handoff — DeepSeek author

## What changed

- Rewrote `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`
  (22 tests) to use the real checked producer (`prepare_compiled_artifact`) and
  typed schema-3 evidence instead of hand-built legacy `AcceptedArtifact` flags.
- Deleted stale `vb_qi37_4_2_strict_runtime_admission.proptest-regressions`
  (seed parameter names no longer exist).
- No production code changed. No other fixture files touched.

## Verification (immutable runner, RUSTFLAGS=-Dwarnings)

`python3 /tmp/vb-run-gate.py 900 .beads/vb-my3b3/evidence/qi37-final env RUSTFLAGS=-Dwarnings cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_1_1_red_recovery_contract_test --test vb_qi37_25_quality_gates --test vb_qi37_2_4_integration_budget_errors --test vb_qi37_4_2_strict_runtime_admission --test vb_qi37_6_state8_setup`

- vb_qi37_1_1_red_recovery_contract_test: 19 passed
- vb_qi37_25_quality_gates: 3 passed
- vb_qi37_2_4_integration_budget_errors: 47 passed
- vb_qi37_4_2_strict_runtime_admission: 22 passed
- vb_qi37_6_state8_setup: 2 passed
- Total: 93 passed, 0 failed, 0 ignored, 0 warnings (exit 0)

Formatting: `rustfmt --edition 2024 --check` clean on the touched file.

## Test inventory (vb_qi37_4_2_strict_runtime_admission.rs)

1. given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation
2. given_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest
3. given_unverified_legacy_evidence_when_strict_run_created_then_evidence_denies
4. given_checked_artifact_when_prepared_then_gate_count_and_durability_are_not_admission_authority
5. accepted_ledger_distinguishes_runtime_validated_from_operator_attested
6. given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies
7. given_stale_artifact_when_strict_run_created_then_stale_certificate_denies
8. given_missing_prefix_or_action_mismatched_capability_then_capability_denied
9. given_required_capability_with_extras_or_duplicates_then_denied
10. given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile
11. given_budget_over_capacity_when_admission_with_budget_runs_then_resource_capacity_error_is_preserved
12. given_raw_or_malformed_storage_bytes_when_strict_run_created_then_decode_failed_matrix_denies
13. given_evidence_schema_tamper_matrix_when_strict_run_created_then_typed_invalid_diagnostic_denies
14. given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved
15. given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated
16. given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required
17. given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called
18. given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied
19. proptest_capability_profiles_admit_iff_required_capability_is_exactly_granted
20. proptest_evidence_schema_version_fails_closed
21. proptest_verification_digest_must_equal_artifact_digest
22. proptest_diagnostic_mapping_is_injective_over_admission_error_categories

## Contract-parity notes

- Obsolete `REQUIRED_GATE_COUNT == 15` oracle and individual proof-flag
  (`bounded`/`taint_safe`/`retry_safe`/`durable`/`replayable`) oracles were
  replaced with the typed-evidence invariant (schema 3, canonical 10-entry
  ledger, `RuntimeValidated` vs `ExternalAssumption`, no `OperatorAttested`).
- Strong negatives retained: tamper/rejection matrix, exact rejection payloads,
  fail-closed schema-version proptest, digest-binding proptest, and the
  no-allocation-on-rejection invariant.
- Root owns VCS: no commits/bookmarks/push/bead close performed here.
