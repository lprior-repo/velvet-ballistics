# Assurance Bundle: vb-qi37.1

STATUS: APPROVED

## Evidence Kernel

- Proof review: `.beads/vb-qi37.1/proof-review.md` -> `STATUS: APPROVED`.
- Contract verification review: `.beads/vb-qi37.1/contract-verification-review.md` -> `STATUS: APPROVED`.
- Test plan review: `.beads/vb-qi37.1/test-plan-review.md` -> `STATUS: APPROVED`.
- Test suite review: `.beads/vb-qi37.1/test-suite-review.md` -> `STATUS: APPROVED`.
- Formal verification: `.beads/vb-qi37.1/formal-verification-report.md` -> `STATUS: APPROVED`.
- Machine gates: `.beads/vb-qi37.1/machine-gate-report.md` -> `STATUS: PASS`.
- Black-hat review: `.beads/vb-qi37.1/black-hat-review.md` -> `STATUS: APPROVED`.
- Verification ledger: `.beads/vb-qi37.1/verification-ledger.jsonl` has 31 JSONL rows covering all proof obligations.

## Raw Command Evidence

- Verus: `17 verified, 0 errors` on `verification/verus/recovery_verification.rs`.
- TLC: RecoveryHydration model completed with no error; 10740192 states generated, 8405208 distinct states, depth 7.
- Moon: `fmt`, `lint-src`, `check`, `source-length`, `test`, and `bench-build` passed.
- Tests: workspace recovery contract 19 passed; storage recovery 77 passed; runtime recovery 9 passed; recovery proptests 3 passed.

## Known Non-Blocking Tooling Issues

- `moon ci` cannot resolve Git `main` in this jj workspace.
- `moon run :verify-proof` references a malformed shell script; exact Verus/TLC proof commands passed directly.
