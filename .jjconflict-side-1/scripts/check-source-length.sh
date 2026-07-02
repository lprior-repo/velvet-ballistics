#!/usr/bin/env bash
# Source-length gate for velvet-ballistics.
#
# Walks every tracked first-party .rs file, categorizes it via
# scripts/lib-source-length.sh, compares the current line count to the
# per-category limit, and fails on any file past its hard limit without a
# valid exception row in .config/source-length-exceptions.txt. Also emits
# WARN-tier notices for files in 80%-100% of their category limit, even
# when an exception row exists, so the ledger cannot accidentally mask
# ongoing drift.
#
# Backward compatibility: the exception ledger format
#     <file>|<owner>|<split_bead>|<removal_plan>|<reason>
# remains canonical. An optional 6th column overrides category inference.
#
# Exit codes:
#   0   gate passed (with optional WARN-tier stderr notices)
#   1   at least one file over its category limit without a valid
#       exception, OR a fatal ledger validation failure, OR
#       cargo-mutants residue detected, OR a compile-split shell missing

set -euo pipefail

# bash glob `**` only matches across `/` when extglob is enabled.
if [[ "${BASH_VERSINFO[0]}" -ge 3 ]]; then
  shopt -s extglob 2>/dev/null || true
fi

# Output verbosity. Default is "summary": the gate prints the per-category
# summary always, FAIL-tier notices always, and WARN-tier notices only when
# -v/--verbose is passed or GATE_VERBOSE=1. -q/--quiet suppresses the
# summary as well, leaving only FAIL-tier notices on stderr.
gate_verbose=0
gate_quiet=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    -v|--verbose) gate_verbose=1 ;;
    -q|--quiet)   gate_quiet=1 ;;
    -h|--help)
      printf 'usage: %s [-v|--verbose] [-q|--quiet]\n' "$(basename "$0")" >&2
      exit 0
      ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
  shift
done
if [[ "${GATE_VERBOSE:-0}" == "1" ]]; then
  gate_verbose=1
fi
if [[ "${GATE_QUIET:-0}" == "1" ]]; then
  gate_quiet=1
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)

# Allow callers (notably the test harness) to point the gate at a
# fixture directory instead of the real repository root. When set, the
# gate stays read-only on the fixture without touching the real repo.
REPO_ROOT="${GATE_REPO_ROOT:-$REPO_ROOT}"
cd "$REPO_ROOT"

# shellcheck source=lib-source-length.sh
. "$SCRIPT_DIR/lib-source-length.sh"

limit=25
source_length_ledger=".config/source-length-exceptions.txt"
status=0

# Emit a WARN-tier line only when verbosity requests it. Centralized
# so callers can switch behavior globally without sprinkling `if`s.
gate_warn() {
  if [[ "$gate_verbose" -eq 1 ]]; then
    printf '%s\n' "$*" >&2
  fi
}

# Emit a FAIL-tier line always. These set status through the caller.
gate_fail() {
  printf '%s\n' "$*" >&2
}

# Emit summary only when not in --quiet mode.
gate_summary() {
  if [[ "$gate_quiet" -ne 1 ]]; then
    printf '%s\n' "$*" >&2
  fi
}

declare -A tracked_rust_lines=()
declare -A source_length_exceptions=()
declare -A exception_categories=()

# Glob of files whose "very hot" function-length policy is enforced.
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
    -g '!**/tests/**' \
    -g '!**/tests.rs' \
    -g '!**/*_tests.rs' \
    -g '!crates/vb_*/src/verification/**' \
    -g '!crates/vb_runtime/src/verification/**' \
    -g '!target/**' \
    -g '!vb-*/**' \
    -g '!arch-drift-*/**'
}

# cargo-mutants residue scan. Unchanged from the previous gate.
check_mutants_residue() {
  local matches
  local grep_status

  set +e
  if command -v git >/dev/null 2>&1 && \
      git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    matches=$(git -C "$REPO_ROOT" grep -n -I -E 'changed by cargo[-]mutants' -- . ':!target' ':!.moon/cache' ':!.beads' 2>/dev/null || true)
    grep_status=$?
  else
    matches=$(rg -n -I 'changed by cargo[-]mutants' --glob '!target/**' --glob '!.moon/cache/**' --glob '!.beads/**' || true)
    grep_status=$?
  fi
  set -e

  if [[ -n "$matches" ]]; then
    printf 'cargo-mutants residue markers found:\n' >&2
    printf '%s\n' "$matches" >&2
    status=1
  fi
}

# Hot-function length policy. Unchanged from the previous gate.
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

# Path predicates
is_excluded_source_path() {
  local file="$1"
  case "$file" in
    target/*|.jj/*|.beads/*|.evidence/*|.cargo_temp/*|arch-drift-*/*|*/target/*|*/.jj/*|*/.beads/*|*/.evidence/*|*/.cargo_temp/*)
      return 0 ;;
    cargo-home/*|cargo_home/*|.cargo/registry/*|*/cargo-home/*|*/cargo_home/*|*/.cargo/registry/*)
      return 0 ;;
    *)
      return 1 ;;
  esac
}

# Files enumerated via git ls-files (preferred), jj file list (fallback)
# or ripgrep scanning (last resort). Tests are NOT filtered out at this
# stage — they are categorized separately by lib-source-length.sh.
tracked_rust_files() {
  if command -v git >/dev/null 2>&1 && \
      git -C "$REPO_ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    git -C "$REPO_ROOT" ls-files '*.rs'
  elif command -v jj >/dev/null 2>&1 && jj root >/dev/null 2>&1; then
    jj file list 2>/dev/null | grep '\.rs$' || true
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

# Categorize the file via library, then load line count.
load_tracked_rust_lines() {
  local file
  local lines

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    [[ "$file" == *.rs ]] || continue
    is_excluded_source_path "$file" && continue
    [[ -f "$file" ]] || continue
    lines=$(wc -l < "$file")
    tracked_rust_lines["$file"]="$lines"
  done < <(tracked_rust_files)
}

# Strict validation of an exception ledger row.
validate_ledger_path() {
  local file="$1"
  local line_no="$2"
  local category_override="${3-}"

  if [[ "$file" = /* || "$file" = ../* || "$file" = */../* ]]; then
    printf '%s:%s invalid path; use a normalized repository-relative path\n' \
      "$source_length_ledger" "$line_no" >&2
    return 1
  fi
  if [[ "$file" != *.rs ]]; then
    printf '%s:%s path is not a Rust source file: %s\n' \
      "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if is_excluded_source_path "$file"; then
    printf '%s:%s path is excluded from first-party source-length checks: %s\n' \
      "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi
  if [[ -z "${tracked_rust_lines[$file]:-}" ]]; then
    printf '%s:%s path is not a tracked first-party Rust source file: %s\n' \
      "$source_length_ledger" "$line_no" "$file" >&2
    return 1
  fi

  # New check: every exception must point at a file whose line count is
  # actually over the canonical production limit. Files under their
  # category limit do not need an exception row, so the row is stale.
  local category
  category="${category_override:-$(sl_categorize "$file")}"
  if [[ -z "$category_override" ]]; then
    category=$(sl_categorize "$file")
  fi
  local limit
  limit=$(sl_limit "$category")
    if [[ "$limit" -ge 0 ]] && \
     [[ "${tracked_rust_lines[$file]}" -le "$limit" ]]; then
    gate_warn "$(printf '%s:%s stale exception for %s in category %s with %s physical lines (limit %d); remove or re-categorize this row' \
      "$source_length_ledger" "$line_no" "$file" "$category" \
      "${tracked_rust_lines[$file]}" "$limit")"
    # Non-fatal by policy: a stale row no longer causes a hard fail but
    # is reported so the ledger can be cleaned up over time.
    return 0
  fi

  return 0
}

# Walk the ledger file with strict validation. Bead ID format is also
# checked here, so a manual paste of "TODO" or "bead-7" gets caught.
validate_source_length_ledger() {
  local line
  local line_no=0
  local file owner split_bead removal_plan reason extra
  local category_override=""

  if [[ ! -f "$source_length_ledger" ]]; then
    printf '%s missing; required for source-length exceptions\n' \
      "$source_length_ledger" >&2
    status=1
    return
  fi

  while IFS= read -r line || [[ -n "$line" ]]; do
    line_no=$((line_no + 1))
    [[ -z "$line" || "$line" == \#* ]] && continue

    IFS='|' read -r file owner split_bead removal_plan reason extra <<< "$line"
    if [[ -n "${extra:-}" && "${extra:-}" != *"|"* ]] \
        && [[ "$extra" =~ ^[a-z_]+$ ]]; then
      # Optional 6th column: explicit category override.
      category_override="$extra"
    fi

    if [[ -z "${file:-}" || -z "${owner:-}" \
          || -z "${split_bead:-}" || -z "${removal_plan:-}" \
          || -z "${reason:-}" ]]; then
      printf '%s:%s malformed row; expected <file_path>|<owner>|<split_bead>|<removal_plan>|<reason>[|<category>]\n' \
        "$source_length_ledger" "$line_no" >&2
      status=1
      continue
    fi

    if [[ "$owner" != *[a-zA-Z0-9]* ]]; then
      printf '%s:%s owner field is empty or invalid: %s\n' \
        "$source_length_ledger" "$line_no" "$owner" >&2
      status=1
      continue
    fi

    if ! sl_bead_id_valid "$split_bead"; then
      printf '%s:%s split_bead field %q does not match vb-<name>(.<part>)?\n' \
        "$source_length_ledger" "$line_no" "$split_bead" >&2
      status=1
      continue
    fi

    if ! validate_ledger_path "$file" "$line_no" "$category_override"; then
      status=1
      continue
    fi

    if [[ -n "${source_length_exceptions[$file]:-}" ]]; then
      printf '%s:%s duplicate exception for %s\n' \
        "$source_length_ledger" "$line_no" "$file" >&2
      status=1
      continue
    fi

    source_length_exceptions["$file"]="$line_no"
    if [[ -n "$category_override" ]]; then
      exception_categories["$file"]="$category_override"
    fi
  done < "$source_length_ledger"
}

# Per-category source-length check. This is the heart of the new gate:
# every first-party .rs file has a category-specific limit. Files past
# the hard limit fail; files in 80%-100% of the hard limit emit a WARN
# notice on stderr but do not fail, so authors see drift before it
# becomes a violation.
check_source_line_limit() {
  local file
  local lines
  local category
  local limit
  local warn_at
  local category_for_ledger

  load_tracked_rust_lines
  validate_source_length_ledger

  declare -A counts=( \
    [production]=0 [test_in_src]=0 [test_top_level]=0 [kani]=0 \
    [verus]=0 [flux]=0 [verification]=0 [generated]=0 [perf]=0 )
  declare -A over_limit=( \
    [production]=0 [test_in_src]=0 [test_top_level]=0 [kani]=0 \
    [verus]=0 [flux]=0 [verification]=0 [generated]=0 [perf]=0 )
  declare -A warn_count=( \
    [production]=0 [test_in_src]=0 [test_top_level]=0 [kani]=0 \
    [verus]=0 [flux]=0 [verification]=0 [generated]=0 [perf]=0 )

  while IFS= read -r file; do
    [[ -z "$file" ]] && continue
    category=$(sl_categorize "$file")
    lines="${tracked_rust_lines[$file]}"
    limit=$(sl_limit "$category")
    warn_at=$(sl_warn_at "$category")

    counts[$category]=$(( ${counts[$category]} + 1 ))

    # Excluded categories never fail and never warn.
    if [[ "$limit" -lt 0 ]]; then
      continue
    fi

    if [[ "$lines" -gt "$limit" ]]; then
      if [[ -z "${source_length_exceptions[$file]:-}" ]]; then
        gate_fail "$(printf 'FAIL %s [category=%s] has %d physical lines (hard limit %d); add a row to %s or split' \
          "$file" "$category" "$lines" "$limit" "$source_length_ledger")"
        over_limit[$category]=$(( ${over_limit[$category]} + 1 ))
        status=1
      elif [[ "$lines" -ge "$warn_at" ]]; then
        gate_warn "$(printf 'WARN %s [category=%s] has %d physical lines (hard limit %d, exception %s:%d granted)' \
          "$file" "$category" "$lines" "$limit" \
          "$source_length_ledger" "${source_length_exceptions[$file]}")"
        warn_count[$category]=$(( ${warn_count[$category]} + 1 ))
      fi
    elif [[ "$lines" -ge "$warn_at" ]]; then
      gate_warn "$(printf 'WARN %s [category=%s] has %d physical lines (warn at %d, hard limit %d)' \
        "$file" "$category" "$lines" "$warn_at" "$limit")"
      warn_count[$category]=$(( ${warn_count[$category]} + 1 ))
    fi
  done < <(printf '%s\n' "${!tracked_rust_lines[@]}" | LC_ALL=C sort)

  # Summary block. Surfaces the per-category distribution so authors
  # see where drift is concentrated.
  gate_summary "source-length gate summary:"
  while IFS= read -r category; do
    [[ -z "$category" ]] && continue
    local lim
    lim=$(sl_limit "$category")
    if [[ "$lim" -lt 0 ]]; then
      gate_summary "$(printf '  %-18s scanned=%-5d excluded (no limit)' \
        "$category" "${counts[$category]:-0}")"
    else
      gate_summary "$(printf '  %-18s scanned=%-5d warn=%-4d over_limit=%-4d limit=%d warn_at=%d' \
        "$category" \
        "${counts[$category]:-0}" \
        "${warn_count[$category]:-0}" \
        "${over_limit[$category]:-0}" \
        "$lim" \
        "$(sl_warn_at "$category")")"
    fi
  done < <(sl_categories)

  gate_summary "$(printf '  ledger rows used:    %d' "${#source_length_exceptions[@]}")"
}

# Compile-split shell policy. Unchanged from the previous gate.
check_compile_split_sources() {
  local compile_dir="crates/vb_compile/src"
  local file

  if [[ -f "$compile_dir/compile_core_impl.rs" ]]; then
    printf '%s must not remain as a hidden production include body\n' \
      "$compile_dir/compile_core_impl.rs" >&2
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

check_source_line_limit
check_mutants_residue

# The compile-split shell policy is specific to this repository and is
# only meaningful when the gate runs against the real project tree.
# When GATE_REPO_ROOT overrides the root to a test fixture, skip the
# check so fixture tests stay focused on length categorization.
if [[ -z "${GATE_REPO_ROOT:-}" ]]; then
  check_compile_split_sources
fi

exit "$status"
