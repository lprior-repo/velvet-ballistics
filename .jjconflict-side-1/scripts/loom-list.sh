#!/usr/bin/env bash
set -euo pipefail
# loom-list.sh — enumerate loom models via xtask loom (fail-closed wrapper)
#
# The `cargo xtask loom` command currently requires --model <name>. This
# script provides a --list equivalent by querying xtask with a sentinel
# model name and parsing the "Available loom models:" output.
#
# usage: bash scripts/loom-list.sh
#   exit 0: model list emitted to stdout; at least one model found
#   exit 1: xtask failed or no models found
#   exit 2: usage error
#
# Obligation: PO-011 (loom model enumeration)

if [ "$#" -ne 0 ]; then
  printf 'usage: bash scripts/loom-list.sh\n' >&2
  exit 2
fi

# Use a sentinel that will never be a valid model name
sentinel="__loom_list_enumerate__"
output="$(mktemp)"
trap 'rm -f -- "$output"' EXIT

set +e
cargo xtask loom --model "$sentinel" >"$output" 2>&1
xtask_exit=$?
set -e

# xtask should exit non-zero since sentinel is not a valid model
# But we still want to parse the "Available loom models:" listing
if [ "$xtask_exit" -eq 0 ]; then
  printf '[loom-list] WARNING: xtask exited 0 for sentinel model (unexpected)\n' >&2
fi

# Parse model names. xtask prints them in two formats:
# 1. Indented list: "  journal_writer_queue — tests ..."
# 2. JSON array: "Available models: [\"journal_writer_queue\", ...]"
# Strategy: extract from the JSON array format (more reliable)
json_array_line="$(grep 'Available models:' "$output" | head -1)"
model_names="$(echo "$json_array_line" | sed -n 's/.*\[\(.*\)\].*/\1/p' | tr ',' '\n' | sed 's/[ "]//g' | grep -v '^$')"

# Fallback: if JSON array not found, try indented list format
if [ -z "$model_names" ]; then
  # Use python to extract model names from indented lines with any dash separator
  model_names="$(echo "$output" | python3 -c "
import sys, re
for line in sys.stdin:
    m = re.match(r'^\s+([a-z_]+)\s', line)
    if m and m.group(1) not in ('Available', 'Error:'):
        print(m.group(1))
" 2>/dev/null)"
fi

if [ -z "$model_names" ]; then
  printf '[loom-list] FAIL: could not parse model list from xtask output\n' >&2
  printf '[loom-list] Raw output:\n' >&2
  cat "$output" >&2
  exit 1
fi

count="$(echo "$model_names" | wc -l)"
printf '[loom-list] Found %d loom models:\n' "$count" >&2
echo "$model_names"
exit 0
