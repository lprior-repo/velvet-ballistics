#!/usr/bin/env bash
set -euo pipefail
# check-moon.sh — fail-closed validator for the `moon` executable.
#
# Background (vb-rdpft): `~/.cargo/bin/moon` is a crates.io placeholder
# (`cargo install moon`, v0.0.0) that prints `Hello, world!` and exits 0 for
# ANY argv. Whenever PATH places `~/.cargo/bin` ahead of the mise shims, a
# bare `moon ci` / `moon run ...` gate "passes" in milliseconds having run
# nothing. The mise shim itself is sound (resolves to npm:@moonrepo/cli).
#
# Usage:
#   MOON="$(scripts/check-moon.sh)" && "$MOON" ci ...
#   MOON_BIN=/home/lewis/.moon/bin/moon scripts/check-moon.sh
#
# Contract:
#   - Validates the PATH-selected `moon`, or `$MOON_BIN` when set.
#   - Probe: `<candidate> --version` must exit 0 AND its first stdout line
#     must match `^moon [0-9]+\.[0-9]+` (real moonrepo CLI identity).
#   - Prints the validated absolute path on stdout; diagnostics on stderr.
#   - Exit 0 iff validated; exit 1 otherwise (fail closed). Never silently
#     substitutes a different binary: if PATH selects the impostor, this
#     exits 1 with a remediation hint instead of picking another moon.

candidate="${MOON_BIN:-moon}"

resolve_candidate() {
  local c="$1" resolved
  if [[ "$c" == *"/"* ]]; then
    resolved="$c"
  else
    if ! resolved="$(command -v -- "$c" 2>/dev/null)"; then
      printf 'check-moon: no `%s` found on PATH\n' "$c" >&2
      return 1
    fi
    resolved="${resolved%%$'\n'*}"
  fi
  if [[ ! -f "$resolved" || ! -x "$resolved" ]]; then
    printf 'check-moon: `%s` resolves to `%s`, not an executable file (function, alias, or missing)\n' "$c" "$resolved" >&2
    return 1
  fi
  printf '%s\n' "$resolved"
}

main() {
  local resolved version
  resolved="$(resolve_candidate "$candidate")" || return 1
  if ! version="$("$resolved" --version 2>/dev/null)"; then
    printf 'check-moon: `%s --version` exited nonzero; refusing to trust it\n' "$resolved" >&2
    return 1
  fi
  version="${version%%$'\n'*}"
  if [[ ! "$version" =~ ^moon\ [0-9]+\.[0-9]+ ]]; then
    printf 'check-moon: `%s --version` reports `%s`, not the moonrepo CLI (suspected hello-world impostor from `cargo install moon`)\n' "$resolved" "$version" >&2
    printf 'check-moon: remediation: put mise shims ahead of ~/.cargo/bin on PATH, or set MOON_BIN to the validated moon (e.g. /home/lewis/.moon/bin/moon)\n' >&2
    return 1
  fi
  printf 'check-moon: validated `%s` (%s)\n' "$resolved" "$version" >&2
  printf '%s\n' "$resolved"
}

main "$@"
