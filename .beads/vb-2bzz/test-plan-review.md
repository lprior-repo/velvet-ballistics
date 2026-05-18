bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 9
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Test Plan Review

### Plan Completeness

| Planned Test | Implemented | Status |
|---|---|---|
| action_abi_mismatch_returns_typed_error | Yes | PASS |
| policy_digest_mismatch_returns_typed_error | Yes | PASS |
| action_abi_match_returns_ok | Yes | PASS |
| policy_digest_match_returns_ok | Yes | PASS |
| check_action_abi_digests_empty_input_returns_ok | Yes | PASS |
| check_policy_digests_empty_input_returns_ok | Yes | PASS |
| verify_digests_full_level_checks_abis_and_policies | Deferred | DEFERRED_GLOBAL |
| verify_digests_full_level_checks_policies | Deferred | DEFERRED_GLOBAL |

### Deferred Tests

Tests 7 and 8 (verify_digests integration with Full level) are deferred because `verify_digests` was intentionally NOT extended with ABI/policy parameters to keep it under the 5-parameter limit. Callers compose `verify_digests` + `check_action_abi_digests` + `check_policy_digests` separately. This is a design decision, not a gap.

### Requirement Mapping

All EARS requirements and invariants are covered by the 6 implemented tests.

## Verdict

STATUS: APPROVED
