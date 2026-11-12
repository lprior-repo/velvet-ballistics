# Bead vb-63js3 Contract

## Scope
Add a private helper `flux_command` in `xtask/src/lanes.rs` that constructs the argv vector for invoking the repository's `scripts/flux-check-package.sh` wrapper script, and update the `flux` match arm in `lane_command` to call it.

## Actual Current State
- `xtask/src/lanes.rs` is 201 lines, defines `Lane`, `lane_command`, `detect_available_lanes`, and private helpers.
- The current `flux` match arm constructs `vec!["cargo", "flux", "-p", crate_name]` directly; it does not use the repository wrapper script.
- The caller is `execute_lane` in `xtask/src/proof_orchestrator.rs`, which calls `lanes::lane_command(lane, crate_name, workspace_root)` and receives `Vec<String>`.
- `scripts/flux-check-package.sh` is a thin bash wrapper that validates args and runs `cargo flux -p "$package" --message-format human "$@"`.

## What Changes
- Add `fn flux_command(crate_name: &str, workspace_root: &Path) -> Vec<String>` (under 25 logical lines).
- Change the `flux` match arm to `flux_command(crate_name, workspace_root)`.
- All other lanes and public API unchanged.

## Tests
- Add tests in `xtask/src/lanes.rs` test module:
  - Exact full `Vec<String>` comparison for `flux` with `/workspace` root.
  - Workspace root containing spaces.
  - Workspace root containing shell metacharacters (`$HOME`, `(`, `)`, `&`).
- Existing tests unchanged.

## Constraints
- No shell execution in tests; tests compare constructed `Vec<String>` to expected vector.
- No `unwrap`, `expect`, `panic`, `unsafe`, `todo`, `dbg`, unchecked arithmetic, or unchecked casts.
- No new traits or dependencies.
- File stays at or below 300 physical lines.
- No real Flux proof claim; this is pure argv construction.

## Verification
- `moon run :nightly-feature-gate` (first-party feature-scope check).
- `moon run :proofs flux` (proof lane runs the actual wrapper).
- Cargo clippy for `xtask`.
- `cargo test -p xtask --lib -- lanes` (lane command tests).

## Risks
- Low: change is contained to one match arm and adds a private helper. No behavioral change to other lanes. The helper constructs the same effective command as the script would produce, just via the script rather than directly.
