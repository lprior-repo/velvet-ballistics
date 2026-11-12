bead_id: vb-apn5
bead_title: "storage/runtime: Single-server database lock enforcement"
phase: 12
updated_at: 2026-05-09T00:00:00Z

# Formal Verification Report

## Proof Obligations

| ID | Layer | Status |
|---|---|---|
| PO1-first-open | unit-test | PASS |
| PO2-second-open-fails | unit-test | PASS |
| PO4-lock-release | unit-test | PASS |
| PO5-lock-before-keyspaces | code-review | PASS |
| I1-at-most-one-holder | unit-test | PASS |
| I3-auto-release | unit-test | PASS |
| I4-no-keyspace-on-lock-fail | unit-test | PASS |

## Waivers
- Kani/Lean/Miri/fuzz/loom: Filesystem I/O and process locking cannot be formally verified at compile time.
- Compensating evidence: 2090 integration tests + security tests.

STATUS: APPROVED
