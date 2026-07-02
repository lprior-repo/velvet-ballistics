bead_id: vb-qi37.23
bead_title: quality: Full gate evidence refresh
phase: 3
updated_at: 2026-05-18T20:33:58Z
attempt: 1-of-7
# Contract

STATUS: SPECIFIED
## Requirements

- REQ-001: The evidence refresh shall run from an isolated workspace, never the source checkout.
- REQ-002: The bundle shall record exact command, commit, timestamp, exit status, and artifact path for each required current-scope gate.
- REQ-003: Mandatory Backend / IR Interpreter DoD gates shall include moon ci, verify-standard, fuzz-smoke, Miri, coverage, mutants-smoke, sanitizer, supply-chain, benchmark build, feature powerset, docs, source/workspace assertion gates, public API, and bloat evidence.
- REQ-004: Any failed mandatory gate shall be classified BLOCK_LOCAL, BLOCK_REGRESSION, BLOCK_RELEASE, REQUIRED_OBLIGATION_FAIL, WAIVED, or DEFERRED_GLOBAL with owner_state and rerun_from.
- REQ-005: No represented/placeholder task or conversational summary shall count as evidence.
## Preconditions

- PRE-001: vb-qi37.25 is closed and pushed.
- PRE-002: Current workspace path is outside /home/lewis/src/velvet-ballistics.
## Postconditions

- POST-001: machine-gate-report.md and layer reports cite raw command evidence for every gate.
- POST-002: verification-ledger.jsonl accounts for every proof obligation.
- POST-003: final evidence decision approves only when all blocking gates pass or have explicit approved waiver.
## Invariants

- INV-001: Evidence artifacts remain under .beads/vb-qi37.23 in the isolated workspace.
- INV-002: Release-critical failures block landing.
## Non-goals

- No production code, test, proof, dependency, or CI task changes are expected.
