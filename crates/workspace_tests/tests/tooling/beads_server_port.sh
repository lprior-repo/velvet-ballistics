set -euo pipefail

checker="$PWD/scripts/check-beads-server-mode.sh"
fixture_dir=$(mktemp -d)
trap 'rm -rf "$fixture_dir"' EXIT

mkdir -p "$fixture_dir/valid/.beads" "$fixture_dir/negative/.beads"
cp -f .beads/metadata.json "$fixture_dir/valid/.beads/metadata.json"

(cd "$fixture_dir/valid" && bash "$checker") >"$fixture_dir/pass.out" 2>"$fixture_dir/pass.err"
test "$(cat "$fixture_dir/pass.out")" = "beads server-mode check passed"
test -s "$fixture_dir/pass.err" && { cat "$fixture_dir/pass.err" >&2; exit 1; }

jq '. + {"dolt_server_port": 1}' .beads/metadata.json > "$fixture_dir/negative/.beads/metadata.json"
if (cd "$fixture_dir/negative" && bash "$checker") >"$fixture_dir/fail.out" 2>"$fixture_dir/fail.err"; then
  echo "unexpected success" >&2
  exit 1
else
  status=$?
  test "$status" -eq 1
fi
test -s "$fixture_dir/fail.out" && { cat "$fixture_dir/fail.out" >&2; exit 1; }
test "$(cat "$fixture_dir/fail.err")" = "beads server-mode check failed: do not pin dolt_server_port in metadata; bd manages .beads/dolt-server.port"

echo "2 regression cases passed"
