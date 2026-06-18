#!/usr/bin/env bash
# Failing-first self-tests for the State 9 runtime residue gate contract.
# Runs the three canonical tests named in test-plan.md and exits 0 only when
# the future scripts/forbid-runtime-fmt.sh gate plus moon wiring satisfy them.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"

GATE="$ROOT/scripts/forbid-runtime-fmt.sh"
FIXTURE_DIR="$ROOT/fixtures/forbid-runtime-fmt"
NEGATIVE_SERDE_JSON="$FIXTURE_DIR/negative_serde_json.rs"
NEGATIVE_UNBOUNDED_CHANNEL="$FIXTURE_DIR/negative_unbounded_channel.rs"
NEGATIVE_UNBOUNDED_GROUPED_IMPORT="$FIXTURE_DIR/negative_unbounded_grouped_import.rs"
NEGATIVE_UNBOUNDED_SPACED_PATH="$FIXTURE_DIR/negative_unbounded_spaced_path.rs"
POSITIVE_ALLOWLISTED="$FIXTURE_DIR/positive_allowlisted.rs"
EMPTY_ALLOW="$FIXTURE_DIR/empty.allow"
POSITIVE_ALLOWLIST="$FIXTURE_DIR/positive_allowlisted.allow"
MALFORMED_ALLOW="$FIXTURE_DIR/malformed_unknown_forbidden.allow"
MOON_WITHOUT_DEPS="$FIXTURE_DIR/moon-task-graph-without-deps.yml"
MOON_TASK_GRAPH="$ROOT/.moon/tasks/all.yml"
MASTER_DOC="$ROOT/velvet-ballistics-MASTER.md"
SCANNER_SOURCE="$ROOT/scripts/forbid-runtime-fmt.rs"
WRAPPER_SOURCE="$ROOT/scripts/forbid-runtime-fmt.sh"
RRO_FILE="$ROOT/.beads/tier-a-0-002/rust-refinement-obligations.jsonl"
PROOF_MAP_FILE="$ROOT/.beads/tier-a-0-002/proof-to-rust-map.md"
ALIGNMENT_FILE="$ROOT/.beads/tier-a-0-002/proof-test-source-alignment.md"

HOT_SERDE_JSON_PATH="crates/vb_core/src/lib.rs"
HOT_SERDE_JSON_LINE="3"
HOT_UNBOUNDED_CHANNEL_PATH="crates/vb_runtime/src/channel.rs"
HOT_UNBOUNDED_CHANNEL_LINE="2"
HOT_ALLOWLISTED_PATH="crates/vb_core/src/allowlisted.rs"
HOT_ALLOWLISTED_LINE="3"
TIMEOUT_SECONDS="30"
PERF_BUDGET_NS="30000000000"
SCRIPT_INVOCATION_FAILURE_REASON="forced script invocation failure"

GATE_OUTPUT=""
GATE_EXIT=0
GATE_DURATION_NS=0
MOON_CHECK_OUTPUT=""
MOON_CHECK_EXIT=0
STATIC_CHECK_OUTPUT=""
STATIC_CHECK_EXIT=0
STAGED_ROOT=""
TMP_DIRS=()

cleanup() {
  local tmp_dir
  for tmp_dir in "${TMP_DIRS[@]}"; do
    rm -rf "$tmp_dir"
  done
}
trap cleanup EXIT

fail() {
  local message="$1"
  printf 'AssertionFailed: %s\n' "$message" >&2
  exit 1
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    fail "required fixture missing: $path"
  fi
}

require_fixtures() {
  require_file "$NEGATIVE_SERDE_JSON"
  require_file "$NEGATIVE_UNBOUNDED_CHANNEL"
  require_file "$NEGATIVE_UNBOUNDED_GROUPED_IMPORT"
  require_file "$NEGATIVE_UNBOUNDED_SPACED_PATH"
  require_file "$POSITIVE_ALLOWLISTED"
  require_file "$EMPTY_ALLOW"
  require_file "$POSITIVE_ALLOWLIST"
  require_file "$MALFORMED_ALLOW"
  require_file "$MOON_WITHOUT_DEPS"
  require_file "$MOON_TASK_GRAPH"
  require_file "$MASTER_DOC"
  require_file "$SCANNER_SOURCE"
  require_file "$WRAPPER_SOURCE"
  require_file "$RRO_FILE"
  require_file "$PROOF_MAP_FILE"
  require_file "$ALIGNMENT_FILE"
}

assert_exit() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  local output="$4"
  if [[ "$expected" != "$actual" ]]; then
    printf 'AssertionFailed: %s expected exit %s, got %s\nOutput:\n%s\n' \
      "$label" "$expected" "$actual" "$output" >&2
    exit 1
  fi
}

assert_output_contains() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*) ;;
    *)
      printf 'AssertionFailed: %s missing %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

assert_output_equals() {
  local label="$1"
  local expected="$2"
  local actual="$3"
  if [[ "$expected" != "$actual" ]]; then
    printf 'AssertionFailed: %s expected exact output:\n%s\nActual:\n%s\n' \
      "$label" "$expected" "$actual" >&2
    exit 1
  fi
}

assert_output_omits() {
  local label="$1"
  local needle="$2"
  local haystack="$3"
  case "$haystack" in
    *"$needle"*)
      printf 'AssertionFailed: %s unexpectedly contained %s\nOutput:\n%s\n' \
        "$label" "$needle" "$haystack" >&2
      exit 1
    ;;
  esac
}

assert_duration_under_budget() {
  local label="$1"
  local duration_ns="$2"
  if (( duration_ns > PERF_BUDGET_NS )); then
    fail "$label perf budget exceeded: ${duration_ns}ns > ${PERF_BUDGET_NS}ns"
  fi
}

stage_empty_hot_repo() {
  STAGED_ROOT="$(mktemp -d)"
  TMP_DIRS+=("$STAGED_ROOT")
  mkdir -p \
    "$STAGED_ROOT/crates/vb_core/src" \
    "$STAGED_ROOT/crates/vb_runtime/src" \
    "$STAGED_ROOT/crates/vb_storage/src" \
    "$STAGED_ROOT/crates/vb_ipc/src" \
    "$STAGED_ROOT/scripts"
  cp -f "$MASTER_DOC" "$STAGED_ROOT/velvet-ballistics-MASTER.md"
}

stage_hot_rs_fixture() {
  local fixture="$1"
  local relative_path="$2"
  local allowlist="$3"
  stage_empty_hot_repo
  local relative_dir="${relative_path%/*}"
  mkdir -p "$STAGED_ROOT/$relative_dir"
  cp -f "$fixture" "$STAGED_ROOT/$relative_path"
  cp -f "$allowlist" "$STAGED_ROOT/scripts/forbid-runtime-fmt.allow"
}

stage_hot_rs_fixture_without_master() {
  local fixture="$1"
  local relative_path="$2"
  local allowlist="$3"
  stage_hot_rs_fixture "$fixture" "$relative_path" "$allowlist"
  rm -f "$STAGED_ROOT/velvet-ballistics-MASTER.md"
}

stage_unreadable_hot_root_fixture() {
  STAGED_ROOT="$(mktemp -d)"
  TMP_DIRS+=("$STAGED_ROOT")
  mkdir -p \
    "$STAGED_ROOT/crates/vb_core/src" \
    "$STAGED_ROOT/crates/vb_runtime" \
    "$STAGED_ROOT/crates/vb_storage/src" \
    "$STAGED_ROOT/crates/vb_ipc/src" \
    "$STAGED_ROOT/scripts"
  printf 'not a directory\n' > "$STAGED_ROOT/crates/vb_runtime/src"
  cp -f "$MASTER_DOC" "$STAGED_ROOT/velvet-ballistics-MASTER.md"
  cp -f "$EMPTY_ALLOW" "$STAGED_ROOT/scripts/forbid-runtime-fmt.allow"
}

require_gate_runner() {
  if [[ ! -x "$GATE" ]]; then
    fail "gate script is missing or not executable: $GATE"
  fi
  if ! command -v timeout >/dev/null 2>&1; then
    fail "timeout command missing; cannot enforce ${TIMEOUT_SECONDS}s gate budget"
  fi
}

run_gate_capture_command() {
  require_gate_runner
  set +e
  local output
  local start_ns
  local end_ns
  start_ns="$(date +%s%N)"
  output="$(timeout "${TIMEOUT_SECONDS}s" "$@" 2>&1)"
  local exit_code=$?
  end_ns="$(date +%s%N)"
  set -e
  if [[ "$exit_code" == "124" ]]; then
    output="${output}"$'\n'"GateTimeout: exceeded ${TIMEOUT_SECONDS}s"
  fi
  GATE_OUTPUT="$output"
  GATE_EXIT=$exit_code
  GATE_DURATION_NS=$((end_ns - start_ns))
}

run_gate_capture() {
  run_gate_capture_command bash "$GATE" "$@"
}

run_gate_capture_forced_script_invocation_failure() {
  local reason="$1"
  shift
  run_gate_capture_command \
    env "FORBID_RUNTIME_FMT_FORCE_SCRIPT_INVOCATION_FAILURE=$reason" \
    bash "$GATE" "$@"
}

run_moon_graph_check_capture() {
  local graph_path="$1"
  set +e
  local output
  output="$(python3 - "$graph_path" <<'PY' 2>&1
from pathlib import Path
import re
import sys

graph = Path(sys.argv[1])
lines = graph.read_text(encoding="utf-8").splitlines()

def emit_error(message: str) -> None:
    print(message, file=sys.stderr)

def task_block(task_name: str) -> list[str] | None:
    task_pattern = re.compile(rf"^  {re.escape(task_name)}:\s*$")
    start = None
    for index, line in enumerate(lines):
        if task_pattern.match(line):
            start = index
            break
    if start is None:
        return None
    end = len(lines)
    next_task = re.compile(r"^  [A-Za-z0-9_-]+:\s*$")
    for index in range(start + 1, len(lines)):
        if next_task.match(lines[index]):
            end = index
            break
    return lines[start:end]

def deps_for_check(block: list[str]) -> list[str]:
    deps: list[str] = []
    in_deps = False
    for line in block:
        if re.match(r"^    deps:\s*$", line):
            in_deps = True
            continue
        if in_deps and re.match(r"^    [A-Za-z0-9_-]+:\s*", line):
            break
        if in_deps:
            match = re.match(r"^      - ['\"]?([^'\"\s]+)['\"]?\s*$", line)
            if match:
                deps.append(match.group(1))
    return deps

forbid_block = task_block("forbid-runtime-fmt")
if forbid_block is None:
    emit_error("MISSING-TASK: forbid-runtime-fmt not declared")
    sys.exit(1)
command_lines = [line.strip() for line in forbid_block if line.strip().startswith("command:")]
if not any("scripts/forbid-runtime-fmt.sh" in line for line in command_lines):
    emit_error("MISSING-COMMAND: forbid-runtime-fmt command missing wrapper path")
    sys.exit(1)
if "      runInCI: true" not in forbid_block:
    emit_error("MISSING-CI: forbid-runtime-fmt options.runInCI true missing")
    sys.exit(1)

check_block = task_block("check")
if check_block is None:
    emit_error("MISSING-CHECK: :check task not declared")
    sys.exit(1)
deps = deps_for_check(check_block)
if "forbid-runtime-fmt" not in deps:
    emit_error("MISSING-DEPS: forbid-runtime-fmt not in :check.deps")
    sys.exit(1)

gate_index = deps.index("forbid-runtime-fmt")
cargo_indexes = [idx for idx, dep in enumerate(deps) if "cargo" in dep]
if cargo_indexes and gate_index > min(cargo_indexes):
    emit_error("ORDER-ERROR: forbid-runtime-fmt appears after a cargo dependency")
    sys.exit(1)

print("ok: forbid-runtime-fmt task declared")
print("ok: forbid-runtime-fmt in :check.deps")
print("ok: ordering preserved (gate runs before cargo)")
PY
)"
  local exit_code=$?
  set -e
  MOON_CHECK_OUTPUT="$output"
  MOON_CHECK_EXIT=$exit_code
}

run_compile_bound_check_capture() {
  local graph_path="$1"
  local wrapper_path="$2"
  set +e
  local output
  output="$(python3 - "$graph_path" "$wrapper_path" <<'PY' 2>&1
from pathlib import Path
import re
import sys

graph = Path(sys.argv[1])
wrapper = Path(sys.argv[2])
graph_lines = graph.read_text(encoding="utf-8").splitlines()
wrapper_lines = wrapper.read_text(encoding="utf-8").splitlines()

def emit_error(message: str) -> None:
    print(message, file=sys.stderr)

def task_block(task_name: str) -> list[str] | None:
    task_pattern = re.compile(rf"^  {re.escape(task_name)}:\s*$")
    start = None
    for index, line in enumerate(graph_lines):
        if task_pattern.match(line):
            start = index
            break
    if start is None:
        return None
    end = len(graph_lines)
    next_task = re.compile(r"^  [A-Za-z0-9_-]+:\s*$")
    for index in range(start + 1, len(graph_lines)):
        if next_task.match(graph_lines[index]):
            end = index
            break
    return graph_lines[start:end]

def command_lines(block: list[str]) -> list[str]:
    return [line.strip() for line in block if line.strip().startswith("command:")]

def moon_command_has_outer_timeout(block: list[str]) -> bool:
    return any("timeout" in line and "scripts/forbid-runtime-fmt.sh" in line for line in command_lines(block))

def rustc_compile_lines() -> list[str]:
    return [line.strip() for line in wrapper_lines if "rustc" in line and "forbid-runtime-fmt.rs" in line]

def wrapper_rustc_line_has_timeout() -> bool:
    return any("timeout" in line and "rustc" in line for line in rustc_compile_lines())

block = task_block("forbid-runtime-fmt")
if block is None:
    emit_error("MISSING-TASK: forbid-runtime-fmt not declared")
    sys.exit(1)
if not any("scripts/forbid-runtime-fmt.sh" in line for line in command_lines(block)):
    emit_error("MISSING-COMMAND: forbid-runtime-fmt command missing wrapper path")
    sys.exit(1)

if moon_command_has_outer_timeout(block) or wrapper_rustc_line_has_timeout():
    print("ok: rustc compile step has production wall-clock bound")
    sys.exit(0)

emit_error("UNBOUNDED-COMPILE: production gate has no timeout around rustc compile step")
emit_error("moon command lines: " + repr(command_lines(block)))
emit_error("wrapper rustc lines: " + repr(rustc_compile_lines()))
sys.exit(1)
PY
)"
  local exit_code=$?
  set -e
  STATIC_CHECK_OUTPUT="$output"
  STATIC_CHECK_EXIT=$exit_code
}

run_master_binding_check_capture() {
  local master_path="$1"
  local scanner_path="$2"
  local rro_path="$3"
  local proof_map_path="$4"
  set +e
  local output
  output="$(python3 - "$master_path" "$scanner_path" "$rro_path" "$proof_map_path" <<'PY' 2>&1
from pathlib import Path
import json
import re
import sys

master = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
scanner = Path(sys.argv[2]).read_text(encoding="utf-8")
rro_path = Path(sys.argv[3])
proof_map = Path(sys.argv[4]).read_text(encoding="utf-8")
errors: list[str] = []

def find_trigger_lines() -> dict[str, int]:
    trigger_heading = None
    for index, line in enumerate(master, 1):
        if line.strip() == "Automatic rejection triggers:":
            trigger_heading = index
            break
    if trigger_heading is None:
        errors.append("MASTER-MISSING: §43 Automatic rejection triggers heading")
        return {}
    fence_start = None
    for index in range(trigger_heading + 1, len(master) + 1):
        if master[index - 1].strip() == "```text":
            fence_start = index
            break
    if fence_start is None:
        errors.append("MASTER-MISSING: §43 trigger fenced text block")
        return {}
    result: dict[str, int] = {}
    for index in range(fence_start + 1, len(master) + 1):
        line = master[index - 1].strip()
        if line == "```":
            break
        if line:
            result[line] = index
    return result

trigger_lines = find_trigger_lines()
required_trigger = {
    "TokioSyncMpscUnbounded": "unbounded queue/loop/retry/fanout",
    "SerdeYaml": "YAML interpreted at runtime",
    "SerdeJson": "JSON inserted into runtime core",
    "Hyper": "HTTP inserted into runtime core",
    "Reqwest": "HTTP inserted into runtime core",
    "Axum": "HTTP inserted into runtime core",
    "HashMapStringGeneric": "HashMap<String, Value> runtime state",
}

def source_master_ref(variant: str) -> tuple[int, int] | None:
    token = f"ForbiddenImportName::{variant} =>"
    start = scanner.find(token)
    if start < 0:
        errors.append(f"SOURCE-MISSING: {token}")
        return None
    next_variant = scanner.find("ForbiddenImportName::", start + len(token))
    end = next_variant if next_variant >= 0 else scanner.find("\n        }\n", start)
    chunk = scanner[start:end]
    match = re.search(r"Self::new\(\s*name\s*,\s*ForbiddenImportKind::[A-Za-z]+\s*,\s*(\d+)\s*,\s*(\d+)\s*\)", chunk, re.S)
    if not match:
        errors.append(f"SOURCE-MISSING: {variant} Self::new(..., section, line) master ref")
        return None
    return int(match.group(1)), int(match.group(2))

for variant, trigger in required_trigger.items():
    actual_line = trigger_lines.get(trigger)
    if actual_line is None:
        errors.append(f"MASTER-MISSING: trigger text {trigger!r}")
        continue
    source_ref = source_master_ref(variant)
    if source_ref is None:
        continue
    section, line = source_ref
    if section != 43 or line != actual_line:
        errors.append(
            f"MASTER-REF-MISMATCH: {variant} source ref ({section}, {line}) "
            f"does not bind §43 line {actual_line}: {trigger}"
        )

try:
    rro_rows = [json.loads(line) for line in rro_path.read_text(encoding="utf-8").splitlines() if line.strip()]
except json.JSONDecodeError as exc:
    errors.append(f"RRO-PARSE: {exc}")
    rro_rows = []
rq002_rows = [row for row in rro_rows if row.get("requirement_id") == "RQ-002"]
if not rq002_rows:
    errors.append("RRO-MISSING: RQ-002 row")
for row in rq002_rows:
    text = json.dumps(row, sort_keys=True)
    if "trigger (7|8|9|10)" in text or "4 triggers cited" in text:
        errors.append("RRO-NONBINDING: RQ-002 still cites report-field triggers 7-10")
if "trigger (7|8|9|10)" in proof_map or "4 triggers cited" in proof_map:
    errors.append("PROOF-MAP-NONBINDING: RQ-002 evidence still counts report fields 7-10")

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)
print("ok: RQ-002 binds ForbiddenImportName variants to actual master §43 trigger lines")
PY
)"
  local exit_code=$?
  set -e
  STATIC_CHECK_OUTPUT="$output"
  STATIC_CHECK_EXIT=$exit_code
}

run_formatter_binding_check_capture() {
  local scanner_path="$1"
  local rro_path="$2"
  local proof_map_path="$3"
  local alignment_path="$4"
  set +e
  local output
  output="$(python3 - "$scanner_path" "$rro_path" "$proof_map_path" "$alignment_path" <<'PY' 2>&1
from pathlib import Path
import json
import sys

scanner = Path(sys.argv[1]).read_text(encoding="utf-8")
rro_path = Path(sys.argv[2])
proof_map = Path(sys.argv[3]).read_text(encoding="utf-8")
alignment = Path(sys.argv[4]).read_text(encoding="utf-8")
errors: list[str] = []

required_symbols = {
    "scripts/forbid-runtime-fmt.rs::ResidueMatch::active_line": "fn active_line(&self) -> String",
    "scripts/forbid-runtime-fmt.rs::ResidueMatch::allowlisted_line": "fn allowlisted_line(&self, entry: &AllowlistEntry) -> String",
    "scripts/forbid-runtime-fmt.rs::ScanReport::summary_line": "fn summary_line(&self) -> String",
    "scripts/forbid-runtime-fmt.rs::emit_pass": "fn emit_pass(report: &ScanReport)",
    "scripts/forbid-runtime-fmt.rs::emit_fail": "fn emit_fail(report: &ScanReport)",
}
for symbol, source_needle in required_symbols.items():
    if source_needle not in scanner:
        errors.append(f"SOURCE-SYMBOL-MISSING: {symbol}")

try:
    rows = [json.loads(line) for line in rro_path.read_text(encoding="utf-8").splitlines() if line.strip()]
except json.JSONDecodeError as exc:
    errors.append(f"RRO-PARSE: {exc}")
    rows = []
rq005_rows = [row for row in rows if row.get("requirement_id") == "RQ-005"]
if not rq005_rows:
    errors.append("RRO-MISSING: RQ-005 row")
for row in rq005_rows:
    refs = row.get("source_refs", [])
    if "scripts/forbid-runtime-fmt.rs::ResidueMatch::fmt" in refs:
        errors.append("RRO-NONEXISTENT-SYMBOL: RQ-005 source_refs include ResidueMatch::fmt")
    for symbol in required_symbols:
        if symbol not in refs:
            errors.append(f"RRO-MISSING-SOURCE-REF: RQ-005 missing {symbol}")

for label, body in [("proof-to-rust-map.md", proof_map), ("proof-test-source-alignment.md", alignment)]:
    if "ResidueMatch::fmt" in body:
        errors.append(f"ARTIFACT-NONEXISTENT-SYMBOL: {label} still cites ResidueMatch::fmt")

if errors:
    for error in errors:
        print(error, file=sys.stderr)
    sys.exit(1)
print("ok: RQ-005 maps deterministic stderr format to real source symbols")
PY
)"
  local exit_code=$?
  set -e
  STATIC_CHECK_OUTPUT="$output"
  STATIC_CHECK_EXIT=$exit_code
}

assert_runtime_summary_for_single_hot_file() {
  local label="$1"
  assert_output_contains "$label summary" \
    "summary: active=1 allowlisted=0 files_scanned=1 hot_paths=1 cold_paths=0" \
    "$GATE_OUTPUT"
}

assert_no_gate_error_for_known_residue() {
  local label="$1"
  assert_output_omits "$label PatternFileMissing" \
    "GateError:PatternFileMissing:" "$GATE_OUTPUT"
  assert_output_omits "$label GlobUnreadable" \
    "GateError:GlobUnreadable:" "$GATE_OUTPUT"
  assert_output_omits "$label AllowlistParseFailure" \
    "GateError:AllowlistParseFailure:" "$GATE_OUTPUT"
  assert_output_omits "$label ScriptInvocationFailure" \
    "GateError:ScriptInvocationFailure:" "$GATE_OUTPUT"
  assert_output_omits "$label NewResidueDetected sentinel" \
    "GateError:NewResidueDetected" "$GATE_OUTPUT"
}

assert_pattern_file_missing_for_json_fixture() {
  stage_hot_rs_fixture_without_master \
    "$NEGATIVE_SERDE_JSON" "$HOT_SERDE_JSON_PATH" "$EMPTY_ALLOW"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "missing-master serde_json fixture" "2" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "missing-master GateError variant" \
    "GateError:PatternFileMissing: serde_json" "$GATE_OUTPUT"
  assert_output_omits "missing-master active residue line" \
    "RUNTIME-FMT: serde_json:" "$GATE_OUTPUT"
}

assert_allowlist_parse_failure_for_json_fixture() {
  stage_hot_rs_fixture \
    "$NEGATIVE_SERDE_JSON" "$HOT_SERDE_JSON_PATH" "$MALFORMED_ALLOW"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "malformed allowlist serde_json fixture" "2" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "malformed allowlist GateError variant" \
    "GateError:AllowlistParseFailure: line 2: unknown forbidden name 'serde_jsonx'" \
    "$GATE_OUTPUT"
  assert_output_omits "malformed allowlist active residue line" \
    "RUNTIME-FMT: serde_json:" "$GATE_OUTPUT"
}

assert_glob_unreadable_for_unbounded_hot_root() {
  stage_unreadable_hot_root_fixture
  run_gate_capture "$STAGED_ROOT"
  assert_exit "unreadable vb_runtime hot root" "2" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "unreadable hot root GateError variant" \
    "GateError:GlobUnreadable: crates/vb_runtime/src:" "$GATE_OUTPUT"
  assert_output_omits "unreadable hot root active residue line" \
    "RUNTIME-FMT: tokio::sync::mpsc::unbounded:" "$GATE_OUTPUT"
}

assert_unbounded_channel_variant_fixture() {
  local label="$1"
  local fixture="$2"
  local expected_line="$3"
  stage_hot_rs_fixture \
    "$fixture" "$HOT_UNBOUNDED_CHANNEL_PATH" "$EMPTY_ALLOW"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "$label fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "$label RUNTIME-FMT line" \
    "${HOT_UNBOUNDED_CHANNEL_PATH}:${expected_line}: RUNTIME-FMT: tokio::sync::mpsc::unbounded:" \
    "$GATE_OUTPUT"
  assert_runtime_summary_for_single_hot_file "$label fixture"
  assert_no_gate_error_for_known_residue "$label fixture"
  assert_output_omits "$label cross-pattern serde_json" \
    "RUNTIME-FMT: serde_json:" "$GATE_OUTPUT"
}

assert_allowlist_precedence_fixture() {
  stage_hot_rs_fixture \
    "$POSITIVE_ALLOWLISTED" "$HOT_ALLOWLISTED_PATH" "$POSITIVE_ALLOWLIST"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "allowlisted serde_json fixture" "0" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "allowlisted serde_json exact line" \
    "${HOT_ALLOWLISTED_PATH}:${HOT_ALLOWLISTED_LINE}: allowlisted: temporary test allowlist precedence: use serde_json;" \
    "$GATE_OUTPUT"
  assert_output_contains "allowlisted serde_json summary" \
    "summary: active=0 allowlisted=1 files_scanned=1 hot_paths=1 cold_paths=0" \
    "$GATE_OUTPUT"
  assert_output_omits "allowlisted serde_json active line" \
    "${HOT_ALLOWLISTED_PATH}:${HOT_ALLOWLISTED_LINE}: RUNTIME-FMT: serde_json:" \
    "$GATE_OUTPUT"
}

assert_script_invocation_failure_for_empty_hot_repo() {
  stage_empty_hot_repo
  cp -f "$EMPTY_ALLOW" "$STAGED_ROOT/scripts/forbid-runtime-fmt.allow"
  run_gate_capture_forced_script_invocation_failure \
    "$SCRIPT_INVOCATION_FAILURE_REASON" "$STAGED_ROOT"
  assert_exit "forced script invocation failure" "2" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_equals "forced script invocation GateError" \
    "GateError:ScriptInvocationFailure: ${SCRIPT_INVOCATION_FAILURE_REASON}" \
    "$GATE_OUTPUT"
  assert_output_omits "forced script invocation active residue" \
    "RUNTIME-FMT:" "$GATE_OUTPUT"
}

test_quarantine_gate_blocks_json_import() {
  printf '[1/5] test_quarantine_gate_blocks_json_import\n'
  stage_hot_rs_fixture "$NEGATIVE_SERDE_JSON" "$HOT_SERDE_JSON_PATH" "$EMPTY_ALLOW"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "serde_json fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "serde_json RUNTIME-FMT line" \
    "${HOT_SERDE_JSON_PATH}:${HOT_SERDE_JSON_LINE}: RUNTIME-FMT: serde_json: use serde_json;" \
    "$GATE_OUTPUT"
  assert_output_omits "serde_json formatter wording drift" \
    "exact substring" "$GATE_OUTPUT"
  assert_runtime_summary_for_single_hot_file "serde_json fixture"
  assert_no_gate_error_for_known_residue "serde_json fixture"
  assert_pattern_file_missing_for_json_fixture
  assert_allowlist_parse_failure_for_json_fixture
  echo "  ok: exit 1 with serde_json RUNTIME-FMT line"
  echo "  ok: summary reports active=1 allowlisted=0"
  echo "  ok: exact GateError checks cover PatternFileMissing and AllowlistParseFailure"
}

test_quarantine_gate_blocks_unbounded_channel() {
  printf '[2/5] test_quarantine_gate_blocks_unbounded_channel\n'
  stage_hot_rs_fixture \
    "$NEGATIVE_UNBOUNDED_CHANNEL" "$HOT_UNBOUNDED_CHANNEL_PATH" "$EMPTY_ALLOW"
  run_gate_capture "$STAGED_ROOT"
  assert_exit "unbounded-channel fixture" "1" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_output_contains "unbounded-channel RUNTIME-FMT line" \
    "${HOT_UNBOUNDED_CHANNEL_PATH}:${HOT_UNBOUNDED_CHANNEL_LINE}: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio::sync::mpsc::unbounded_channel();" \
    "$GATE_OUTPUT"
  assert_output_omits "unbounded-channel formatter wording drift" \
    "exact substring" "$GATE_OUTPUT"
  assert_runtime_summary_for_single_hot_file "unbounded-channel fixture"
  assert_no_gate_error_for_known_residue "unbounded-channel fixture"
  assert_output_omits "unbounded-channel cross-pattern serde_json" \
    "RUNTIME-FMT: serde_json:" "$GATE_OUTPUT"
  assert_output_omits "unbounded-channel cross-pattern hyper" \
    "RUNTIME-FMT: hyper:" "$GATE_OUTPUT"
  assert_output_omits "unbounded-channel cross-pattern reqwest" \
    "RUNTIME-FMT: reqwest:" "$GATE_OUTPUT"
  assert_output_omits "unbounded-channel cross-pattern axum" \
    "RUNTIME-FMT: axum:" "$GATE_OUTPUT"

  assert_unbounded_channel_variant_fixture \
    "grouped-import unbounded-channel" "$NEGATIVE_UNBOUNDED_GROUPED_IMPORT" "2"
  assert_output_contains "grouped-import exact snippet" \
    "${HOT_UNBOUNDED_CHANNEL_PATH}:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: use tokio::sync::mpsc::{unbounded_channel};" \
    "$GATE_OUTPUT"

  assert_unbounded_channel_variant_fixture \
    "spaced-path unbounded-channel" "$NEGATIVE_UNBOUNDED_SPACED_PATH" "2"
  assert_output_contains "spaced-path exact snippet" \
    "${HOT_UNBOUNDED_CHANNEL_PATH}:2: RUNTIME-FMT: tokio::sync::mpsc::unbounded: let _channel_pair = tokio :: sync :: mpsc :: unbounded_channel::<u8>();" \
    "$GATE_OUTPUT"

  assert_glob_unreadable_for_unbounded_hot_root
  echo "  ok: exit 1 with unbounded-channel RUNTIME-FMT line"
  echo "  ok: grouped-import and spaced-path unbounded forms are blocked"
  echo "  ok: summary reports active=1 allowlisted=0"
  echo "  ok: exact GateError check covers GlobUnreadable"
  echo "  ok: no cross-pattern false positives"
}

test_moon_ci_quarantine_dependency_correctly_ordered() {
  printf '[3/5] test_moon_ci_quarantine_dependency_correctly_ordered\n'
  run_moon_graph_check_capture "$MOON_TASK_GRAPH"
  assert_exit "real moon task graph" "0" "$MOON_CHECK_EXIT" "$MOON_CHECK_OUTPUT"
  assert_output_contains "real moon task graph dependency" \
    "ok: forbid-runtime-fmt in :check.deps" "$MOON_CHECK_OUTPUT"
  assert_output_contains "real moon task graph ordering" \
    "ok: ordering preserved (gate runs before cargo)" "$MOON_CHECK_OUTPUT"

  run_compile_bound_check_capture "$MOON_TASK_GRAPH" "$WRAPPER_SOURCE"
  assert_exit "production rustc compile bound" "0" "$STATIC_CHECK_EXIT" \
    "$STATIC_CHECK_OUTPUT"
  assert_output_contains "production rustc compile bound" \
    "ok: rustc compile step has production wall-clock bound" \
    "$STATIC_CHECK_OUTPUT"

  assert_allowlist_precedence_fixture

  assert_script_invocation_failure_for_empty_hot_repo

  run_gate_capture
  assert_exit "real repository scan" "0" "$GATE_EXIT" "$GATE_OUTPUT"
  assert_duration_under_budget "real repository scan" "$GATE_DURATION_NS"
  assert_output_contains "real repository active-free summary" \
    "summary: active=0 allowlisted=" "$GATE_OUTPUT"
  assert_output_contains "real repository files scanned" \
    "files_scanned=" "$GATE_OUTPUT"
  assert_output_contains "real repository hot path count" \
    "hot_paths=" "$GATE_OUTPUT"
  assert_output_contains "real repository cold path count" \
    "cold_paths=" "$GATE_OUTPUT"
  assert_output_omits "real repository active residue" \
    "RUNTIME-FMT:" "$GATE_OUTPUT"

  run_moon_graph_check_capture "$MOON_WITHOUT_DEPS"
  assert_exit "moon graph without deps" "1" "$MOON_CHECK_EXIT" "$MOON_CHECK_OUTPUT"
  assert_output_contains "moon graph without deps diagnostic" \
    "MISSING-DEPS: forbid-runtime-fmt not in :check.deps" "$MOON_CHECK_OUTPUT"
  echo "  ok: forbid-runtime-fmt in :check.deps"
  echo "  ok: ordering preserved (gate runs before cargo)"
  echo "  ok: allowlist precedence fixture reports allowlisted=1 and no active line"
  echo "  ok: ScriptInvocationFailure maps to exit 2"
  echo "  ok: real repository scan completed under 30s (${GATE_DURATION_NS}ns)"
  echo "  ok: negative fixture detects missing-deps"
}

test_static_evidence_binds_master_rejection_triggers() {
  printf '[4/5] test_static_evidence_binds_master_rejection_triggers\n'
  run_master_binding_check_capture \
    "$MASTER_DOC" "$SCANNER_SOURCE" "$RRO_FILE" "$PROOF_MAP_FILE"
  assert_exit "RQ-002 master/source binding" "0" "$STATIC_CHECK_EXIT" \
    "$STATIC_CHECK_OUTPUT"
  assert_output_contains "RQ-002 master/source binding" \
    "ok: RQ-002 binds ForbiddenImportName variants to actual master §43 trigger lines" \
    "$STATIC_CHECK_OUTPUT"
  echo "  ok: RQ-002 source refs bind actual master §43 automatic rejection lines"
}

test_static_evidence_binds_real_formatter_symbols() {
  printf '[5/5] test_static_evidence_binds_real_formatter_symbols\n'
  run_formatter_binding_check_capture \
    "$SCANNER_SOURCE" "$RRO_FILE" "$PROOF_MAP_FILE" "$ALIGNMENT_FILE"
  assert_exit "RQ-005 formatter source binding" "0" "$STATIC_CHECK_EXIT" \
    "$STATIC_CHECK_OUTPUT"
  assert_output_contains "RQ-005 formatter source binding" \
    "ok: RQ-005 maps deterministic stderr format to real source symbols" \
    "$STATIC_CHECK_OUTPUT"
  echo "  ok: RQ-005 maps stderr format to existing source symbols"
}

run_all_tests() {
  test_quarantine_gate_blocks_json_import
  test_quarantine_gate_blocks_unbounded_channel
  test_moon_ci_quarantine_dependency_correctly_ordered
  test_static_evidence_binds_master_rejection_triggers
  test_static_evidence_binds_real_formatter_symbols
  echo "self-test PASSED"
}

require_fixtures
case "${1:-all}" in
  all)
    run_all_tests
  ;;
  test_quarantine_gate_blocks_json_import)
    test_quarantine_gate_blocks_json_import
  ;;
  test_quarantine_gate_blocks_unbounded_channel)
    test_quarantine_gate_blocks_unbounded_channel
  ;;
  test_moon_ci_quarantine_dependency_correctly_ordered)
    test_moon_ci_quarantine_dependency_correctly_ordered
  ;;
  test_static_evidence_binds_master_rejection_triggers)
    test_static_evidence_binds_master_rejection_triggers
  ;;
  test_static_evidence_binds_real_formatter_symbols)
    test_static_evidence_binds_real_formatter_symbols
  ;;
  *)
    fail "unknown test name: ${1:-}"
  ;;
esac
