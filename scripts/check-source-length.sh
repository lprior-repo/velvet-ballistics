#!/usr/bin/env bash
set -euo pipefail

limit=25
status=0

hot_files() {
  rg --files \
    -g 'crates/*/src/engine.rs' \
    -g 'crates/*/src/runtime/**' \
    -g 'crates/*/src/generated/**' \
    -g 'crates/*/src/perf/**' \
    -g '!target/**' \
    -g '!vb-*/**'
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

exit "$status"
