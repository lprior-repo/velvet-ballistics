#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  echo "InvalidInvocation: run from repository root" >&2
  exit 64
fi

COMMAND_SUMMARY="bash scripts/check-panic-surface.sh"
PATTERN='(^|[^A-Za-z0-9_])(assert!|assert_eq!|assert_ne!|unreachable!)'

commit_sha() {
  if git rev-parse HEAD 2>/dev/null; then
    return 0
  fi
  if command -v jj >/dev/null 2>&1; then
    if jj log -r @ --no-graph -T 'commit_id.short(12) ++ "\n"' 2>/dev/null; then
      return 0
    fi
  fi
  printf '%s\n' "unknown"
}

echo "CWD: $ROOT"
echo "CommitSHA: $(commit_sha)"
echo "Toolchain: $(rustc --version)"
echo "Command: $COMMAND_SUMMARY"
echo "ScanDomain: crates/*/src"
echo "NonProductionPathExcluded: tests benches examples fuzz target .beads fixtures build.rs path-scoped tests.rs *_tests.rs kani harnesses loom models vb_ajc40_flux"

set +e
mapfile -t violations < <(
  rg -n "$PATTERN" crates/*/src \
    --glob '!**/workspace_tests/**' \
    --glob '!**/test_loop_inventory/**' \
    --glob '*.rs' \
    --glob '!**/tests/**' \
    --glob '!**/tests.rs' \
    --glob '!**/*_tests.rs' \
    --glob '!**/*_test*.rs' \
    --glob '!**/lifecycle_tests/**' \
    --glob '!**/kani*.rs' \
    --glob '!**/models/loom/**' \
    --glob '!**/benches/**' \
    --glob '!**/examples/**' \
    --glob '!**/proofs/**' \
    --glob '!fuzz/**' \
    --glob '!target/**' \
    --glob '!.beads/**' \
    --glob '!fixtures/**' \
    --glob '!build.rs' \
    --glob '!crates/vb_ajc40_flux/**' \
  | while IFS=: read -r file linenum rest; do
    if [[ "$file" == *"_tests.rs" ]] || [[ "$file" == *"tests.rs" ]] || [[ "$file" == *"test/"* ]]; then
      continue
    fi
    before_count=$(sed -n "1,${linenum}p" "$file" | grep -cE '^\s*#\[(cfg\(test\)|test)\]' || true)
    after_count=$(sed -n "1,${linenum}p" "$file" | grep -cE '^\s*#\[kani::proof\]' || true)
    if [[ "$before_count" -gt 0 ]] || [[ "$after_count" -gt 0 ]]; then
      continue
    fi
    if [[ "$rest" =~ ^[[:space:]]*// ]]; then
      continue
    fi
    echo "${file}:${linenum}:${rest}"
  done
)
status="${PIPESTATUS[0]}"
set -e

if [[ ${#violations[@]} -gt 0 ]] || [[ -n "${violations[*]}" ]]; then
  printf '%s\n' "${violations[@]}"
  echo "ViolationFound: production panic/assert macro surface is non-empty" >&2
  echo "ExitCode: 2"
  exit 2
fi

echo "NoViolationFound"
echo "ExitCode: 0"
exit 0
