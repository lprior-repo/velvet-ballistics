#!/usr/bin/env bash
set -euo pipefail

limit="${SOURCE_LENGTH_HOT_FUNCTION_LIMIT:-25}"
source_line_limit="${SOURCE_LENGTH_FILE_LIMIT:-300}"
source_length_ledger="${SOURCE_LENGTH_LEDGER:-.config/source-length-exceptions.txt}"
hot_function_ledger="${SOURCE_LENGTH_HOT_FUNCTION_LEDGER:-.config/hot-function-length-exceptions.txt}"
status=0

declare -A tracked_rust_lines=()
declare -A source_length_exceptions=()
declare -A hot_function_exceptions=()

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
  local violations
  local awk_status
  local violation_file
  local start
  local count
  local key

  set +e
  violations=$(awk -v limit="$limit" -v file="$file" '
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
      seen_body = 0
    }

    in_fn {
      if (logical($0)) count += 1
      if (index($0, "{") > 0) seen_body = 1
      depth += braces($0)
      if (seen_body && depth <= 0) {
        if (count > limit) {
          printf "%s|%d|%d\n", file, start, count
        }
        in_fn = 0
      }
    }
  ' "$file")
  awk_status=$?
  set -e

  if [[ "$awk_status" -ne 0 ]]; then
    status=1
    return
  fi

  while IFS='|' read -r violation_file start count; do
    [[ -z "${violation_file:-}" ]] && continue
    key="${violation_file}:${start}"
    if [[ -n "${hot_function_exceptions[$key]:-}" ]]; then
      continue
    fi
    printf "%s:%s hot function has %s logical lines (limit %d)\n" \
      "$violation_file" "$start" "$count" "$limit" >&2
    status=1
  done <<< "$violations"
}

is_excluded_source_path() {
  local file="$1"

  case "$file" in
    target/*|.jj/*|.beads/*|.evidence/*|.cargo_temp/*|arch-drift-*/*|*/target/*|*/.jj/*|*/.beads/*|*/.evidence/*|*/.cargo_temp/*)
      return 0
      ;;
    cargo-home/*|cargo_home/*|.cargo/registry/*|*/cargo-home/*|*/cargo_home/*|*/.cargo/registry/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

is_test_like_source_path() {
  local file="$1"

  case "$file" in
    */tests.rs|*/*_tests.rs|*/*tests*.rs|*/tests/*|*/tests/**/*|*/tests*/*|*/tests*/**)
      return 0
      ;;
    */diagnostic*|*/diagnostics/*|*/diagnostics/**/*|*/fixture*|*/fixtures/*|*/fixtures/**/*)
      return 0
      ;;
    */harness*|*harness*.rs|*/harnesses/*|*/harnesses/**/*|*/kani*.rs|*/kani/*|*/kani/**/*|*/loom/*|*/loom/**/*)
      return 0
      ;;
    */model/*|*/models/*|*/models/**/*|*model*.rs|*/proof*|*proof*.rs|*/proofs/*|*/proofs/**/*|*/property*|*property*.rs|*/properties/*|*/properties/**/*)
      return 0
      ;;
    */verification/*|*/verification/**/*|verification/*|verification/**/*|*/proptest*.rs|*/benches/*|*/benches/**/*|*/support/*|*/support/**/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

is_hot_function_source_path() {
  local file="$1"

  if is_test_like_source_path "$file"; then
    return 1
  fi

  case "$file" in
    crates/vb_*/src/engine.rs|crates/vb_*/src/engine/*|crates/vb_*/src/engine/**/*)
      return 0
      ;;
    crates/vb_runtime/src/*|crates/vb_runtime/src/**/*)
      return 0
      ;;
    crates/vb_*/src/runtime/*|crates/vb_*/src/runtime/**/*)
      return 0
      ;;
    crates/vb_*/src/generated/*|crates/vb_*/src/generated/**/*)
      return 0
      ;;
    crates/vb_*/src/perf/*|crates/vb_*/src/perf/**/*)
      return 0
      ;;
    crates/vb_cli/src/engine.rs|crates/vb_cli/src/engine/*|crates/vb_cli/src/engine/**/*)
      return 0
      ;;
    crates/vb_cli/src/runtime/*|crates/vb_cli/src/runtime/**/*)
      return 0
      ;;
    crates/vb_cli/src/generated/*|crates/vb_cli/src/generated/**/*)
      return 0
      ;;
    crates/vb_cli/src/perf/*|crates/vb_cli/src/perf/**/*)
      return 0
      ;;
    *)
      return 0
      ;;
  esac
}

tracked_rust_files() {
  if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git ls-files '*.rs'
  elif command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
    jj file list | rg '\.rs$'
  else
    rg --files -g '*.rs' \
      -g '!target/**' \
      -g '!.jj/**' \
      -g '!.beads/**' \
      -g '!.evidence/**' \
      -g '!.cargo_temp/**' \
      -g '!cargo-home/**' \
      -g '!cargo_home/**' \
      -g '!.cargo/registry/**'
  fi
}

load_tracked_rust_lines() {
  local file
  local lines

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    [[ "$file" == *.rs ]] || continue
    is_excluded_source_path "$file" && continue
    if [[ ! -f "$file" ]]; then
      continue
    fi
    lines=$(wc -l < "$file")
    tracked_rust_lines["$file"]="$lines"
  done < <(tracked_rust_files)
}

validate_ledger_path() {
  local file="$1"
  local line_no="$2"

  if [[ "$file" = /* || "$file" = ../* || "$file" = */../* ]]; then
    printf '%s:%s invalid path; use a normalized repository-relative path\n' "$source_length_ledger" "$line_no" >&2
    return 1
  fi
  if [[ "$file" != *.rs ]]; then
    printf '%s:%s path is not a Rust source file: %s\n' "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if is_excluded_source_path "$file"; then
    printf '%s:%s path is excluded from first-party source-length checks: %s\n' "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if [[ -z "${tracked_rust_lines[$file]:-}" ]]; then
    printf '%s:%s path is not a tracked first-party Rust source file: %s\n' "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if [[ "${tracked_rust_lines[$file]}" -le "$source_line_limit" ]]; then
    printf '%s:%s stale exception for %s with %s physical lines (limit >%d); keeping non-fatal for historical ledger cleanup\n' \
      "$source_length_ledger" "$line_no" "$file" "${tracked_rust_lines[$file]}" "$source_line_limit" >&2
    return 0
  fi

  return 0
}

validate_source_length_ledger() {
  local line
  local line_no=0
  local file
  local owner
  local split_bead
  local removal_plan
  local reason
  local extra

  if [[ ! -f "$source_length_ledger" ]]; then
    printf '%s missing; required for source-length exceptions\n' "$source_length_ledger" >&2
    status=1
    return
  fi

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_no=$((line_no + 1))
    [[ -z "$line" || "$line" == \#* ]] && continue

    IFS='|' read -r file owner split_bead removal_plan reason extra <<< "$line"
    if [[ -n "${extra:-}" || -z "${file:-}" || -z "${owner:-}" || -z "${split_bead:-}" || -z "${removal_plan:-}" || -z "${reason:-}" ]]; then
      printf '%s:%s malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>\n' \
        "$source_length_ledger" "$line_no" >&2
      status=1
      continue
    fi

    if ! validate_ledger_path "$file" "$line_no"; then
      status=1
      continue
    fi

    if [[ -n "${source_length_exceptions[$file]:-}" ]]; then
      printf '%s:%s duplicate exception for %s\n' "$source_length_ledger" "$line_no" "$file" >&2
      status=1
      continue
    fi

    source_length_exceptions["$file"]="$line_no"
  done < "$source_length_ledger"
}

validate_hot_function_ledger_path() {
  local file="$1"
  local start_line="$2"
  local line_no="$3"

  if [[ "$file" = /* || "$file" = ../* || "$file" = */../* ]]; then
    printf '%s:%s invalid path; use a normalized repository-relative path\n' "$hot_function_ledger" "$line_no" >&2
    return 1
  fi
  if [[ "$file" != *.rs ]]; then
    printf '%s:%s path is not a Rust source file: %s\n' "$hot_function_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if [[ ! "$start_line" =~ ^[1-9][0-9]*$ ]]; then
    printf '%s:%s start line is not a positive integer: %s\n' "$hot_function_ledger" "$line_no" "$start_line" >&2
    return 1
  fi
  if is_excluded_source_path "$file"; then
    printf '%s:%s path is excluded from first-party source-length checks: %s\n' "$hot_function_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if [[ -z "${tracked_rust_lines[$file]:-}" ]]; then
    printf '%s:%s path is not a tracked first-party Rust source file: %s\n' "$hot_function_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if ! is_hot_function_source_path "$file"; then
    printf '%s:%s path is not in the hot-function scan scope: %s\n' "$hot_function_ledger" "$line_no" "$file" >&2
    return 1
  fi

  return 0
}

validate_hot_function_ledger() {
  local line
  local line_no=0
  local file
  local start_line
  local owner
  local split_bead
  local removal_plan
  local reason
  local extra
  local key

  if [[ ! -f "$hot_function_ledger" ]]; then
    printf '%s missing; required for hot-function length exceptions\n' "$hot_function_ledger" >&2
    status=1
    return
  fi

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_no=$((line_no + 1))
    [[ -z "$line" || "$line" == \#* ]] && continue

    IFS='|' read -r file start_line owner split_bead removal_plan reason extra <<< "$line"
    if [[ -n "${extra:-}" || -z "${file:-}" || -z "${start_line:-}" || -z "${owner:-}" || -z "${split_bead:-}" || -z "${removal_plan:-}" || -z "${reason:-}" ]]; then
      printf '%s:%s malformed row; expected <file_path>|<start_line>|<owner>|<split_bead>|<removal_plan>|<reason>\n' \
        "$hot_function_ledger" "$line_no" >&2
      status=1
      continue
    fi

    if ! validate_hot_function_ledger_path "$file" "$start_line" "$line_no"; then
      status=1
      continue
    fi

    key="$file:$start_line"
    if [[ -n "${hot_function_exceptions[$key]:-}" ]]; then
      printf '%s:%s duplicate exception for %s\n' "$hot_function_ledger" "$line_no" "$key" >&2
      status=1
      continue
    fi

    hot_function_exceptions["$key"]="$line_no"
  done < "$hot_function_ledger"
}

check_source_line_limit() {
  local file
  local lines

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    lines="${tracked_rust_lines[$file]}"
    if [[ "$lines" -gt "$source_line_limit" && -z "${source_length_exceptions[$file]:-}" ]]; then
      printf '%s has %d physical lines (limit <=%d) and no valid %s row\n' \
        "$file" "$lines" "$source_line_limit" "$source_length_ledger" >&2
      status=1
    fi
  done < <(printf '%s\n' "${!tracked_rust_lines[@]}" | LC_ALL=C sort)
}

check_hot_function_line_limit() {
  local file

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    if is_hot_function_source_path "$file"; then
      check_file "$file"
    fi
  done < <(printf '%s\n' "${!tracked_rust_lines[@]}" | LC_ALL=C sort)
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

load_tracked_rust_lines
validate_source_length_ledger
validate_hot_function_ledger
check_source_line_limit
check_hot_function_line_limit
check_mutants_residue
check_compile_split_sources

exit "$status"
