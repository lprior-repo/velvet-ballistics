bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 7
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Test Plan

### Test 1: action_abi_mismatch_returns_typed_error (unignore + fix)
- **Given**: A journal with ActionScheduled events and an expected ABI digest map
- **When**: check_action_abi_digests is called with a mismatched ABI for one action
- **Then**: Returns `RecoveryError::ActionAbiMismatch { action_id }` with the exact action_id
- **Coverage**: EARS-1, INV-1, INV-4

### Test 2: policy_digest_mismatch_returns_typed_error (unignore + fix)
- **Given**: A journal with RunAdmission events and an expected policy digest map
- **When**: check_policy_digests is called with a mismatched policy digest for one step
- **Then**: Returns `RecoveryError::PolicyDigestMismatch { step }` with the exact step
- **Coverage**: EARS-2, INV-2, INV-4

### Test 3: action_abi_match_returns_ok
- **Given**: A journal with ActionScheduled events
- **When**: check_action_abi_digests is called with matching ABI digests
- **Then**: Returns Ok(())
- **Coverage**: INV-1, INV-3

### Test 4: policy_digest_match_returns_ok
- **Given**: A journal with RunAdmission events
- **When**: check_policy_digests is called with matching policy digests
- **Then**: Returns Ok(())
- **Coverage**: INV-2, INV-3

### Test 5: check_action_abi_digests_empty_input_returns_ok
- **Given**: A journal with ActionScheduled events
- **When**: check_action_abi_digests is called with empty expected_abis
- **Then**: Returns Ok(()) (no guessing from missing data)
- **Coverage**: EARS-3, INV-1

### Test 6: check_policy_digests_empty_input_returns_ok
- **Given**: A journal with RunAdmission events
- **When**: check_policy_digests is called with empty expected_policy_digests
- **Then**: Returns Ok(()) (no guessing from missing data)
- **Coverage**: EARS-3, INV-2

### Test 7: verify_digests_full_level_checks_abis_and_policies
- **Given**: A journal with ActionScheduled and RunAdmission events
- **When**: verify_digests is called with DigestCheck::Full and mismatched ABI
- **Then**: Returns ActionAbiMismatch
- **Coverage**: EARS-1, EARS-3

### Test 8: verify_digests_full_level_checks_policies
- **Given**: A journal with RunAdmission events
- **When**: verify_digests is called with DigestCheck::Full and mismatched policy
- **Then**: Returns PolicyDigestMismatch
- **Coverage**: EARS-2, EARS-3
