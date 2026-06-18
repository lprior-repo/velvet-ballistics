#!/usr/bin/env bash
set -uo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  printf '%s\n' "InvalidInvocation: run from repository root" >&2
  exit 64
fi

# Static stderr contract witness for State 13 grep evidence.
: <<'SUMMARY_FORMAT'
summary: active=<N> allowlisted=<M> files_scanned=<K> hot_paths=<H> cold_paths=<C>
SUMMARY_FORMAT

if [[ -n "${FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE:-}" ]]; then
  printf 'GateError:ScriptInvocationFailure: %s\n' \
    "$FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE" >&2
  exit 2
fi

if ! command -v timeout >/dev/null 2>&1; then
  printf '%s\n' "GateError:ScriptInvocationFailure: timeout command missing" >&2
  exit 2
fi

mkdir -p target/gate-tools target/tmp || {
  printf '%s\n' "GateError:ScriptInvocationFailure: could not create target/gate-tools" >&2
  exit 2
}

BIN="target/gate-tools/forbid-runtime-fmt"
LOCK="target/gate-tools/forbid-runtime-fmt.lock"
tmp_bin="$(mktemp target/gate-tools/forbid-runtime-fmt.bin.XXXXXX)"
compile_output="$(timeout 30s flock "$LOCK" rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o "$tmp_bin" 2>&1)"
compile_status=$?
if [[ "$compile_status" -ne 0 ]]; then
  rm -f "$tmp_bin"
  if [[ "$compile_status" -eq 124 ]]; then
    first_line="rustc timed out after 30s"
  else
    first_line="${compile_output%%$'\n'*}"
  fi
  if [[ -z "$first_line" ]]; then
    first_line="rustc failed"
  fi
  printf 'GateError:ScriptInvocationFailure: %s\n' "$first_line" >&2
  exit 2
fi
if ! mv -f "$tmp_bin" "$BIN"; then
  rm -f "$tmp_bin"
  printf '%s\n' "GateError:ScriptInvocationFailure: could not install scanner binary" >&2
  exit 2
fi

stdout_file="$(mktemp target/tmp/forbid-runtime-fmt.stdout.XXXXXX)"
stderr_file="$(mktemp target/tmp/forbid-runtime-fmt.stderr.XXXXXX)"
cleanup() {
  rm -f "$stdout_file" "$stderr_file"
}
trap cleanup EXIT

timeout 30s "$BIN" "$@" >"$stdout_file" 2>"$stderr_file"
run_status=$?
if [[ "$run_status" -eq 124 ]]; then
  printf '%s\n' "GateError:ScriptInvocationFailure: scanner timed out after 30s" >&2
  exit 2
fi

if [[ -s "$stderr_file" ]]; then
  sort -u "$stderr_file" >&2
fi
if [[ -s "$stdout_file" ]]; then
  cat "$stdout_file"
fi

exit "$run_status"
