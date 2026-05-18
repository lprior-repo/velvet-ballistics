#!/usr/bin/env bash
set -euo pipefail

limit=25
source_line_limit=300
status=0

hot_files() {
  rg --files \
    -g 'crates/vb_*/src/engine.rs' \
    -g 'crates/vb_*/src/engine/**' \
    -g 'crates/vb_runtime/src/**' \
    -g 'crates/vb_*/src/runtime/**' \
    -g 'crates/vb_*/src/generated/**' \
    -g 'crates/vb_*/src/perf/**' \
    -g 'crates/vb_cli/src/engine.rs' \
    -g 'crates/vb_cli/src/engine/**' \
    -g 'crates/vb_cli/src/runtime/**' \
    -g 'crates/vb_cli/src/generated/**' \
    -g 'crates/vb_cli/src/perf/**' \
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

check_source_line_limit() {
  local compile_dir="crates/vb_compile/src"
  local file
  local lines
  local saved_globstar
  local saved_nullglob

  saved_globstar=$(shopt -p globstar || true)
  saved_nullglob=$(shopt -p nullglob || true)
  shopt -s globstar nullglob

  for file in \
    "$compile_dir"/*.rs \
    "$compile_dir"/mod_compile_*/**/*.rs; do
    [[ -e "$file" ]] || continue
    lines=$(wc -l < "$file")
    if [[ "$lines" -ge "$source_line_limit" ]]; then
      case "$file" in
        "$compile_dir/lib.rs"|"$compile_dir"/mod_compile_*.rs|"$compile_dir"/mod_compile_*/**/*.rs)
          printf '%s has %d physical lines (limit <%d)\n' "$file" "$lines" "$source_line_limit" >&2
          status=1
          ;;
        *)
          printf 'DEFERRED_GLOBAL: %s has %d physical lines (limit <%d)\n' "$file" "$lines" "$source_line_limit" >&2
          ;;
      esac
    fi
  done

  eval "$saved_globstar"
  eval "$saved_nullglob"
}

check_compile_split_sources() {
  local compile_dir="crates/vb_compile/src"
  local file

  if [[ -f "$compile_dir/compile_core_impl.rs" ]]; then
    printf '%s must not remain as a hidden production include body\n' "$compile_dir/compile_core_impl.rs" >&2
    status=1
  fi

  for file in \
    "$compile_dir/mod_compile_core.rs" \
    "$compile_dir/mod_compile_errors.rs" \
    "$compile_dir/mod_compile_validation.rs" \
    "$compile_dir/mod_compile_lowering.rs"; do
    if [[ ! -f "$file" ]]; then
      printf '%s missing from compile split\n' "$file" >&2
      status=1
      continue
    fi
    if rg -n 'include!\s*\(' "$file" >/dev/null; then
      printf '%s contains monolithic include body\n' "$file" >&2
      status=1
    fi
    if [[ "$(wc -l < "$file")" -lt 50 ]] && ! rg -n '^mod ' "$file" >/dev/null; then
      printf '%s is doc-only shell, not an owned implementation module\n' "$file" >&2
      status=1
    fi
  done
}

while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  check_file "$file"
done < <(hot_files)

check_mutants_residue
check_source_line_limit
check_compile_split_sources

exit "$status"
