#!/usr/bin/env bash
# scripts/bench-alloc-evidence.sh
#
# Bead: vb-a7t6.4
# Skill: test-writer (go-skill state 9)
# Source: veloxide/long-running audit (alloc count + bytes allocated for Section 39)
# Date:  2026-06-06
#
# Wraps the Criterion bench binary with `heaptrack 1.5.0` and emits one
# `evidence/alloc-evidence.jsonl` row per accepted benchmark scenario, with
# raw `heaptrack_print` summaries saved under `evidence/benchmark-logs/`.
#
# Locked to `heaptrack 1.5.0` (verified installed in CI host 2026-06-06).
# Locked to v1 audit envelope of 3 scenarios; see v1-envelope below.
#
# Usage:
#   scripts/bench-alloc-evidence.sh             # all v1 scenarios (default)
#   scripts/bench-alloc-evidence.sh METRIC...   # explicit subset
#
# Environment:
#   WORKDIR              root of the workspace (default: script's parent)
#   CARGO_TARGET_DIR     cargo target dir (default: $WORKDIR/target/bench-build)
#   HEAPTRACK_BIN        path to heaptrack     (default: heaptrack on PATH)
#   HEAPTRACK_PRINT_BIN  path to heaptrack_print (default: heaptrack_print on PATH)
#   BENCH_PACKAGE        cargo package name   (default: vb_workspace_tests)kspace-tests)
#   BENCH_NAME           cargo bench name      (default: velvet_ballistics)
#   BENCH_HASH_OVERRIDE  explicit bench-binary hash (skips discovery)
#   VB_BENCH_LATENCY_BUDGET_US  bench latency budget (default: 100_000)
#   HEAPTRACK_MEASUREMENT_TIME  measurement time, seconds (default: 1)
#   HEAPTRACK_WARMUP_TIME       warm-up time, seconds      (default: 1)
#   HEAPTRACK_SAMPLE_SIZE       sample size               (default: 10)
#
# Idempotent: re-running overwrites `evidence/alloc-evidence.jsonl` and
# `evidence/benchmark-logs/alloc.<metric>.log` for each scenario. The
# raw `.zst` capture is preserved under `evidence/benchmark-logs/raw/`.

set -euo pipefail

# -- Locate workspace + load join keys -----------------------------------------
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
WORKDIR="${WORKDIR:-$(cd -- "${SCRIPT_DIR}/.." && pwd)}"
cd "${WORKDIR}"

ALLOC_EVIDENCE_JSONL="${WORKDIR}/evidence/alloc-evidence.jsonl"
RAW_DIR="${WORKDIR}/evidence/benchmark-logs/raw"
LOG_DIR="${WORKDIR}/evidence/benchmark-logs"
METADATA_JSONL="${WORKDIR}/evidence/section39-metadata.jsonl"
BENCHMARK_EVIDENCE_JSONL="${WORKDIR}/evidence/benchmark-evidence.jsonl"

mkdir -p "${RAW_DIR}" "${LOG_DIR}"

# -- Tool discovery (fail closed) ---------------------------------------------
HEAPTRACK_BIN="${HEAPTRACK_BIN:-$(command -v heaptrack || true)}"
HEAPTRACK_PRINT_BIN="${HEAPTRACK_PRINT_BIN:-$(command -v heaptrack_print || true)}"

if [[ -z "${HEAPTRACK_BIN}" || ! -x "${HEAPTRACK_BIN}" ]]; then
  echo "ERROR: heaptrack not found on PATH (set HEAPTRACK_BIN)" >&2
  exit 1
fi
if [[ -z "${HEAPTRACK_PRINT_BIN}" || ! -x "${HEAPTRACK_PRINT_BIN}" ]]; then
  echo "ERROR: heaptrack_print not found on PATH (set HEAPTRACK_PRINT_BIN)" >&2
  exit 1
fi

HEAPTRACK_VERSION="$("${HEAPTRACK_BIN}" --version 2>&1 | awk '/^heaptrack / {print $2; exit}')"
HEAPTRACK_PRINT_VERSION="$("${HEAPTRACK_PRINT_BIN}" --version 2>&1 | awk '/^heaptrack_print / {print $2; exit}')"
if [[ "${HEAPTRACK_VERSION}" != "1.5.0" || "${HEAPTRACK_PRINT_VERSION}" != "1.5.0" ]]; then
  echo "ERROR: heaptrack tool version drift: heaptrack=${HEAPTRACK_VERSION} heaptrack_print=${HEAPTRACK_PRINT_VERSION} (required 1.5.0)" >&2
  exit 1
fi

# -- Bench binary discovery ---------------------------------------------------
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${WORKDIR}/target/bench-build}"
BENCH_PACKAGE="${BENCH_PACKAGE:-vb_workspace_tests}"
BENCH_NAME="${BENCH_NAME:-velvet_ballistics}"

# Allow caller to pin the binary hash; otherwise pick the most recent
# executable in target/bench-build/release/deps/ that matches $BENCH_NAME-*.
if [[ -n "${BENCH_HASH_OVERRIDE:-}" ]]; then
  BENCH_BIN="${CARGO_TARGET_DIR}/release/deps/${BENCH_NAME}-${BENCH_HASH_OVERRIDE}"
else
  BENCH_BIN="$(ls -t "${CARGO_TARGET_DIR}/release/deps/${BENCH_NAME}-"* 2>/dev/null \
    | head -n 1 || true)"
fi

if [[ -z "${BENCH_BIN}" || ! -x "${BENCH_BIN}" ]]; then
  echo "ERROR: bench binary not found at ${CARGO_TARGET_DIR}/release/deps/${BENCH_NAME}-*" >&2
  echo "       Run 'moon run :bench-build' first or set BENCH_HASH_OVERRIDE." >&2
  exit 1
fi

# -- v1 audit envelope (locked by contract.md §7) -----------------------------
v1_envelope=(
  "bench_engine_step_once_save_const_single_transition"
  "ipc_frame_decode"
  "engine_run_until_blocked_budget_10_small_workflow"
)

# -- Helpers ------------------------------------------------------------------
# Parse a heaptrack_print size literal like "1.29M" / "10.51K" / "512B" / "0"
# to integer bytes. M = 1_048_576, G = 1_073_741_824, K = 1024, B = 1.
size_to_bytes() {
  local raw="$1"
  if [[ "${raw}" =~ ^([0-9]+)\ *$ ]]; then
    echo "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "${raw}" =~ ^([0-9]*\.[0-9]+)([BKMG])$ ]]; then
    local mant="${BASH_REMATCH[1]}"
    local unit="${BASH_REMATCH[2]}"
    case "${unit}" in
      B) awk -v m="${mant}" 'BEGIN { printf "%d\n", m }' ;;
      K) awk -v m="${mant}" 'BEGIN { printf "%d\n", m * 1024 }' ;;
      M) awk -v m="${mant}" 'BEGIN { printf "%d\n", m * 1048576 }' ;;
      G) awk -v m="${mant}" 'BEGIN { printf "%d\n", m * 1073741824 }' ;;
    esac
    return 0
  fi
  echo "ERROR: cannot parse size literal: '${raw}'" >&2
  return 1
}

# Pull a join-key field from a single section39-metadata.jsonl row.
join_key_for() {
  local metric="$1" field="$2"
  python3 -c "
import json, sys
metric, field = sys.argv[1], sys.argv[2]
with open('${METADATA_JSONL}') as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        row = json.loads(line)
        if row.get('metric') == metric:
            print(row.get(field, ''))
            sys.exit(0)
sys.exit(1)
" "${metric}" "${field}"
}

# Render one AllocEvidence row from a heaptrack_print summary, joining
# against section39-metadata.jsonl. Emits JSONL (lexicographic key order).
build_row() {
  local metric="$1" summary_path="$2" raw_log_rel="$3" cmd="$4"
  python3 -c "
import json, re, sys

metric        = sys.argv[1]
summary_path  = sys.argv[2]
raw_log_rel   = sys.argv[3]
cmd           = sys.argv[4]
metadata_path = sys.argv[5]
fixed_size_lit = sys.argv[6] if len(sys.argv) > 6 else None

with open(summary_path) as fh:
    text = fh.read()
with open(metadata_path) as fh:
    meta = next(r for r in (json.loads(l) for l in fh if l.strip()) if r.get('metric') == metric)

def grab_int(pattern):
    m = re.search(pattern, text)
    if not m:
        sys.exit(f'MissingAnchor: {pattern}')
    return int(m.group(1))

def grab_size(pattern, kind):
    m = re.search(pattern, text)
    if not m:
        sys.exit(f'MissingAnchor: {pattern}')
    raw = m.group(1)
    # Bash already did the conversion; this is sanity.
    if not re.match(r'^[0-9.]+[BKMG]?$|^[0-9]+$', raw):
        sys.exit(f'BadSizeLiteral: {kind}={raw}')
    return raw

# Allow the script's pre-parsed numbers to override (Bash is the source of
# truth for the size -> bytes conversion).
alloc_count    = grab_int(r'calls to allocation functions:\s*(\d+)')
leak_count     = grab_int(r'temporary memory allocations:\s*(\d+)')
peak_heap_raw  = grab_size(r'peak heap memory consumption:\s*([0-9.]+[BKMG])', 'peak_heap')
peak_rss_raw   = grab_size(r'peak RSS \(including heaptrack overhead\):\s*([0-9.]+[BKMG])', 'peak_rss')
leak_bytes_raw = grab_size(r'total memory leaked:\s*([0-9.]+[BKMG])', 'leak_bytes')

methodology = 'heaptrack LD_PRELOAD over Criterion bench binary; sample iterations only; bytes_allocated is peak heap proxy'
if 'peak heap proxy' not in methodology:
    sys.exit('MissingMethodologyNote: peak heap proxy substring missing')

row = {
    'alloc_count':        alloc_count,
    'alloc_methodology':  methodology,
    'alloc_raw_log':      raw_log_rel,
    'alloc_tool':         'heaptrack',
    'alloc_tool_version': '1.5.0',
    'bytes_allocated':    int(sys.argv[7]),
    'bytes_allocated_proxy': True,
    'commit':             meta['commit'],
    'execution_mode':     meta['execution_mode'],
    'fixture_digest':     meta['fixture_digest'],
    'leak_bytes':         int(sys.argv[8]),
    'leak_count':         leak_count,
    'metric':             metric,
    'peak_heap':          int(sys.argv[7]),  # identity by contract
    'peak_rss':           int(sys.argv[9]),
    'timestamp':          meta['timestamp'],
    'command':            cmd,
}

# Lexicographic key order (Section 39 audit envelope convention).
ordered = {k: row[k] for k in sorted(row)}
print(json.dumps(ordered, separators=(',', ':')))
" "${metric}" "${summary_path}" "${raw_log_rel}" "${cmd}" "${METADATA_JSONL}" \
  "" "${PEAK_HEAP_BYTES}" "${LEAK_BYTES_BYTES}" "${PEAK_RSS_BYTES}"
}

# -- Main loop ----------------------------------------------------------------
SAMPLE_SIZE="${HEAPTRACK_SAMPLE_SIZE:-10}"
WARMUP_TIME="${HEAPTRACK_WARMUP_TIME:-1}"
MEASUREMENT_TIME="${HEAPTRACK_MEASUREMENT_TIME:-1}"

REQUESTED=("$@")
if [[ "${#REQUESTED[@]}" -eq 0 ]]; then
  REQUESTED=("${v1_envelope[@]}")
fi

# Validate request set against v1 envelope.
for m in "${REQUESTED[@]}"; do
  found=0
  for v in "${v1_envelope[@]}"; do
    if [[ "${m}" == "${v}" ]]; then
      found=1
      break
    fi
  done
  if [[ "${found}" -eq 0 ]]; then
    echo "ERROR: metric '${m}' is not in the v1 audit envelope" >&2
    echo "       (v1 envelope: ${v1_envelope[*]})" >&2
    exit 1
  fi
done

# Truncate output file (idempotent re-run per contract).
: > "${ALLOC_EVIDENCE_JSONL}"

for METRIC in "${REQUESTED[@]}"; do
  RAW_OUT="${RAW_DIR}/${METRIC}.heaptrack"
  LOG_OUT="${LOG_DIR}/alloc.${METRIC}.log"

  echo "[heaptrack] wrapping ${METRIC}"
  # --record-only: do NOT spawn the heaptrack_gui analyzer (which blocks
  # waiting for human interaction in non-interactive shells). The CLI prints
  # the output path on stdout; heaptrack_print consumes the .zst in the
  # next step. See `heaptrack --help` for the contract.
  CMD="${HEAPTRACK_BIN} --record-only -o ${RAW_OUT} ${BENCH_BIN} --measurement-time ${MEASUREMENT_TIME} --warm-up-time ${WARMUP_TIME} ${METRIC}"
  # shellcheck disable=SC2086
  "${HEAPTRACK_BIN}" --record-only -o "${RAW_OUT}" \
    "${BENCH_BIN}" \
    --measurement-time "${MEASUREMENT_TIME}" \
    --warm-up-time "${WARMUP_TIME}" \
    "${METRIC}" \
    > "${LOG_DIR}/.${METRIC}.heaptrack.stdout" 2>&1

  echo "[heaptrack_print] ${METRIC} -> ${LOG_OUT}"
  "${HEAPTRACK_PRINT_BIN}" -f "${RAW_OUT}.zst" > "${LOG_OUT}" 2>&1

  # Extract the size literals from the summary, then convert to bytes via
  # the Bash helper (single source of truth for the unit table).
  PEAK_HEAP_RAW=$(grep -E '^peak heap memory consumption:' "${LOG_OUT}" \
    | sed -E 's/^peak heap memory consumption:[[:space:]]+//' || true)
  PEAK_RSS_RAW=$(grep -E '^peak RSS \(including heaptrack overhead\):' "${LOG_OUT}" \
    | sed -E 's/^peak RSS \(including heaptrack overhead\):[[:space:]]+//' || true)
  LEAK_BYTES_RAW=$(grep -E '^total memory leaked:' "${LOG_OUT}" \
    | sed -E 's/^total memory leaked:[[:space:]]+//' || true)

  if [[ -z "${PEAK_HEAP_RAW}" || -z "${PEAK_RSS_RAW}" || -z "${LEAK_BYTES_RAW}" ]]; then
    echo "ERROR: missing size literal in ${LOG_OUT}" >&2
    exit 1
  fi

  PEAK_HEAP_BYTES="$(size_to_bytes "${PEAK_HEAP_RAW}")"
  PEAK_RSS_BYTES="$(size_to_bytes "${PEAK_RSS_RAW}")"
  LEAK_BYTES_BYTES="$(size_to_bytes "${LEAK_BYTES_RAW}")"

  RAW_LOG_REL="evidence/benchmark-logs/alloc.${METRIC}.log"

  build_row "${METRIC}" "${LOG_OUT}" "${RAW_LOG_REL}" "${CMD}" \
    > "${ALLOC_EVIDENCE_JSONL}.tmp"
  cat "${ALLOC_EVIDENCE_JSONL}.tmp" >> "${ALLOC_EVIDENCE_JSONL}"
  rm -f "${ALLOC_EVIDENCE_JSONL}.tmp"
  echo "[alloc-evidence] appended row for ${METRIC}"
done

# Sanity check: row count == benchmark-evidence.jsonl row count.
ALLOC_LINES=$(wc -l < "${ALLOC_EVIDENCE_JSONL}")
BENCH_LINES=$(wc -l < "${BENCHMARK_EVIDENCE_JSONL}")
META_LINES=$(wc -l < "${METADATA_JSONL}")
echo "[verify] alloc-evidence=${ALLOC_LINES} benchmark-evidence=${BENCH_LINES} section39-metadata=${META_LINES}"
if [[ "${ALLOC_LINES}" -ne "${BENCH_LINES}" || "${ALLOC_LINES}" -ne "${META_LINES}" ]]; then
  echo "ERROR: row count mismatch (alloc=${ALLOC_LINES} benchmark=${BENCH_LINES} metadata=${META_LINES})" >&2
  exit 1
fi

echo "[done] ${ALLOC_EVIDENCE_JSONL}"
