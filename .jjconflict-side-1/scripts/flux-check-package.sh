#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 1 ]; then
  printf 'usage: bash scripts/flux-check-package.sh <package> [cargo-flux options]\n' >&2
  exit 2
fi

package="$1"
shift

for arg in "$@"; do
  case "$arg" in
    --lib|--test|--tests|--benches|--all-targets)
      printf 'unsupported cargo-flux target selector for installed cargo-flux: %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

cargo flux -p "$package" --message-format human "$@"
