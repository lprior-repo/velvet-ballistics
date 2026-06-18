#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"

mkdir -p "$ROOT/target/gate-tools"
rustc --edition=2024 "$SCRIPT_DIR/check-source-length.rs" -o "$ROOT/target/gate-tools/check-source-length"
"$ROOT/target/gate-tools/check-source-length"

# DEDUP-11 (vb-t060f): split-or-retire-before-release row count must be
# monotonically non-increasing per quarter. The self-test reads the prior
# quarter counts from $SOURCE_LENGTH_QUARTERLY_STATE (default
# .config/source-length-quarterly-counts.jsonl) and asserts the current
# count does not exceed any prior quarter's recorded count. On a fresh
# state file, the current quarter is appended as the new baseline. Re-runs
# within the same quarter are idempotent.
split_or_retire_quarterly_self_test() {
  local state_file="${SOURCE_LENGTH_QUARTERLY_STATE:-$ROOT/.config/source-length-quarterly-counts.jsonl}"
  local source_ledger="$ROOT/.config/source-length-exceptions.txt"
  local hot_ledger="$ROOT/.config/hot-function-length-exceptions.txt"

  local month
  month="$(date -u +'%-m' 2>/dev/null || date -u +'%m' | sed 's/^0//')"
  local year
  year="$(date -u +'%Y')"
  local quarter_num=$(( (month - 1) / 3 + 1 ))
  local current_quarter="${year}-Q${quarter_num}"
  local current_date
  current_date="$(date -u +'%Y-%m-%d')"

  local current_count=0
  if [ -f "$source_ledger" ]; then
    local source_file owner split_bead removal_plan reason source_extra
    while IFS='|' read -r source_file owner split_bead removal_plan reason source_extra || [ -n "${source_file:-}" ]; do
      case "$source_file" in
        ''|\#*) continue ;;
      esac
      if [ -n "${owner:-}" ] && [ -n "${split_bead:-}" ] && [ -n "${removal_plan:-}" ] && [ -n "${reason:-}" ] && [ -z "${source_extra:-}" ]; then
        current_count=$((current_count + 1))
      fi
    done < "$source_ledger"
  fi
  if [ -f "$hot_ledger" ]; then
    local hot_file start_line hot_owner hot_split_bead hot_removal_plan hot_reason hot_extra
    while IFS='|' read -r hot_file start_line hot_owner hot_split_bead hot_removal_plan hot_reason hot_extra || [ -n "${hot_file:-}" ]; do
      case "$hot_file" in
        ''|\#*) continue ;;
      esac
      if [ -n "${start_line:-}" ] && [ -n "${hot_owner:-}" ] && [ -n "${hot_split_bead:-}" ] && [ -n "${hot_removal_plan:-}" ] && [ -n "${hot_reason:-}" ] && [ -z "${hot_extra:-}" ]; then
        current_count=$((current_count + 1))
      fi
    done < "$hot_ledger"
  fi

  local violation=0
  local violation_report=""
  local current_already_recorded=0

  if [ -f "$state_file" ]; then
    while IFS= read -r line || [ -n "$line" ]; do
      case "$line" in
        ''|\#*) continue ;;
      esac
      local q
      q="$(printf '%s' "$line" | sed -nE 's/.*"quarter"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p')"
      local c
      c="$(printf '%s' "$line" | sed -nE 's/.*"count"[[:space:]]*:[[:space:]]*([0-9]+).*/\1/p')"
      if [ -z "$q" ] || [ -z "$c" ]; then
        continue
      fi
      if [ "$current_count" -gt "$c" ]; then
        violation=1
        local delta=$((current_count - c))
        violation_report="${violation_report}  - quarter ${q} recorded ${c} rows; current ${current_quarter} has ${current_count} (+${delta})"$'\n'
      fi
      if [ "$q" = "$current_quarter" ]; then
        current_already_recorded=1
      fi
    done < "$state_file"
  fi

  if [ "$violation" -eq 1 ]; then
    {
      printf 'DEDUP-11 split-or-retire-before-release quarterly self-test FAILED\n'
      printf 'The count of split-or-retire-before-release rows must be monotonically non-increasing per quarter.\n'
      printf 'Current quarter: %s with %s rows\n' "$current_quarter" "$current_count"
      printf 'Violations (current count exceeds a prior quarter baseline):\n'
      printf '%s' "$violation_report"
      printf 'Triage: review recent additions to %s and %s; the owning split_bead on each new row must actually retire the row before release.\n' "$source_ledger" "$hot_ledger"
    } >&2
    exit 1
  fi

  if [ "$current_already_recorded" -eq 0 ]; then
    mkdir -p "$(dirname -- "$state_file")"
    printf '{"quarter":"%s","count":%s,"date":"%s"}\n' "$current_quarter" "$current_count" "$current_date" >> "$state_file"
  fi
}

split_or_retire_quarterly_self_test
