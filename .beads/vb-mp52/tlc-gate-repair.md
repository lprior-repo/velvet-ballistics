# vb-mp52 TLC Gate Repair

## Scope

Repaired `.moon/tasks/tlc.yml` so required CI TLC tasks fail closed and use the
installed TLC runner or an explicit `TLA2TOOLS_JAR` instead of assuming a
repository-local `tla2tools.jar`.

## Files changed

- `.moon/tasks/tlc.yml`

## Change summary

- Added root `verification/tla/*.tla` and `verification/tla/*.cfg` to TLC task
  inputs.
- Replaced `java -jar tla2tools.jar` with runner discovery:
  - `tlc` wrapper when available.
  - `java -jar "$TLA2TOOLS_JAR"` when an existing jar path is provided.
  - fail-closed `exit 127` when neither is available.
- Removed invalid `-death` TLC option.
- Fixed path references for:
  - `verification/tla/WorkflowBoundedAdmission.tla`
  - `verification/tla/WorkflowBoundedAdmission.cfg`
  - `verification/tla/IdempotencySafety.tla`
  - `verification/tla/IdempotencySafety.cfg`
- Added per-run `-metadir target/tlc-tmp/...-$$` directories so parallel TLC
  tasks do not collide in timestamp-named `states/` directories.
- Scoped `verify-tlc` to the fail-closed CI baseline root models. Heavier
  exploratory specs under `verification/tla/specs/` remain outside this CI task
  until they have bounded, time-safe configs.

## Tool discovery

```text
java --version
openjdk 26.0.1 2026-04-21
OpenJDK Runtime Environment (build 26.0.1+8-34)
OpenJDK 64-Bit Server VM (build 26.0.1+8-34, mixed mode, sharing)

command -v tlc
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc

command -v tla2tools
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tla2tools
```

## Evidence

Commands run from `/home/lewis/src/velvet-ballistics`:

```text
moon run velvet-ballistics:verify-tlc-workflow
PASS: TLC checked verification/tla/WorkflowBoundedAdmission.tla with no error;
2589 states generated, 1520 distinct states, depth 7.

moon run velvet-ballistics:verify-tlc-idempotency
PASS: TLC checked verification/tla/IdempotencySafety.tla with no error;
986 states generated, 306 distinct states, depth 7.

moon run velvet-ballistics:verify-tlc
PASS: TLC checked WorkflowBoundedAdmission and IdempotencySafety with no error;
WorkflowBoundedAdmission: 2589 states generated, 1520 distinct states, depth 7.
IdempotencySafety: 986 states generated, 306 distinct states, depth 7.
```

## Blocked/heavy evidence

Direct exploratory run of `verification/tla/specs/ActionRouting.tla` was not
kept in the CI baseline because it did not finish within 10 minutes with one
worker and also did not finish within 4 minutes with 8 workers. Keeping it in
`moon ci` would replace the original missing-jar failure with a long-running
global CI blocker rather than a bounded fail-closed gate.

## Residual risk

This repair proves the CI TLC runner and two root baseline models. It does not
claim full coverage for every exploratory TLA+ model under
`verification/tla/specs/`; those need separate bounded configs or bead-scoped
proof obligations before becoming CI-blocking.
