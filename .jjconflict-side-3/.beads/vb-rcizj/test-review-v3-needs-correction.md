**Findings**

1. **MINOR** — No test exercises `AttemptBeyondMax` for attempt=0, capacity=0, or attempt>capacity with correct ticket identity. These are contract requirements but are unreachable in normal operation because `require_pending_action_ownership` demands exact ticket match (including valid attempt/capacity) before `validate_ticket_attempt` runs. The redundant validation in `preflight_action_completion` is untested but harmless.

2. **OBSERVATION** — Contract text and production behavior differ on error precedence for forged tickets (ownership checked before attempt validation). The addendum documents this; tests match production. Acceptable.

3. **OBSERVATION** — Mutation resistance demonstrated for 5 specific guards (step, action, capacity, key, sequence), each killed by named stale_guards tests with exit 101. Not exhaustive mutation testing but provides reasonable confidence for the in-scope deletions.

**Evidence**

- 28 tests pass independently (9 durable_retry + 2 matrix + 17 lifecycle including 10 stale_guards subcases).
- All 5 production guard removals killed by tests; original code restored and full suite passes.
- Tests assert exact journal equality, pending ticket preservation, PC/slot/taint immutability on rejection.
- Happy path verified with exact envelope data (value, taint, digest, encoded_len).
- Existing StaleAttempt cases (attempt 2 vs current 3) unchanged and passing.

**Limitations**

- Mutation scope limited to 5 specific guard deletions; not exhaustive mutation or proof coverage.
- `validate_ticket_attempt` path (attempt=0, capacity=0, attempt>capacity) untested due to ownership precedence making it unreachable with valid tickets.
- No property-based or fuzz testing of ticket validation.

**STATUS: APPROVED**