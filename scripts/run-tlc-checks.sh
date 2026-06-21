#!/bin/bash
set -euo pipefail
if [ -n "${TLA2TOOLS_JAR:-}" ] && [ -f "${TLA2TOOLS_JAR}" ]; then
  JAR="${TLA2TOOLS_JAR}"
elif command -v tlc >/dev/null 2>&1; then
  JAR=""
else
  echo "error: cannot locate tla2tools.jar" >&2
  echo "       set TLA2TOOLS_JAR=/path/to/tla2tools.jar, or place a 'tlc' wrapper on PATH" >&2
  exit 127
fi
TLA=verification/tla
for cfg in "$TLA"/*.cfg; do
  tla="${cfg%.cfg}.tla"
  if [ -f "$tla" ]; then
    echo "Checking $tla..."
    if [ -n "$JAR" ]; then
      java -cp "$JAR" tlc2.TLC -seed 0 -config "$cfg" "$tla" 2>&1 | tail -3
    else
      tlc -seed 0 -config "$cfg" "$tla" 2>&1 | tail -3
    fi
  fi
done
