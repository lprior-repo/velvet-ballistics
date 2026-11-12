# Regression Diff: vb-qi37.1

STATUS: PASS

## Scope

- Production source changed in this continuation: none.
- Verification/model artifacts changed in isolated workspace: `verification/verus/recovery_verification.rs`, `verification/tla/RecoveryHydration.tla`, `verification/tla/RecoveryHydration.cfg`.
- Bead artifacts changed under `.beads/vb-qi37.1/`.

## Regression Evidence

- `moon run :test` passed all 8358 executed tests.
- `moon run :lint-src`, `moon run :check`, `moon run :fmt`, `moon run :source-length`, and `moon run :bench-build` passed.
- No regression failure was observed in scoped recovery tests or formal proof reruns.
