bead_id: vb-qi37.2.4
phase: 11
attempt: 1-of-7

STATUS: APPROVED

# Formal Verification Report

Required proof/deep/standard lanes executed successfully after gauntlet script repair.

- `moon run :verify-proof` passed Kani proof harnesses and reported Verus proof obligations waived because the Verus toolchain is not installed in that lane.
- `moon run :verify-deep` passed node-dedup Kani harnesses.
- `moon run :verify-standard` passed clippy/unit/Kani standard checks.
- `moon ci` passed all resolved tasks.

No required bead-local obligation remains failed.
