# Flux Own-Patch Audit v8

**Auditor:** local5090 independent preservation auditor  
**Workspace:** vb-fyktj-flux-own-audit-v8  
**Historical commit:** 56d14ca7878d011d1b12a5c12f230110ca41186a  
**Current commit:** 0210e36fc8961f7d4454eb4ca53b6a39948a28b8  
**Date:** 2026-10-22

## Scope

Audit of 3 historical own-hunks from commit 56d14ca78, which enhanced `scripts/flux-check-package.sh` as the unified entry point for all Flux verification, added Moon tasks, and routed the xtask flux lane through the canonical script.

## Findings

### Hunk 1: New Moon tasks file `.moon/tasks/flux.yml`

**Historical patch:** Added new file with `flux-check` and `flux-check-all` Moon tasks that invoke `scripts/flux-check-package.sh`.

**Current state:** File NOT FOUND at `.moon/tasks/flux.yml`.

**Determination:** GENUINELY MISSING. The Moon tasks for Flux were added in the historical patch and are not present in the current workspace. Without this file, `moon run :flux-check` and `moon run :flux-check-all` would fail.

### Hunk 2: Rewrite of `scripts/flux-check-package.sh` (21→164 lines)

**Historical patch:** Replaced a 21-line minimal version with a 164-line canonical script featuring:
- `--all` flag to discover and run flux on all crates with `#[cfg(flux)]` annotations
- `--crates P1 [P2 ...]` flag for specific package selection
- `--dry-run` flag for command preview
- `--help` usage display
- Package discovery via `grep -rl '#\[cfg(flux)\]'` across crate source directories
- Target-selector validation rejecting `--lib`, `--test`, `--tests`, `--benches`, `--all-targets`
- Proper argument parsing with flag vs positional separation

**Current state:** File is the OLD 21-line minimal version:
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

**Determination:** GENUINELY MISSING. The current script is identical to the pre-patch version. None of the enhanced features (--all, --crates, --dry-run, --help, discovery) are present.

### Hunk 3: Change `xtask/src/lanes.rs` flux lane to use canonical script

**Historical patch:** Replaced the inline flux command:
```rust
"flux" => vec![
    "cargo".into(),
    "flux".into(),
    "-p".into(),
    crate_name.into(),
],
```
With routing through the canonical script via a new `flux_command()` function.

**Current state:** The flux lane is STILL the OLD inline command:
```rust
"flux" => vec![
    "cargo".into(),
    "flux".into(),
    "-p".into(),
    crate_name.into(),
],
```
The `flux_command()` function does not exist. The `lanes.rs` file is 145 lines with no `flux_command()` helper.

**Determination:** GENUINELY MISSING. The xtask flux lane bypasses the canonical script and runs `cargo flux` directly.

## Overall Assessment

All 3 historical own-hunks are GENUINELY MISSING from the current workspace. The current commit (0210e36fc) appears to be on a branch that predates or is independent of the historical patch (56d14ca78). The changes were never applied, not superseded or invalidated.

**Evidence supporting genuine absence:**
- `scripts/flux-check-package.sh` is byte-for-byte the pre-patch 21-line version
- `xtask/src/lanes.rs` flux lane is the pre-patch inline command
- `.moon/tasks/flux.yml` does not exist
- No trace of `flux_command()` or the enhanced script anywhere in the codebase

## Consequence

Without these 3 hunks, the Flux tooling baseline described in the master contract (section 76.9) is not fully satisfied:
- `bash scripts/flux-check-package.sh --all` does not work (no --all flag)
- `bash scripts/flux-check-package.sh --crates` does not work (no --crates flag)
- `bash scripts/flux-check-package.sh --dry-run` does not work (no --dry-run flag)
- `moon run :flux-check` and `moon run :flux-check-all` fail (file missing)
- The xtask flux lane runs `cargo flux` directly rather than through the canonical script

## Proposed Child Bead Contract

A child bead should apply the 3 missing hunks to reach the current Flux tooling baseline:

**Contract:** Route all Flux verification through `scripts/flux-check-package.sh` as the canonical entry point.

**Acceptance criteria:**
1. `scripts/flux-check-package.sh` accepts `--all`, `--crates`, `--dry-run`, `--help` flags and single-package invocation
2. `scripts/flux-check-package.sh` rejects `--lib`, `--test`, `--tests`, `--benches`, `--all-targets` target selectors
3. `xtask/src/lanes.rs` flux lane uses `flux_command()` that invokes the canonical script
4. `.moon/tasks/flux.yml` defines `flux-check` and `flux-check-all` Moon tasks
5. The `discover_all_flux_crates()` function finds crates with `#[cfg(flux)]` annotations

**Scope:** Bounded to the 3 files from the historical patch. No other source changes.
