# Bead vb-63js3 Research

**Claimed by:** local3090 contract author
**Date:** 2026-10-22

## Current Implementation Analysis

### `xtask/src/lanes.rs` — Flux Lane Command (lines 76-82)

```rust
"flux" => vec![
    "cargo".into(),
    "flux".into(),
    "-p".into(),
    crate_name.into(),
],
```

**Observations:**
- Direct `cargo flux` invocation.
- No `--message-format human` flag.
- No wrapper script invocation.
- No target flag validation.

### `scripts/flux-check-package.sh` — Existing Wrapper

```bash
#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  printf 'usage: bash scripts/flux-check-package.sh <package> [cargo-flux options]\n' >&2
  exit 2
fi

package="$1"
shift

for arg in "$@"; do
  case "$arg" in
    --lib|--test|--tests|--benches|--all-targets)
      printf 'unsupported cargo-flux target selector for installed cargo-flux: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

cargo flux -p "$package" --message-format human "$@"
```

**Wrapper value-adds:**
1. Rejects unsupported target flags with clear error.
2. Always adds `--message-format human`.
3. Provides usage message when no package specified.
4. Documents canonical invocation in AGENTS.md and master doc section 077.

## Historical Deltas (3 remaining from original scope)

The original bead scope identified several deltas; this child bead addresses the routing gap. Three deltas remain for future beads or documentation:

### Delta 1: Missing `--message-format human` in direct call
- **Current:** `cargo flux -p <pkg>` (no format flag)
- **Wrapper provides:** `--message-format human`
- **Resolution:** This child bead resolves by routing through wrapper.

### Delta 2: No target flag validation in lane command
- **Current:** Lane passes whatever flags it constructs; no validation.
- **Wrapper provides:** Rejects `--lib`, `--test`, `--tests`, `--benches`, `--all-targets`.
- **Resolution:** This child bead resolves by routing through wrapper.

### Delta 3: No test coverage for Flux lane command
- **Current:** `lanes.rs` tests cover `test` and `clippy` lanes only.
- **Resolution:** This child bead adds Flux lane argv test.

## Caller Analysis

### `xtask/src/proof_orchestrator.rs` — `execute_lane()`

```rust
fn execute_lane(...) -> anyhow::Result<LaneResult> {
    let command = lanes::lane_command(lane, crate_name, workspace_root);
    let cmd_str = command.join(" ");
    
    // ... dry-run handling ...
    
    let status = std::process::Command::new(command.first().map(String::as_str).unwrap_or("echo"))
        .args(command.iter().skip(1))
        .current_dir(workspace_root)
        .output()
        .map(|o| if o.status.success() { "pass" } else { "fail" })
        .unwrap_or("fail");
    
    // ... logging ...
}
```

**Key points:**
- `lane_command()` returns `Vec<String>`.
- First element is the executable (`"bash"` in our case).
- Remaining elements are passed as arguments via `.args()`.
- Working directory is set to `workspace_root`.
- Exit code determines pass/fail.
- No special error handling; nonzero exit = "fail" status.

**Implication:** Our wrapper routing will work correctly with this invocation pattern. The command `["bash", "<path>/scripts/flux-check-package.sh", "vb_runtime"]` will be executed as:
```bash
bash /workspace/scripts/flux-check-package.sh vb_runtime
```

## AGENTS.md and Master Doc Context

### AGENTS.md (loaded)
- Explicitly documents `bash scripts/flux-check-package.sh <package>` as the canonical Flux check command.
- Notes that installed `cargo-flux` does not accept `--lib` or `--test`.
- States package smoke pass is not proof unless refinement artifact is wired in.

### Master doc section 077 (found via grep)
- Permits either `bash scripts/flux-check-package.sh <package>` OR `cargo flux -p <package> --message-format human`.
- Lists unsupported target flags: `--lib`, `--test`, `--tests`, `--benches`, `--all-targets`.
- Clarifies package smoke vs proof distinction.

**Conclusion:** Routing through the wrapper is consistent with both AGENTS.md and master doc. The absence of wrapper usage in `lanes.rs` is not a master doc breach, but routing through the wrapper improves consistency and robustness.

## Test Strategy

### New Unit Test: `test_lane_command_flux`

```rust
#[test]
fn test_lane_command_flux() {
    let lane = Lane {
        name: "flux".to_string(),
        required: false,
    };
    let cmd = lane_command(&lane, "vb_runtime", Path::new("/workspace"));
    assert_eq!(cmd.len(), 3);
    assert_eq!(cmd[0], "bash");
    assert!(cmd[1].ends_with("scripts/flux-check-package.sh"));
    assert_eq!(cmd[2], "vb_runtime");
}
```

**What this proves:**
- Exact argv for one-package invocation.
- No extra flags added.
- Package name is separate argument.
- Bash is used as the executor.

**What this does NOT prove:**
- Actual `cargo flux` execution (requires installed tool).
- Wrapper script behavior (tested by wrapper itself).
- Target flag rejection (tested by wrapper script).

### Path with Spaces Test

```rust
#[test]
fn test_lane_command_flux_path_with_spaces() {
    let lane = Lane {
        name: "flux".to_string(),
        required: false,
    };
    let cmd = lane_command(&lane, "vb_runtime", Path::new("/path with spaces/workspace"));
    assert_eq!(cmd.len(), 3);
    assert_eq!(cmd[0], "bash");
    assert!(cmd[1].contains("path with spaces"));
    assert!(cmd[1].ends_with("scripts/flux-check-package.sh"));
    assert_eq!(cmd[2], "vb_runtime");
}
```

**What this proves:**
- Path handling works with spaces.
- No shell interpolation (path is single argument to `bash`).

## Implementation Notes

### Path Construction

Use `workspace_root.join("scripts").join("flux-check-package.sh")` and convert to String via `.display()` or `.to_string_lossy()`. Given AGENTS.md's no-`unwrap` rule, use `.to_string_lossy().into_owned()` or handle the PathBuf directly.

**Recommended approach:**
```rust
let script_path = workspace_root
    .join("scripts")
    .join("flux-check-package.sh")
    .to_string_lossy()
    .into_owned();
```

This avoids `unwrap` and handles non-UTF-8 paths gracefully (rare in practice).

### No API Changes

The `lane_command()` signature remains unchanged:
```rust
pub fn lane_command(lane: &Lane, crate_name: &str, workspace_root: &Path) -> Vec<String>
```

Callers (`proof_orchestrator.rs`) require no modification.

## Verification Plan

1. Run `cargo test -p xtask` to verify all lane command tests pass.
2. Verify no other lanes' command generation is affected by inspecting test output.
3. Optionally dry-run the proof orchestrator with the flux lane to confirm command generation (no actual execution required).

## Remaining Work Items (Not in this bead)

1. **Moon task entrypoint:** `moon run :flux` or similar. Requires separate evaluation and bead.
2. **Multi-package Flux execution:** Not in scope for this lane-based design.
3. **Flag passthrough:** If users want to pass additional `cargo-flux` options, this requires API changes beyond this bead.
4. **Full proof closure:** Actual Flux verification of production code is a separate concern from lane command generation.

## Conclusion

This child bead has a clear, bounded scope: route the Flux lane through the existing canonical wrapper script via typed Vec argv construction, preserving the current API and adding appropriate test coverage. The implementation is low-risk and addresses a consistency gap documented in AGENTS.md and the master doc.