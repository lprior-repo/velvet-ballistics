# Bead vb-63js3 Contract

**Status:** claimed
**Claimed by:** local3090 contract author
**Workspace:** ~/src/isoloated/vb-63js3-contract-local3090-v1

## Problem

The Flux verification lane in `xtask/src/lanes.rs` directly invokes `cargo flux -p <pkg>`, bypassing the canonical package wrapper script `scripts/flux-check-package.sh`. The wrapper provides two value-adds over the direct call:

1. **Target flag validation:** Rejects unsupported `cargo-flux` target selectors (`--lib`, `--test`, `--tests`, `--benches`, `--all-targets`) with clear error messages, per AGENTS.md and master doc section 077.
2. **Consistent message format:** Always adds `--message-format human`, which the current direct call omits.

This is a consistency and robustness fix, not a master doc breach (master permits either wrapper or direct cargo call with `--message-format human`).

## Scope

### In Scope

**Implementation:**
- Modify `xtask/src/lanes.rs` `lane_command()` for the `"flux"` lane to generate a command that invokes the existing wrapper script `scripts/flux-check-package.sh` via `bash`, passing the selected package name as the sole positional argument.
- Command shape: `["bash", "scripts/flux-check-package.sh", "<pkg>"]`
- Workspace root path handling: compute the absolute path to the script via `workspace_root.join("scripts/flux-check-package.sh")` and pass as the script argument, preserving robustness when workspace root contains spaces.

**Testing:**
- Add a new unit test in `xtask/src/lanes.rs` `mod tests` verifying the generated argv for the `"flux"` lane:
  - Assert the command contains exactly `bash`, the script path, and the package name.
  - Assert no extra flags are added.
  - Assert the package name appears as a separate argument (not shell-interpolated into a single string).
- Test with a synthetic workspace root path containing spaces to validate path handling.
- No test runner invocation; test is pure argv construction verification.

### Out of Scope

- No new `--all` / `--crates` / multi-package discovery or bulk proof execution.
- No passthrough of user-supplied `cargo-flux` flags through the lane command.
- No modification to `scripts/flux-check-package.sh` behavior.
- No new Moon task entrypoint (`moon run :flux`); deferred to separate bead.
- No full verifier proof closure or performance benchmarks.
- No changes to other lanes (verus, kani, etc.).
- No new generic types, traits, or module reorganizations.

## Constraints

### Functional

- The lane must invoke `scripts/flux-check-package.sh` for every Flux lane execution.
- Only one package name is passed; the wrapper handles single-package mode.
- Execution errors (nonzero exit, missing script, missing cargo-flux) propagate through the existing `execute_lane()` path in `proof_orchestrator.rs`; no special error handling added in this bead.

### Engineering (per AGENTS.md)

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`.
- No unchecked indexing, slicing, casts, or arithmetic.
- Existing overlong file exceptions apply (lanes.rs currently 201 lines, within limits).
- Path handling must use `std::path::Path`/`PathBuf` operations, not string concatenation.

### Verification

- New test proves exact argv for one-package Flux invocation.
- Existing lane command tests (`test_lane_command_test`, `test_lane_command_clippy`) remain unchanged.
- Existing wrapper behavior (unsupported flag rejection, `--message-format human`) is tested via the wrapper script itself, not re-tested in lanes.rs.
- Real `cargo flux` execution is not required for this contract; argv correctness is sufficient.

## Deliverables

1. Modified `xtask/src/lanes.rs` with Flux lane routing through wrapper script.
2. New unit test for Flux lane argv construction in `xtask/src/lanes.rs` `mod tests`.
3. This contract document.
4. Research document (separate file).

## Acceptance Criteria

1. `lane_command(&flux_lane, "vb_runtime", workspace_root)` returns `["bash", "<workspace_root>/scripts/flux-check-package.sh", "vb_runtime"]`.
2. No extra flags or arguments are added beyond bash, script path, and package name.
3. All existing lane command tests pass.
4. New Flux lane test passes.
5. `cargo test -p xtask` succeeds.
6. No other lanes' command generation is affected.

## Dependencies

- Existing `scripts/flux-check-package.sh` (unchanged).
- Existing `lanes.rs` infrastructure (Lane struct, lane_command API).
- `std::path::Path` for workspace root handling.

## Risk Assessment

**Low.** This is a focused routing change:
- Single lane affected (flux).
- Wrapper script is stable and well-tested.
- No API changes to callers.
- No new dependencies.

**Residual risk:** If the wrapper script is renamed or moved in a future bead, the hard-coded path in lanes.rs will need updating. This is acceptable given the wrapper is documented in AGENTS.md as the canonical entry point.