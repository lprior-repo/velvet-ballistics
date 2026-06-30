#!/bin/bash
set -euo pipefail
JAR=/home/lewis/.local/share/mise/http-tarballs/36e4d95a99aa33dde9ff7b288bf3092f3dfbb26e450fc9758ee765cdb250ce38/tla2tools.jar
TLA=verification/tla
for cfg in "$TLA"/*.cfg; do
  tla="${cfg%.cfg}.tla"
  if [ -f "$tla" ]; then
    echo "Checking $tla..."
    java -cp "$JAR" tlc2.TLC -seed 0 -config "$cfg" "$tla" 2>&1 | tail -3
  fi
done
