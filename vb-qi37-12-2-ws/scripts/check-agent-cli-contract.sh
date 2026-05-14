#!/usr/bin/env bash
set -euo pipefail

status=0
cli_src="crates/velvet_ballastics/src"
master="velvet-ballistics-MASTER.md"

require_literal() {
  local literal="$1"
  shift

  if ! rg --quiet --fixed-strings "$literal" "$@"; then
    printf 'agent CLI contract missing required literal: %s\n' "$literal" >&2
    status=1
  fi
}

reject_literal() {
  local literal="$1"
  shift

  if rg --quiet --fixed-strings "$literal" "$@"; then
    printf 'agent CLI contract rejected literal: %s\n' "$literal" >&2
    rg --line-number --fixed-strings "$literal" "$@" >&2 || true
    status=1
  fi
}

require_literal '"agent-context"' "$cli_src"
require_literal '"schema_version"' "$cli_src"
require_literal '"--json"' "$cli_src"
require_literal '"stdout"' "$cli_src"
require_literal '"stderr"' "$cli_src"
require_literal 'Agent-First CLI Principles' "$master"

reject_literal '"info" =>' "$cli_src"
reject_literal '"ls" =>' "$cli_src"
reject_literal 'named_flag(args, "--format")' "$cli_src"
reject_literal 'named_flag(args, "--output")' "$cli_src"
reject_literal 'named_flag(args, "--skip-confirmations")' "$cli_src"
reject_literal 'named_flag(args, "--skip-confirmation")' "$cli_src"

exit "$status"
