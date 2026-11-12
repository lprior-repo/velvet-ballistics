bead_id: vb-2bzz
bead_title: storage: Expose action ABI and policy digest recovery mismatch checks
phase: 9
updated_at: 2026-05-17T02:00:00Z
attempt: 1-of-7

## Test Suite Review

### Test Plan Coverage

| Test | Contract Clause | Assertion Strength | Deterministic |
|---|---|---|---|
| `action_abi_mismatch_returns_typed_error` | EARS-1, INV-4 | Exact variant + exact action_id | Yes |
| `action_abi_match_returns_ok` | INV-3 | Ok() on matching digests | Yes |
| `check_action_abi_digests_empty_input_returns_ok` | EARS-3, INV-1 | Ok() on empty input | Yes |
| `policy_digest_mismatch_returns_typed_error` | EARS-2, INV-4 | Exact variant + exact step | Yes |
| `policy_digest_match_returns_ok` | INV-3 | Ok() on matching digests | Yes |
| `check_policy_digests_empty_input_returns_ok` | EARS-3, INV-2 | Ok() on empty input | Yes |

### Contract Parity

- All EARS requirements have corresponding tests ✓
- All invariants have corresponding tests ✓
- No untested contract clauses ✓

### Assertion Strength

- Tests assert exact error variants (not just `is_err()`) ✓
- Tests assert exact field values (action_id, step) ✓
- Tests assert Ok() for matching and empty cases ✓
- No hollow Ok(_) arms (removed from ignored tests) ✓

### Deterministic Execution

- All tests use constant test values (no randomness) ✓
- No external I/O in new tests (pure comparison functions) ✓
- No shared mutable state between tests ✓

### Mutation Kill Rate

- Mismatch test would fail if `!=` changed to `==` ✓
- Match test would fail if comparison removed ✓
- Empty input test would fail if empty check removed ✓
- Exact field assertion would fail if wrong field returned ✓

### Removed Hollow Tests

The two previously ignored tests (`action_abi_mismatch_returns_typed_error` and `policy_digest_mismatch_returns_typed_error`) had hollow `Ok(_) => {}` arms that accepted broken behavior. These have been replaced with executable tests that assert exact error variants.

## Verdict

STATUS: APPROVED

Test suite covers all contract clauses with exact assertions. Tests are deterministic, behavior-focused, and would catch mutations to the comparison logic. Hollow Ok(_) arms have been eliminated.
