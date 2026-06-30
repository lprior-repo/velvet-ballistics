bead_id: vb-qi37.23
bead_title: quality: Full gate evidence refresh
phase: 7
updated_at: 2026-05-18T20:34:52Z
attempt: 1-of-7
# Test Plan

STATUS: PLANNED
## Behavior Inventory
- Evidence refresh rejects missing command output.
- Evidence refresh blocks release-critical failed gates.
- Evidence refresh preserves isolation from source checkout.
- Evidence refresh maps every required gate to ledger result.
## BDD Scenarios
### given_closed_blocker_when_refresh_resumes_then_state2_scope_is_written
Given vb-qi37.25 is closed; When State 2 runs; Then delivery-scope.jsonl parses and names required gates.
### given_gate_fails_when_classified_then_blocking_owner_state_is_recorded
Given a required release gate exits nonzero; When State 11 classifies; Then regression-diff.md records BLOCK_RELEASE or REQUIRED_OBLIGATION_FAIL with owner_state/rerun_from.
### given_all_gates_pass_when_evidence_packaged_then_final_decision_approved
Given every required ledger row passes or is waived; When truth-serum audits raw evidence; Then final-evidence-decision.md says STATUS: APPROVED.
## Mutation/Fuzz/Coverage checkpoints
State 11 executes existing repository gates: mutants-smoke, fuzz-smoke, coverage, Miri, sanitizer, supply-chain, benchmark build, public API, bloat, moon ci.
