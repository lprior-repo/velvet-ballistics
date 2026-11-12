# Research: Flux Command Construction

## What exists now
- `lanes.rs` has a `lane_command` function with match arms for 13 proof/test lanes.
- The `flux` arm constructs: `vec!["cargo", "flux", "-p", crate_name]`.
- This bypasses the repository's `scripts/flux-check-package.sh` wrapper.

## What the script provides
- `scripts/flux-check-package.sh` validates arguments, rejects unsupported target selectors (`--lib`, `--test`, etc.), and runs `cargo flux -p "$package" --message-format human`.
- Using the script as the canonical entrypoint gives consistent behavior across CI and manual use.

## Caller
- `execute_lane` in `proof_orchestrator.rs` calls `lanes::lane_command(lane, crate_name, workspace_root)` and receives `Vec<String>`.
- `workspace_root` is already passed to `lane_command`; the helper can use it to construct the absolute script path.
- The caller does not need to change.

## Test approach
- Tests construct expected `Vec<String>` values and compare with `lane_command` output.
- No shell execution; tests verify argv segregation (correct quoting for paths with spaces/metacharacters).
- This demonstrates that the argv is correctly structured for `std::process::Command` to execute safely.
