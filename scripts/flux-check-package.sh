#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Canonical Flux Check Script
#
# Single entry point for all flux verification across:
#   - xtask lane runner (scripts/flux-check-package.sh <package>)
#   - Moon tasks (bash scripts/flux-check-package.sh <package> [--all | --crates ...])
#   - Direct invocation (bash scripts/flux-check-package.sh --help | --dry-run ...)
#
# Usage:
#   bash scripts/flux-check-package.sh <package> [cargo-flux options]
#   bash scripts/flux-check-package.sh --all [cargo-flux options]
#   bash scripts/flux-check-package.sh --crates <p1> <p2> ... [cargo-flux options]
#   bash scripts/flux-check-package.sh --dry-run --all
#   bash scripts/flux-check-package.sh --help
#
# The installed cargo-flux does not accept --lib, --test, --tests, --benches,
# or --all-targets target selectors; these are rejected at the script level.
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

usage() {
  cat >&2 <<EOF
usage: bash scripts/flux-check-package.sh [OPTIONS] [PACKAGE]

Canonical flux-check entry point.

Options:
  --all                    Run flux on all crates with flux annotations
  --crates P1 [P2 ...]    Run flux on specific packages
  --dry-run               Print commands without executing
  --help                  Show this help message

When --all or --crates is omitted, the first positional argument is treated
as the package name. Additional positional arguments are forwarded to
cargo-flux.
EOF
  exit 2
}

# --- Argument parsing ---
dry_run=false
run_all=false
declare -a extra_packages=()
declare -a flux_args=()

while [ "$#" -gt 0 ]; do
  case "$1" in
    --help)
      usage
      ;;
    --dry-run)
      dry_run=true
      shift
      ;;
    --all)
      run_all=true
      shift
      ;;
    --crates)
      shift
      if [ "$#" -lt 1 ]; then
        printf 'error: --crates requires at least one package name\n' >&2
        exit 2
      fi
      while [ "$#" -gt 0 ]; do
        case "$1" in
          --dry-run|--all|--crates|--help)
            break
            ;;
          *)
            extra_packages+=("$1")
            shift
            ;;
        esac
      done
      ;;
    *)
      flux_args+=("$1")
      shift
      ;;
  esac
done

# --- Target-selector validation ---
# The installed cargo-flux does not accept these flags.
reject_target_selector() {
  printf 'unsupported cargo-flux target selector: %s\n' "$1" >&2
  exit 2
}

for arg in "${flux_args[@]+"${flux_args[@]}"}"; do
  case "$arg" in
    --lib|--test|--tests|--benches|--all-targets)
      reject_target_selector "$arg"
      ;;
  esac
done

# --- Discover packages ---
discover_all_flux_crates() {
  # Find crates with #[cfg(flux)] gated flux source files.
  # This avoids running flux on every crate when only a subset has annotations.
  local found=()
  for manifest in "$WORKSPACE_ROOT"/crates/*/Cargo.toml; do
    local crate_name
    crate_name="$(basename "$(dirname "$manifest")")"
    # Check if the crate has flux-annotated source files
    local crate_src="$WORKSPACE_ROOT/crates/$crate_name/src"
    if [ -d "$crate_src" ] && grep -rl '#\[cfg(flux)\]' "$crate_src" >/dev/null 2>&1; then
      found+=("$crate_name")
    fi
  done
  printf '%s\n' "${found[@]+"${found[@]}"}"
}

if $run_all; then
  declare -a packages=()
  while IFS= read -r pkg; do
    [ -n "$pkg" ] && packages+=("$pkg")
  done < <(discover_all_flux_crates)
elif [ "${#extra_packages[@]}" -gt 0 ]; then
  packages=("${extra_packages[@]}")
else
  # Single package mode: first non-flag arg (if any) is the package name.
  # Remaining positional args after the first are forwarded to cargo-flux.
  if [ "${#flux_args[@]}" -gt 0 ]; then
    packages=("${flux_args[0]}")
    flux_args=("${flux_args[@]:1}")
  else
    printf 'error: no package specified. Use <package>, --all, or --crates <p1> <p2> ...\n' >&2
    exit 2
  fi
fi

# --- Execute ---
if [ "${#packages[@]}" -eq 0 ]; then
  printf 'no flux-annotated crates found in %s/crates/\n' "$WORKSPACE_ROOT" >&2
  exit 2
fi

any_failure=false

for pkg in "${packages[@]}"; do
  if $dry_run; then
    printf 'DRY-RUN: cargo flux -p %s --message-format human %s\n' "$pkg" "${flux_args[*]+"${flux_args[*]}"}"
  else
    printf 'flux-check: %s ... ' "$pkg" >&2
    if cargo flux -p "$pkg" --message-format human "${flux_args[@]+"${flux_args[@]}"}"; then
      printf 'pass\n' >&2
    else
      printf 'fail\n' >&2
      any_failure=true
    fi
  fi
done

if $any_failure; then
  exit 1
fi
