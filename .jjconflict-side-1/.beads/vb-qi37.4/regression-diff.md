# Regression Diff: vb-qi37.4

STATUS: PASS

## Changed Areas

- Proof/review/evidence artifacts under `.beads/vb-qi37.4/`.
- Verification artifacts under `specs/` and `verification/verus/` from prior State 5.
- Proof wrapper script from prior local repair.
- Loom model compile repair in two `crates/vb_runtime/src/models/loom/` files.

## Regression Classification

- No bead-local regression observed after final gates.
- Prior `moon ci` missing-Git-main failure is workspace invocation/tooling-specific and resolved by `moon ci --stdin` using `jj diff --name-only`.
