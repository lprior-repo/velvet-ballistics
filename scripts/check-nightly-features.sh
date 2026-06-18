#!/usr/bin/env bash
set -euo pipefail

normal_allowed='^(try_blocks|portable_simd)$'
perf_only_allowed='^(allocator_api|generic_const_exprs)$'
perf_marker='velvet-allow-perf-nightly-feature'
status=0

is_perf_scoped_path() {
  local file="$1"

  [[ "$file" == crates/*/src/perf/* ]] && return 0
  [[ "$file" == crates/*/src/generated/* ]] && return 0
  [[ "$file" == benches/* ]] && return 0

  return 1
}

has_perf_marker() {
  local file="$1"

  rg --quiet --fixed-strings "$perf_marker" "$file"
}

check_feature() {
  local file="$1"
  local line_number="$2"
  local name="$3"

  if [[ "$name" =~ $normal_allowed ]]; then
    return 0
  fi

  if [[ "$name" =~ $perf_only_allowed ]]; then
    if is_perf_scoped_path "$file" || has_perf_marker "$file"; then
      return 0
    fi

    printf 'perf-only unstable feature %s outside approved scope in %s:%s\n' "$name" "$file" "$line_number" >&2
    status=1
    return 0
  fi

  printf 'disallowed unstable feature %s in %s:%s\n' "$name" "$file" "$line_number" >&2
  status=1
}

scan_file() {
  local file="$1"
  local line_number=0
  local feature_line=0
  local line
  local features
  local raw
  local name
  local collecting=0

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_number=$((line_number + 1))

    if (( collecting == 1 )); then
      if [[ "$line" == *')]'* ]]; then
        features+="${line%%')]'*}"
        collecting=0
      else
        features+="$line"
        continue
      fi
    elif [[ "$line" =~ \#\!\[feature\((.*)\)\] ]]; then
      feature_line="$line_number"
      features="${BASH_REMATCH[1]}"
    elif [[ "$line" =~ \#\!\[feature\((.*)$ ]]; then
      feature_line="$line_number"
      features="${BASH_REMATCH[1]}"
      collecting=1
      continue
    else
      continue
    fi

    IFS=',' read -ra names <<< "$features"

    for raw in "${names[@]}"; do
      name="${raw//[[:space:]]/}"
      [[ -z "$name" ]] && continue
      check_feature "$file" "$feature_line" "$name"
    done
  done < "$file"

  if (( collecting == 1 )); then
    printf 'unterminated unstable feature attribute in %s:%s\n' "$file" "$feature_line" >&2
    status=1
  fi
}

while IFS= read -r -d '' file; do
  scan_file "$file"
done < <(
  rg --files -0 \
    -g '*.rs' \
    -g '!target/**' \
    -g '!.git/**' \
    -g '!.beads/**' \
    -g '!vb-*/**' \
    -g '!arch-drift-*/**' \
    -g '!**/target/**' \
    -g '!**/generated-build/**' \
    -g '!**/build-output/**'
)

# RUSTC_BOOTSTRAP enforcement (master §4 "RUSTC_BOOTSTRAP is rejected in developer
# shells, CI, scripts, and docs"). Scan all tracked file types for the env var.
while IFS= read -r -d '' file; do
  # Skip our own audit script (it MUST mention RUSTC_BOOTSTRAP to detect it).
  if [[ "$file" == "scripts/check-nightly-features.sh" ]]; then
    continue
  fi
  # Skip the master contract and governance docs that document the prohibition.
  case "$file" in
    velvet-ballistics-MASTER.md|docs/rust-governance.md|docs/xtask-prd.md) continue ;;
  esac
  if rg --quiet -e 'RUSTC_BOOTSTRAP' "$file"; then
    printf 'RUSTC_BOOTSTRAP is rejected by master §4 in %s\n' "$file" >&2
    status=1
  fi
done < <(
  rg --files -0 \
    -g '*.sh' -g '*.bash' -g '*.zsh' \
    -g '*.yml' -g '*.yaml' -g '*.toml' \
    -g '*.rs' -g '*.py' -g '*.md' \
    -g '!target/**' -g '!.git/**' -g '!.beads/**' \
    -g '!vb-*/**' -g '!arch-drift-*/**' \
    -g '!**/target/**' \
    -g '!**/generated-build/**' \
    -g '!**/build-output/**'
)

exit "$status"
