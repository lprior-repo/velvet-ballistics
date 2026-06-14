#!/usr/bin/env bash
set -euo pipefail

template="docs/nightly-update-bead-template.md"

if [[ ! -f "$template" ]]; then
  printf 'nightly update template missing: %s\n' "$template" >&2
  exit 1
fi

require() {
  local label="$1"
  local pattern="$2"
  if ! grep -q "$pattern" "$template"; then
    printf 'nightly update template missing required field: %s\n' "$label" >&2
    exit 1
  fi
}

require 'current nightly' 'Current nightly'
require 'target nightly' 'Target nightly'
require 'motivation' 'Motivation'
require 'changed compiler behavior' 'Changed compiler behavior'
require 'rollback plan' 'Rollback plan'
require 'full CI' 'Full CI'
require 'Miri' 'Miri'
require 'fuzz smoke' 'Fuzz smoke'
require 'recovery tests' 'Recovery tests'
require 'before benchmark' 'Before-update benchmark'
require 'after benchmark' 'After-update benchmark'
require 'benchmark delta' 'Delta summary'

printf 'nightly update template: PASS\n'
