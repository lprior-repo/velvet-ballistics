#!/usr/bin/env bash
# Self-test for the Moon `test` task isolated-scratch policy (vb-x6o83).
# Extracts the real task script from .moon/tasks/all.yml and runs it with
# stubbed flock/timeout/rustup, asserting: fresh unique TMPDIR under the
# system TMPDIR, cleanup on success and failure, exit-code propagation,
# ambient-TMPDIR-override parent selection, and spaces in paths.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
task_file="${repo_root}/.moon/tasks/all.yml"

work="$(mktemp -d "${TMPDIR:-/tmp}/scratch-selftest-XXXXXX")"
trap 'rm -rf "$work"' EXIT

awk '
  /^  test:$/ { in_test = 1; next }
  in_test && /^    script: \|$/ { in_script = 1; next }
  in_script && /^    [^ ]/ { exit }
  in_script {
    if ($0 ~ /^      /) { print substr($0, 7) }
    else { print $0 }
  }
' "$task_file" > "${work}/task.sh"

grep -q 'mktemp -d' "${work}/task.sh" || { echo "FAIL: extraction missed mktemp"; exit 1; }
grep -q 'trap cleanup_scratch EXIT' "${work}/task.sh" || { echo "FAIL: extraction missed trap"; exit 1; }
bash -n "${work}/task.sh"

mkdir -p "${work}/bin" "${work}/out"
cat > "${work}/bin/flock" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
while [ "$#" -gt 0 ]; do
  case "$1" in --shared|--exclusive) shift ;; *) break ;; esac
done
shift
exec "$@"
EOF
cat > "${work}/bin/timeout" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
shift
exec "$@"
EOF
cat > "${work}/bin/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
printf '%s' "${TMPDIR:-<unset>}" > "${STUB_OUT}/seen_${STUB_TAG}"
touch "${TMPDIR}/probe-ok"
exit "${STUB_EXIT:-0}"
EOF
chmod +x "${work}/bin/flock" "${work}/bin/timeout" "${work}/bin/rustup"

run_task() {
  local tag="$1" exit_code="$2"
  shift 2
  local run_dir
  run_dir="$(mktemp -d "${work}/run-XXXXXX")"
  local code=0
  (
    cd "$run_dir"
    STUB_OUT="${work}/out" STUB_TAG="$tag" STUB_EXIT="$exit_code" \
      PATH="${work}/bin:${PATH}" env "$@" bash "${work}/task.sh"
  ) > "${work}/out/log_${tag}" 2>&1 || code="$?"
  printf '%s' "$code"
}

fail() { echo "FAIL: $1"; exit 1; }

# Case 1: default base (/tmp), success.
code="$(run_task "base1" 0 env -u TMPDIR)"
[ "$code" -eq 0 ] || fail "success run exited ${code}"
seen1="$(cat "${work}/out/seen_base1")"
case "$seen1" in /tmp/moon-test-*) ;; *) fail "TMPDIR not fresh scratch: ${seen1}" ;; esac
[ -e "$seen1" ] && fail "scratch not cleaned on success: ${seen1}"

# Case 2: uniqueness across runs.
code="$(run_task "base2" 0 env -u TMPDIR)"
[ "$code" -eq 0 ] || fail "second run exited ${code}"
seen2="$(cat "${work}/out/seen_base2")"
[ "$seen1" != "$seen2" ] || fail "scratch not unique: ${seen1}"

# Case 3: failure propagates exit code and still cleans up.
code="$(run_task "fail" 3 env -u TMPDIR)"
[ "$code" -eq 3 ] || fail "exit code not propagated, got ${code}"
seen3="$(cat "${work}/out/seen_fail")"
[ -e "$seen3" ] && fail "scratch not cleaned on failure: ${seen3}"

# Case 4: ambient TMPDIR override (with spaces) selects the parent.
override_base="${work}/override base"
mkdir -p "$override_base"
code="$(run_task "override" 0 env "TMPDIR=${override_base}")"
[ "$code" -eq 0 ] || fail "override run exited ${code}"
seen4="$(cat "${work}/out/seen_override")"
case "$seen4" in "${override_base}"/moon-test-*) ;; *) fail "override parent not honored: ${seen4}" ;; esac
[ "$seen4" != "$override_base" ] || fail "override dir used directly, not isolated"
[ -e "$seen4" ] && fail "override scratch not cleaned: ${seen4}"
[ -d "$override_base" ] || fail "override parent itself was removed"

echo "PASS: isolated scratch policy (unique=${seen1##*/}, propagated-fail=3, override-ok)"
