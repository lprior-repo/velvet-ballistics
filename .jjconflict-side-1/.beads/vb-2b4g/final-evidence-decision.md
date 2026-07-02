# Final Evidence Decision - vb-2b4g

## Status

STATUS: APPROVED_FOR_SCOPED_LANDING_WITH_RESIDUAL_RISKS

## Decision

`vb-2b4g` satisfies the bead-local evidence contract for generated Rust parity of `Repeat*`, `Reduce*`, `Together*`, and `Collect*` against `vb_runtime::engine::drive::drive_deterministic_full`.

The bead is approved for scoped landing consideration because the active-context truth-serum audit reproduced the required scoped gates and found no remaining bead-local blocker.

## Approved Claims

- Scoped executable runtime parity for repeat, reduce, together, and collect generated paths.
- Journal-signature parity at the normalized semantic-observation level used by the test harness.
- Generated-source contract checks for forbidden placeholders and unsupported fail-open behavior.
- Touched-crate compile, trybuild, fmt, full local package test, and strict production clippy confidence for `vb_codegen`.
- Honest residual-risk accounting for global, formal, mutation, performance, and harness-design gaps.

## Rejected Claims

- No global `moon ci` pass claim.
- No final release-confidence claim.
- No formal proof, theorem proof, Kani, Verus, TLA+, Lean, Aeneas, Hax, mutation, fuzzing, or performance claim.
- No claim that synthesized `RunFinished` helper evidence is equivalent to native runtime terminal-event emission.

## Current Direct Evidence

- `.beads/vb-2b4g/truth-serum-report.md`: active-context audit evidence and verdict.
- `.beads/vb-2b4g/assurance-bundle.md`: packaged requirement-to-evidence map and residual risk table.
- `.beads/vb-2b4g/machine-gate-report.md`: scoped gate command results and `moon ci` global-debt classification.
- `.beads/vb-2b4g/regression-diff.md`: local/global failure classification.
- `.beads/vb-2b4g/proof-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/test-plan-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/test-suite-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/contract-verification-review.md`: `STATUS: APPROVED`.
- `.beads/vb-2b4g/formal-verification-report.md`: `STATUS: APPROVED` with formal lanes waived/not in scope.
- `.beads/vb-2b4g/black-hat-review.md`: `STATUS: APPROVED`.

## Required Follow-Up Before Release Confidence

- Rerun `moon ci` after disk quota/resource cleanup and record the result.
- Do not convert formal waivers or mutation gaps into pass claims without new reviewed evidence.
- Track any future native runtime terminal-event exposure separately from this bead.

## Final Disposition

Proceed to landing-skill / bead closure flow for scoped delivery. If the landing policy requires a fresh global `moon ci` pass, classify this bead as ready-but-globally-blocked by environment quota until that gate can run successfully.
