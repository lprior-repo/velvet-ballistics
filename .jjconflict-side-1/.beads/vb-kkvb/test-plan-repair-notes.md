# Test Plan Repair Notes: vb-kkvb

Repair scope: `.beads/vb-kkvb/test-plan.md` only; no implementation or test code.

## Blocker Mapping

- Unit-test density: added a deletion-resistant catalog of 40 named unit tests (U01-U40), exceeding the minimum 35, each with concrete expected values or exact error variants.
- `route_command` proptest coverage: added explicit `route_command` determinism and accepted/rejected command-domain proptest invariants.
- `XtaskCommandError::Unavailable`: added BDD Behavior 30 and unit test U34 asserting exact `Err(Unavailable { command: "perf", reason: "perf automation is not implemented in bead vb-kkvb" })`.
- Non-concrete assertions: fixed renderer format to JSON Lines; specified exact JSON output, exit code `2` for CLI validation failures, exact structured status values, exact `InvalidInput` reasons, exact `InternalInvariantViolation` strings, and exact dependency-boundary errors.
