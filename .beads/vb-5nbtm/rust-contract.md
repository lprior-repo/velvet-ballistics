# Rust Contract: vb-5nbtm - Final Fixture and Independent Probe Merge

## Purpose
Merge three frozen candidates into the final deliverable, resolve conflicts by preserving both the NewActionTicket parameter struct API and the fixture migration's run_attempt field semantics, repair two unchecked subtractions in the independent review test suite, and validate with focused test suites.

## Input Candidates
- af142d34: Combined ticket generation, replay accessor, typed ticket, dispatch params, B/C/D retry safety, compiler repair
- a8452f9e: Completed fixture migration (workspace test compilation + core/storage/runtime library tests pass)
- b5baccbd: Independent B/C/D review with nine adversarial tests

## Conflict Resolution Rules
- Preserve NewActionTicket parameter struct API (issue_action_ticket takes NewActionTicket spec)
- Preserve fixture migration's run_attempt=1 for initial run fixtures
- Preserve ticket.run_attempt in copies
- Preserve arbitrary values in symbolic generators
- Preserve every prior fixture value, generated input, and assertion
- Test assertions are ground truth; resolve conflicts to match assertions

## Conflicts to Resolve
Three conflicts in crates/vb_core/src/action/tests.rs:
1. issue_action_ticket_captures_all_fields: run_attempt=2, attempt=3 (assertions require these values)
2. issue_action_ticket_with_zero_key_is_valid: run_attempt=1, attempt=1 (both versions agree)
3. issue_action_ticket_with_max_values: all u16::MAX/u128::MAX (both versions agree)

Resolution: Use NewActionTicket struct API with struct-side values for all three (they match assertions).

## Test Repairs
Two unchecked subtractions in crates/vb_runtime/tests/vb_4qog3_independent_review.rs:
- Line ~468: prior_seq - 1 must use checked_sub with fallible error propagation
- Line ~588: same pattern
- Preserve exact tamper values and assertions
- No unwrap, expect, panic, unchecked arithmetic, lint allowances, or weakened oracles

## Verification Suites
1. Workspace test compilation
2. Core library suite: vb_core
3. Storage library suite: vb_storage
4. Runtime library suite: vb_runtime
5. Focused suite: durable_retry_run_attempt_authority
6. Focused suite: vb_r4fm9_event_generation
7. Focused suite: durable_retry_boundary
8. Focused suite: vb_4qog3_independent_review
9. Focused suite: durable_retry_execution
10. Focused suite: durable_retry_cli
11. Focused suite: runtime_fjall_pending_action_recovery

## Acceptance Criteria
- Zero conflicts remaining
- All three NewActionTicket conflicts resolved with struct API
- Two unchecked subtractions repaired with checked_sub
- All verification suites pass
- Full stdout/stderr and true exit JSON captured via /tmp/vb-run-gate.py runner
- No broad moon ci (separate lane handles Flux syntax and safe indexing)

## Deliverables
- Resolved conflicts in crates/vb_core/src/action/tests.rs
- Repaired subtractions in crates/vb_runtime/tests/vb_4qog3_independent_review.rs
- Verification evidence in .beads/vb-5nbtm/evidence/
- Handoff document in .beads/vb-5nbtm/handoff.md
- Bead notes updated, bead left OPEN
