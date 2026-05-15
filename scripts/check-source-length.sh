#!/usr/bin/env bash
set -euo pipefail

limit=25
status=0

hot_files() {
  rg --files \
    -g 'crates/vb_*/src/engine.rs' \
    -g 'crates/vb_*/src/engine/**' \
    -g 'crates/vb_runtime/src/**' \
    -g 'crates/vb_*/src/runtime/**' \
    -g 'crates/vb_*/src/generated/**' \
    -g 'crates/vb_*/src/perf/**' \
    -g 'crates/velvet_ballastics/src/engine.rs' \
    -g 'crates/velvet_ballastics/src/engine/**' \
    -g 'crates/velvet_ballastics/src/runtime/**' \
    -g 'crates/velvet_ballastics/src/generated/**' \
    -g 'crates/velvet_ballastics/src/perf/**' \
    -g '!target/**' \
    -g '!vb-*/**'
}

check_mutants_residue() {
  local matches
  local grep_status

  set +e
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    matches=$(git grep -n -I -E 'changed by cargo[-]mutants' -- . ':!target' ':!.moon/cache' ':!.beads')
  else
    matches=$(rg -n -I 'changed by cargo[-]mutants' --glob '!target/**' --glob '!.moon/cache/**' --glob '!.beads/**' || true)
  fi
  grep_status=$?
  set -e

  if [[ "$grep_status" -eq 0 && -n "$matches" ]]; then
    printf 'cargo-mutants residue markers found:\n' >&2
    printf '%s\n' "$matches" >&2
    status=1
  elif [[ "$grep_status" -gt 1 ]]; then
    printf 'cargo-mutants residue check failed\n' >&2
    status=1
  fi
}

check_file() {
  local file="$1"

  awk -v limit="$limit" -v file="$file" '
    function logical(line) {
      line = trim(line)
      return line != "" && line !~ /^\/\// && line != "{" && line != "}"
    }

    function trim(text) {
      sub(/^[[:space:]]+/, "", text)
      sub(/[[:space:]]+$/, "", text)
      return text
    }

    function braces(text,   idx, ch, delta) {
      delta = 0
      for (idx = 1; idx <= length(text); idx += 1) {
        ch = substr(text, idx, 1)
        if (ch == "{") delta += 1
        if (ch == "}") delta -= 1
      }
      return delta
    }

    /^[[:space:]]*(pub([[:space:]]*\([^)]*\))?[[:space:]]+)?(const[[:space:]]+)?(async[[:space:]]+)?fn[[:space:]]+/ {
      in_fn = 1
      start = NR
      count = 0
      depth = 0
    }

    in_fn {
      if (logical($0)) count += 1
      depth += braces($0)
      if (depth <= 0 && index($0, "{") > 0) {
        if (count > limit) {
          printf "%s:%d hot function has %d logical lines (limit %d)\n", file, start, count, limit > "/dev/stderr"
          failed = 1
        }
        in_fn = 0
      }
    }

    END { exit failed ? 1 : 0 }
  ' "$file" || status=1
}

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  check_file "$file"
done < <(hot_files)

check_mutants_residue

exit "$status"
