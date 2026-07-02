bead_id: vb-qi37.25
bead_title: quality: Workspace assertion sharpness and spelling gates
phase: 13
updated_at: 2026-05-18T14:41:00Z
attempt: 1-of-7
STATUS: APPROVED

# Assurance bundle

## Scope
Acceptance requires exact assertions for workspace membership, crate/package names, binary names, feature flags, forbidden dependencies, and canonical spelling rejection outside the allowlist; tests must be mutation-resistant and not broad substring checks.

## Requirement evidence map
- R1 exact workspace membership
  - Contract: contract.md R1.
  - Tests/evidence: vb_8ma2_workspace_assertions::unexpected_workspace_member_fails_exact_gate; rtk bash scripts/check-workspace-assertions.sh PASS; moon :workspace-assertions PASS.
  - Ledger: PO-vb-qi37.25-1 PASS.
- R2 exact package/crate names
  - Contract: contract.md R2.
  - Tests/evidence: vb_qi37_25_quality_gates::package_name_drift_reports_exact_member_and_expected_name PASS.
  - Ledger: PO-vb-qi37.25-1 PASS.
- R3 exact binary names
  - Contract: contract.md R3.
  - Tests/evidence: vb_qi37_25_quality_gates::binary_alias_reports_exact_allowed_binary_set PASS.
  - Ledger: PO-vb-qi37.25-1 PASS.
- R4 exact feature flags / forbidden feature names
  - Contract: contract.md R4.
  - Tests/evidence: vb_qi37_25_quality_gates::feature_drift_reports_exact_expected_feature_set PASS.
  - Ledger: PO-vb-qi37.25-1 PASS.
- R5 forbidden dependencies
  - Contract: contract.md R5.
  - Tests/evidence: vb_8ma2 dependency tests PASS; generated-boundary token test PASS.
  - Ledger: PO-vb-qi37.25-1 PASS.
- R6 canonical spelling gate / exact allowlist
  - Contract: contract.md R6.
  - Tests/evidence: vb_37lc canonical spelling suite PASS (76 tests); vb_qi37_25 spelling_gate_rejects_legacy_spelling_outside_exact_allowlist and broad_substring_allowlist_is_configuration_error PASS.
  - Ledger: PO-vb-qi37.25-2 PASS.

## Review evidence
- proof-review.md: STATUS: APPROVED.
- contract-verification-review.md: STATUS: APPROVED.
- test-plan-review.md: STATUS: APPROVED.
- test-suite-review.md: STATUS: APPROVED.
- formal-verification-report.md: STATUS: APPROVED.
- black-hat-review.md: STATUS: APPROVED.

## Machine gate evidence
- machine-gate-report.md: STATUS: PASS.
- regression-diff.md: STATUS: PASS.
- moon ci: PASS, 23 completed; 10946 tests passed, 44 skipped; mutants-smoke 1 caught.

## Waivers/debt
No waivers required. No remaining State 11 blockers.
