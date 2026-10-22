#!/usr/bin/env python3
"""vb-j3e3s benchmark evidence collector (fail-closed, real tool formats).

Produces exactly the accepted schema demonstrated by the three original
records in evidence/benchmark-evidence.jsonl.  Every required field is
mandatory; nothing is None/unknown/0-faked.  The collector:

* reads the budgeted benchmark names + thresholds from contracts/perf-budget.yaml
* discovers the bench executable from
  `cargo bench -p velvet-ballistics-workspace-tests --bench velvet_ballistics
   --no-run --message-format=json` (or verifies a supplied --binary)
* for each benchmark runs criterion + perf stat + heaptrack (heaptrack 1.5),
  each with its own unique CRITERION_HOME so runs never overwrite each other's
  timing data
* decodes the heaptrack .zst trace (v3) for exact allocation count, exact
  total allocated bytes and exact peak heap bytes
* captures actual tool stdout/stderr byte-for-byte into a per-benchmark raw log
* rejects forbidden placeholder markers, stale commits, dirty inputs, bad
  samples and unparseable counters instead of inventing values
* publishes nothing unless all three benchmarks succeed; old evidence is
  preserved byte-exact under .beads/vb-j3e3s/history first, then the canonical
  evidence JSONL and raw logs are replaced atomically.

Only ROOT executes this against the final frozen source.  The product author
must remove the forbidden BENCH_METADATA placeholders before a real run can
pass the forbidden-marker gate (see FORBIDDEN_MARKERS).
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

RUN_GATE = "/tmp/vb-run-gate.py"
GIT = "/usr/bin/git" if Path("/usr/bin/git").is_file() else "git"

CRITERION_FLAGS = ["--bench", "--sample-size", "10", "--warm-up-time", "1",
                   "--measurement-time", "1"]

BENCH_SOURCE_RELPATH = "crates/workspace_tests/benches/velvet_ballistics.rs"
EVIDENCE_JSONL_RELPATH = "evidence/benchmark-evidence.jsonl"
RAW_LOG_DIR_RELPATH = "evidence/benchmark-logs"
HISTORY_DIR_RELPATH = ".beads/vb-j3e3s/history"

# Mirrors xtask/src/benchmark_policy.rs FORBIDDEN_EVIDENCE_MARKERS.  A
# criterion full_id currently carries these (BENCH_METADATA in the bench
# source); the collector must reject them rather than publish placeholder
# metadata.
FORBIDDEN_MARKERS = [
    "instructions=not-collected",
    "instruction_count=not-collected",
    "allocations=allocator-external",
    "allocation_count=allocator-external",
]

# Truthful per-benchmark descriptors (match the accepted original schema).
BENCH_META = {
    "bench_engine_step_once_save_const_single_transition": {
        "claim": "Current HEAD IR interpreter baseline snapshot; baseline_value equals result_value intentionally because no speedup claim is made",
        "durability_profile": "direct in-memory runtime core; no fjall durability",
        "fjall_write_latency": "not applicable to runtime_core direct benchmark",
        "direct_api_is_mean": True,
        "ipc_is_mean": False,
    },
    "ipc_frame_decode": {
        "claim": "Current HEAD IR interpreter IPC decode baseline snapshot; baseline_value equals result_value intentionally because no speedup claim is made",
        "durability_profile": "mixed IPC frame decode benchmark; no fjall write durability",
        "fjall_write_latency": "not applicable to ipc_frame_decode",
        "direct_api_is_mean": False,
        "ipc_is_mean": True,
    },
    "engine_run_until_blocked_budget_10_small_workflow": {
        "claim": "Current HEAD IR interpreter small-workflow baseline snapshot; baseline_value equals result_value intentionally because no speedup claim is made",
        "durability_profile": "direct in-memory runtime core; no fjall durability",
        "fjall_write_latency": "not applicable to runtime_core direct benchmark",
        "direct_api_is_mean": True,
        "ipc_is_mean": False,
    },
}

SI_MULTIPLIERS = {"B": 1, "K": 10 ** 3, "M": 10 ** 6, "G": 10 ** 9,
                  "T": 10 ** 12}


class CollectorError(RuntimeError):
    """A fail-closed collection failure with a clear, non-invented reason."""


# ---------------------------------------------------------------------------
# production parsers / validators (imported and exercised by the test suite)
# ---------------------------------------------------------------------------

def parse_budget(text):
    """Return {benchmark_name: max_regression_percent} from perf-budget.yaml."""
    if "benchmarks:" not in text:
        raise CollectorError("perf-budget.yaml has no benchmarks: section")
    benches = {}
    in_section = False
    for raw in text.splitlines():
        if raw.strip() == "benchmarks:":
            in_section = True
            continue
        if in_section:
            if raw.startswith("  ") and not raw.startswith("    "):
                name = raw.strip().rstrip(":")
                if name and ":" in raw:
                    benches[name] = None
            elif raw.startswith("    max_regression_percent:"):
                # belongs to the most recent 2-space name
                if not benches:
                    raise CollectorError("perf-budget.yaml threshold before benchmark name")
                last = list(benches.keys())[-1]
                benches[last] = int(raw.split(":", 1)[1].strip())
            elif raw.strip() and not raw.startswith(" "):
                in_section = False
    missing = [k for k, v in benches.items() if v is None]
    if missing:
        raise CollectorError("benchmarks missing max_regression_percent: " + ", ".join(missing))
    return benches


def check_no_forbidden_markers(text, label):
    for marker in FORBIDDEN_MARKERS:
        if marker in text:
            raise CollectorError(
                f"{label} contains forbidden placeholder metadata {marker!r}; "
                f"product author must remove it from BENCH_METADATA before collection"
            )


def metric_matches(full_id, metric):
    """Strict full_id -> budgeted metric match (full_id == group_id/function_id)."""
    if full_id == metric:
        return True
    func = full_id.rsplit("/", 1)[-1]
    return func == metric or func.startswith(metric + ";")


def percentile(sorted_values, p):
    """Linear-interpolation percentile (matches original accepted schema)."""
    if not sorted_values:
        raise CollectorError("cannot take percentile of empty sample")
    idx = p / 100.0 * (len(sorted_values) - 1)
    lo = int(idx)
    frac = idx - lo
    if lo + 1 < len(sorted_values):
        return sorted_values[lo] + frac * (sorted_values[lo + 1] - sorted_values[lo])
    return sorted_values[lo]


def _finite_positive(x):
    try:
        v = float(x)
    except (TypeError, ValueError):
        return False
    return v == v and v not in (float("inf"), float("-inf")) and v > 0


def parse_criterion_sample(sample_json, min_samples=10):
    """Validate and derive per-iteration times (ns) from sample.json.

    Returns (sorted per-iteration ns list, sample_count).  Rejects unequal
    lengths, < min_samples entries, and any non-finite/non-positive iters/times.
    """
    iters = sample_json.get("iters")
    times = sample_json.get("times")
    if not isinstance(iters, list) or not isinstance(times, list):
        raise CollectorError("sample.json missing iters/times lists")
    if len(iters) != len(times):
        raise CollectorError(
            f"sample.json iters/times length mismatch: {len(iters)} vs {len(times)}"
        )
    if len(times) < min_samples:
        raise CollectorError(f"sample.json has {len(times)} samples, need >= {min_samples}")
    if any(not _finite_positive(x) for x in iters):
        raise CollectorError("sample.json has non-finite/non-positive iters")
    if any(not _finite_positive(x) for x in times):
        raise CollectorError("sample.json has non-finite/non-positive times")
    per_iter = [times[i] / iters[i] for i in range(len(times))]
    per_iter.sort()
    return per_iter, len(times)


def parse_criterion_estimates(estimates_json):
    """Return the mean point estimate (ns) from estimates.json."""
    mean = estimates_json.get("mean")
    if not isinstance(mean, dict):
        raise CollectorError("estimates.json missing mean estimate")
    point = mean.get("point_estimate")
    if not _finite_positive(point):
        raise CollectorError("estimates.json mean point_estimate not finite/positive")
    return float(point)


def parse_perf_csv(content):
    """Return the whole-process instruction count from perf stat -x ';' CSV.

    Format: `count;;instructions:u;runtime;percent;;`.  Requires a positive
    integer; anything else (missing, zero, cycles-only, <not counted>) fails.
    """
    for raw in content.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if ";;" not in line:
            continue
        parts = line.split(";;")
        if len(parts) < 2:
            continue
        try:
            count = int(parts[0].strip())
        except ValueError:
            continue
        inner = parts[1].split(";")
        if not inner:
            continue
        event = inner[0].strip()
        if event.split(":")[0] != "instructions":
            continue
        if count <= 0:
            raise CollectorError(f"perf instruction count not positive: {count}")
        return count
    raise CollectorError("could not parse positive instruction count from perf CSV")


def parse_byte_quantity(text):
    """Parse an SI byte quantity like '73.73K' / '8.04M' into integer bytes."""
    m = re.match(r"^([0-9]+(?:\.[0-9]+)?)\s*([KMGTB]?)$", text.strip())
    if not m:
        raise CollectorError(f"unparseable byte quantity: {text!r}")
    number = float(m.group(1))
    unit = m.group(2) or "B"
    if unit not in SI_MULTIPLIERS:
        raise CollectorError(f"unknown byte unit in {text!r}")
    return int(round(number * SI_MULTIPLIERS[unit]))


def parse_heaptrack_text(text):
    """Return (allocation_count, peak_byte_quantity_string) from heaptrack_print."""
    allocs = None
    peak = None
    for line in text.splitlines():
        m = re.match(r"^calls to allocation functions:\s+([0-9]+)", line)
        if m:
            allocs = int(m.group(1))
        m = re.match(r"^peak heap memory consumption:\s+(\S+)", line)
        if m:
            peak = m.group(1)
    if allocs is None:
        raise CollectorError("heaptrack_print did not report allocation count")
    if peak is None:
        raise CollectorError("heaptrack_print did not report peak heap memory")
    return allocs, peak


def parse_heaptrack_trace(text):
    """Decode heaptrack v3 .zst trace into exact counts.

    Returns (allocation_count, total_allocated_bytes, peak_bytes).

    v3 stream records (space separated):
      `v <version> <major>`    version header (major must be 3)
      `a <size_hex> <trace>`   push allocation-info (size in hex) to the table
      `+ <index>`              allocation: zero-based allocation-info lookup,
                               increments allocation count and live heap
      `- <index>`              deallocation: zero-based allocation-info lookup
    All other record types (s/i/t/X/I/c/R) are metadata and are skipped.
    Unknown format / out-of-range index / truncation fail closed.
    """
    sizes = []
    major = None
    live = 0
    peak = 0
    total = 0
    count = 0
    for lineno, raw in enumerate(text.splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        tok = line.split()
        op = tok[0]
        if op == "v":
            if len(tok) < 3:
                raise CollectorError(f"truncated version header at line {lineno}")
            try:
                major = int(tok[2])
            except ValueError:
                raise CollectorError(f"bad version header at line {lineno}: {raw!r}")
            if major != 3:
                raise CollectorError(f"unknown heaptrack format major {major}")
        elif op == "a":
            if len(tok) < 2:
                raise CollectorError(f"truncated allocation record at line {lineno}: {raw!r}")
            try:
                sizes.append(int(tok[1], 16))
            except ValueError:
                raise CollectorError(f"bad allocation size at line {lineno}: {raw!r}")
        elif op in ("+", "-"):
            if len(tok) < 2:
                raise CollectorError(f"truncated event record at line {lineno}: {raw!r}")
            try:
                idx = int(tok[1])
            except ValueError:
                raise CollectorError(f"bad event index at line {lineno}: {raw!r}")
            if idx < 0 or idx >= len(sizes):
                raise CollectorError(
                    f"out-of-range allocation-info index {idx} at line {lineno}"
                )
            size = sizes[idx]
            if op == "+":
                count += 1
                total += size
                live += size
                if live > peak:
                    peak = live
            else:
                live -= size
                if live < 0:
                    raise CollectorError(f"negative live heap at line {lineno}")
        # else: s / i / t / X / I / c / R metadata records are ignored
    if major is None:
        raise CollectorError("heaptrack trace missing v3 version header")
    if count == 0:
        raise CollectorError("heaptrack trace recorded zero allocations")
    return count, total, peak


def parse_rust_toolchain(text):
    m = re.search(r'^channel\s*=\s*"nightly-([0-9]{4}-[0-9]{2}-[0-9]{2})"',
                  text, re.MULTILINE)
    if not m:
        raise CollectorError("rust-toolchain.toml has no pinned nightly channel")
    return m.group(1)


def parse_cargo_criterion_version(cargo_lock_text):
    """Extract the locked criterion version from Cargo.lock text."""
    m = re.search(r'name = "criterion"\nversion = "([0-9.]+)"', cargo_lock_text)
    if not m:
        raise CollectorError("Cargo.lock has no criterion version")
    return m.group(1)


# ---------------------------------------------------------------------------
# command execution through the official run gate
# ---------------------------------------------------------------------------

def run_gated(seconds, prefix, cmd_args, cwd=None):
    """Run `cmd_args` via /tmp/vb-run-gate.py; return (stdout, stderr, meta)."""
    if not Path(RUN_GATE).is_file():
        raise CollectorError(f"run gate missing: {RUN_GATE}")
    Path(prefix).parent.mkdir(parents=True, exist_ok=True)
    full = [sys.executable, RUN_GATE, str(seconds), prefix, *cmd_args]
    r = subprocess.run(full, cwd=cwd)
    out_p = Path(prefix + ".stdout")
    err_p = Path(prefix + ".stderr")
    json_p = Path(prefix + ".json")
    stdout = out_p.read_bytes() if out_p.is_file() else b""
    stderr = err_p.read_bytes() if err_p.is_file() else b""
    meta = json.loads(json_p.read_text()) if json_p.is_file() else {}
    code = meta.get("exit_code", r.returncode)
    if code != 0:
        raise CollectorError(
            f"command failed (exit {code}): {' '.join(cmd_args)}\n"
            f"stderr:\n{stderr.decode('utf-8', 'replace')[:2000]}"
        )
    return stdout, stderr, meta


def _env_cmd(extra, cmd_args):
    return ["/usr/bin/env", *[f"{k}={v}" for k, v in extra.items()], *cmd_args]


# ---------------------------------------------------------------------------
# metadata / environment collection
# ---------------------------------------------------------------------------

def run_plain(cmd, cwd=None):
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if r.returncode != 0:
        raise CollectorError(f"command failed (exit {r.returncode}): {' '.join(cmd)}")
    return r.stdout


def system_metadata(root):
    kernel = run_plain(["uname", "-a"]).strip()
    cpu = "unknown"
    for line in Path("/proc/cpuinfo").read_text().splitlines():
        if line.lower().startswith("model name"):
            cpu = line.split(":", 1)[1].strip()
            break
    gov_path = Path("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
    gov = gov_path.read_text().strip() if gov_path.is_file() else "unknown"
    rustc = run_plain(["rustc", "--version"], cwd=str(root)).strip()
    toolchain_text = Path(root, "rust-toolchain.toml").read_text()
    nightly = parse_rust_toolchain(toolchain_text)
    lock_text = Path(root, "Cargo.lock").read_text()
    criterion_version = parse_cargo_criterion_version(lock_text)
    rustflags = os.environ.get("RUSTFLAGS", "").strip()
    return {
        "kernel_version": kernel,
        "cpu_model": cpu,
        "cpu_governor": gov,
        "rustc_version": rustc,
        "nightly_date": nightly,
        "criterion_version": criterion_version,
        "build_profile": "bench profile optimized",
        "rustflags": rustflags if rustflags else "default bench invocation; no RUSTFLAGS override",
    }


# ---------------------------------------------------------------------------
# commit / cleanliness checks
# ---------------------------------------------------------------------------

def _git(args, root):
    r = subprocess.run([GIT, *args], cwd=str(root), capture_output=True, text=True)
    if r.returncode != 0:
        raise CollectorError(f"git {' '.join(args)} failed: {r.stderr.strip()}")
    return r.stdout


def current_head(root):
    return _git(["rev-parse", "HEAD"], root).strip()


def check_commit(root, expected):
    head = current_head(root)
    if not re.fullmatch(r"[0-9a-f]{40}", head):
        raise CollectorError(f"HEAD is not a 40-hex sha: {head!r}")
    if not re.fullmatch(r"[0-9a-f]{40}", expected):
        raise CollectorError(f"--commit is not a 40-hex sha: {expected!r}")
    if head != expected:
        raise CollectorError(f"commit mismatch: expected {expected}, HEAD {head}")
    return head


# directories whose tracked/untracked changes are permitted (bead + evidence + docs)
_ALLOWED_DIRS = ("evidence/", ".beads/", "docs/", "reports/", "design/", "target/")


def check_clean(root):
    out = _git(["status", "--porcelain", "--untracked-files=all"], root)
    dirty = []
    for raw in out.splitlines():
        line = raw.rstrip("\n")
        if not line:
            continue
        code = line[:2]
        path = line[3:].strip()
        if code == "??":
            if not any(path == d or path.startswith(d) for d in _ALLOWED_DIRS):
                dirty.append(f"untracked:{path}")
        else:
            if not any(path == d or path.startswith(d) for d in _ALLOWED_DIRS):
                dirty.append(f"{code}:{path}")
    if dirty:
        raise CollectorError("dirty benchmark/source/toolchain/Cargo/config/budget inputs: "
                             + ", ".join(dirty[:20]))


# ---------------------------------------------------------------------------
# binary discovery
# ---------------------------------------------------------------------------

def find_binary(root, scratch):
    prefix = str(Path(scratch) / "gate" / f"cargo-build-{time.time_ns()}")
    stdout, _stderr, _meta = run_gated(
        1800, prefix,
        ["cargo", "bench", "-p", "velvet-ballistics-workspace-tests",
         "--bench", "velvet_ballistics", "--no-run", "--message-format=json",
         "--locked"],
        cwd=str(root),
    )
    for line in stdout.decode("utf-8", "replace").splitlines():
        try:
            obj = json.loads(line)
        except ValueError:
            continue
        if obj.get("reason") != "compiler-artifact":
            continue
        kind = obj.get("target", {}).get("kind", [])
        if "bench" not in kind:
            continue
        exe = obj.get("executable") or (obj.get("filenames") or [None])[0]
        if exe and Path(exe).is_file():
            return os.path.relpath(exe, str(root))
    raise CollectorError("could not discover bench executable from cargo --no-run output")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


# ---------------------------------------------------------------------------
# measurement stages
# ---------------------------------------------------------------------------

def criterion_run(binary, metric, root, criterion_home, prefix):
    home = Path(criterion_home)
    home.mkdir(parents=True, exist_ok=False)
    cmd = [binary, metric, *CRITERION_FLAGS]
    stdout, stderr, _meta = run_gated(
        600, prefix, _env_cmd({"CRITERION_HOME": str(home), "LC_ALL": "C"}, cmd),
        cwd=str(root),
    )
    # locate benchmark.json whose full_id matches the metric, only under the
    # unique CRITERION_HOME (no target/criterion fallback)
    match_dir = None
    full_id = None
    for bj in home.rglob("benchmark.json"):
        try:
            data = json.loads(bj.read_text())
        except (ValueError, OSError):
            continue
        fid = data.get("full_id", "")
        if metric_matches(fid, metric):
            match_dir = bj.parent
            full_id = fid
            break
    if match_dir is None:
        raise CollectorError(f"no criterion benchmark.json for {metric} under {home}")
    check_no_forbidden_markers(full_id, f"criterion full_id for {metric}")

    estimates = json.loads((match_dir / "estimates.json").read_text())
    sample = json.loads((match_dir / "sample.json").read_text())
    mean_ns = parse_criterion_estimates(estimates)
    per_iter_ns, sample_count = parse_criterion_sample(sample)
    p50 = percentile(per_iter_ns, 50)
    p95 = percentile(per_iter_ns, 95)
    p99 = percentile(per_iter_ns, 99)
    return {
        "mean_ns": mean_ns,
        "sample_count": sample_count,
        "p50_ns": p50,
        "p95_ns": p95,
        "p99_ns": p99,
        "stdout": stdout,
        "stderr": stderr,
    }


def perf_run(binary, metric, root, criterion_home, perf_csv, prefix):
    Path(criterion_home).mkdir(parents=True, exist_ok=False)
    cmd = ["perf", "stat", "-x", ";", "-e", "instructions",
           "-o", str(perf_csv), "--", binary, metric, *CRITERION_FLAGS]
    stdout, stderr, _meta = run_gated(
        600, prefix, _env_cmd({"CRITERION_HOME": str(criterion_home), "LC_ALL": "C"}, cmd),
        cwd=str(root),
    )
    content = Path(perf_csv).read_text()
    instruction_count = parse_perf_csv(content)
    return {"instruction_count": instruction_count, "stdout": stdout, "stderr": stderr}


def heaptrack_run(binary, metric, root, criterion_home, ht_base, prefix):
    Path(criterion_home).mkdir(parents=True, exist_ok=False)
    record_prefix = prefix + "-record"
    record_cmd = ["heaptrack", "--record-only", "-o", str(ht_base),
                  binary, metric, *CRITERION_FLAGS]
    rec_out, rec_err, _meta = run_gated(
        600, record_prefix,
        _env_cmd({"CRITERION_HOME": str(criterion_home), "LC_ALL": "C"}, record_cmd),
        cwd=str(root),
    )
    zst = Path(str(ht_base) + ".zst")
    if not zst.is_file():
        raise CollectorError(f"heaptrack .zst not found: {zst}")

    print_prefix = prefix + "-print"
    print_cmd = ["heaptrack_print", str(zst)]
    print_out, print_err, _meta = run_gated(300, print_prefix, print_cmd, cwd=str(root))

    zstd_prefix = prefix + "-zstd"
    zstd_cmd = ["zstd", "-dc", str(zst)]
    zstd_out, zstd_err, _meta = run_gated(120, zstd_prefix, zstd_cmd, cwd=str(root))

    trace_count, _total_bytes, peak_bytes = parse_heaptrack_trace(
        zstd_out.decode("utf-8", "replace")
    )
    text_count, peak_str = parse_heaptrack_text(print_out.decode("utf-8", "replace"))
    if text_count != trace_count:
        raise CollectorError(
            f"heaptrack count mismatch: print={text_count} trace={trace_count}"
        )
    _ = parse_byte_quantity(peak_str)  # peak_str must be a valid positive quantity

    return {
        "allocation_count": trace_count,
        "bytes_allocated": peak_bytes,
        "record_stdout": rec_out,
        "record_stderr": rec_err,
        "print_stdout": print_out,
        "print_stderr": print_err,
        "zst": zst,
    }


def fixture_digest(root):
    """sha256 of the bench source fixture (plus any include!/mod helpers it pulls in)."""
    bench = Path(root, BENCH_SOURCE_RELPATH)
    if not bench.is_file():
        raise CollectorError(f"bench source missing: {bench}")
    files = [bench]
    src = bench.read_text()
    for m in re.finditer(r'include!\s*\(\s*"([^"]+)"', src):
        p = bench.parent / m.group(1)
        if p.is_file() and p not in files:
            files.append(p)
    for m in re.finditer(r'#\[path\s*=\s*"([^"]+)"\]\s*mod\s+[A-Za-z0-9_]+', src):
        p = bench.parent / m.group(1)
        if p.is_file() and p not in files:
            files.append(p)
    h = hashlib.sha256()
    for f in sorted(files, key=str):
        h.update(f.read_bytes())
    return f"sha256:{h.hexdigest()} {BENCH_SOURCE_RELPATH}"


# ---------------------------------------------------------------------------
# record assembly
# ---------------------------------------------------------------------------

def build_record(metric, threshold, mean_ns, sample_count, p50, p95, p99,
                 instruction_count, allocation_count, bytes_allocated,
                 commit, meta, binary_relpath, digest, raw_log_relpath):
    meta_entry = BENCH_META[metric]
    mean_str = f"{mean_ns:.3f}"
    record = {
        "metric": metric,
        "claim": meta_entry["claim"],
        "baseline_value": int(round(mean_ns * 1000.0)),
        "result_value": int(round(mean_ns * 1000.0)),
        "unit": "picoseconds",
        "threshold_percent": threshold,
        "raw_log": raw_log_relpath,
        "command": f"{binary_relpath} {metric} " + " ".join(CRITERION_FLAGS),
        "commit": commit,
        "rustc_version": meta["rustc_version"],
        "nightly_date": meta["nightly_date"],
        "cpu_model": meta["cpu_model"],
        "cpu_governor": meta["cpu_governor"],
        "kernel_version": meta["kernel_version"],
        "build_profile": meta["build_profile"],
        "rustflags": meta["rustflags"],
        "benchmark_tool_version": f"criterion {meta['criterion_version']}",
        "sample_count_or_instruction_count":
            f"criterion_samples={sample_count}; perf_instructions={instruction_count}",
        "input_fixture_digest": digest,
        "durability_profile": meta_entry["durability_profile"],
        "mode": "ir-interpreter",
        "p50_latency": f"{p50:.6f} ns sample-derived",
        "p95_latency": f"{p95:.6f} ns sample-derived",
        "p99_latency": f"{p99:.6f} ns sample-derived",
        "instruction_count": instruction_count,
        "allocation_count": allocation_count,
        "bytes_allocated": bytes_allocated,
        "fjall_write_latency": meta_entry["fjall_write_latency"],
        "direct_api_latency":
            f"{mean_str} ns Criterion mean estimate" if meta_entry["direct_api_is_mean"]
            else "not applicable to ipc_frame_decode",
        "ipc_latency":
            f"{mean_str} ns Criterion mean estimate" if meta_entry["ipc_is_mean"]
            else "not applicable to runtime_core direct benchmark",
    }
    # every required field mandatory: reject any null / empty / placeholder value
    required = [
        "metric", "claim", "baseline_value", "result_value", "unit",
        "threshold_percent", "raw_log", "command", "commit", "rustc_version",
        "nightly_date", "cpu_model", "cpu_governor", "kernel_version",
        "build_profile", "rustflags", "benchmark_tool_version",
        "sample_count_or_instruction_count", "input_fixture_digest",
        "durability_profile", "mode", "p50_latency", "p95_latency", "p99_latency",
        "instruction_count", "allocation_count", "bytes_allocated",
        "fjall_write_latency", "direct_api_latency", "ipc_latency",
    ]
    for field in required:
        value = record.get(field)
        if value is None or (isinstance(value, str) and not value.strip()):
            raise CollectorError(f"record field {field!r} is empty/missing")
    line = json.dumps(record) + "\n"
    check_no_forbidden_markers(line, f"evidence record for {metric}")
    return line


# ---------------------------------------------------------------------------
# publication
# ---------------------------------------------------------------------------

def atomic_write(path, data_bytes):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=str(path.parent), prefix=path.name + ".tmp-")
    try:
        with os.fdopen(fd, "wb") as f:
            f.write(data_bytes)
        os.replace(tmp, str(path))
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


def preserve_history(root, ts):
    hist = Path(root, HISTORY_DIR_RELPATH, ts)
    hist.mkdir(parents=True, exist_ok=True)
    evidence = Path(root, EVIDENCE_JSONL_RELPATH)
    if evidence.is_file():
        shutil.copyfile(evidence, hist / evidence.name)
    raw_dir = Path(root, RAW_LOG_DIR_RELPATH)
    if raw_dir.is_dir():
        for f in raw_dir.iterdir():
            if f.is_file():
                shutil.copyfile(f, hist / f.name)


def publish(root, records, raw_logs):
    """Preserve old evidence byte-exact, then atomically replace JSONL + logs."""
    ts = time.strftime("%Y%m%dT%H%M%S", time.gmtime())
    preserve_history(root, ts)
    atomic_write(Path(root, EVIDENCE_JSONL_RELPATH),
                 "".join(records).encode("utf-8"))
    raw_dir = Path(root, RAW_LOG_DIR_RELPATH)
    raw_dir.mkdir(parents=True, exist_ok=True)
    for metric, log_bytes in raw_logs.items():
        atomic_write(raw_dir / f"{metric}.log", log_bytes)


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------

def main(argv=None):
    parser = argparse.ArgumentParser(description="vb-j3e3s benchmark evidence collector")
    parser.add_argument("--root", required=True, help="repo root")
    parser.add_argument("--commit", required=True, help="frozen 40-hex commit SHA")
    parser.add_argument("--output-dir", required=True,
                        help="NEW scratch dir (created; refuses reuse)")
    parser.add_argument("--binary", help="optional prebuilt bench binary")
    args = parser.parse_args(argv)

    root = Path(args.root).resolve()
    output_dir = Path(args.output_dir).resolve()

    if output_dir.exists() and any(output_dir.iterdir()):
        raise CollectorError(f"output dir already exists and is not empty: {output_dir}")
    output_dir.mkdir(parents=True, exist_ok=True)

    commit = check_commit(root, args.commit)
    check_clean(root)

    budget = parse_budget(Path(root, "contracts", "perf-budget.yaml").read_text())
    meta = system_metadata(root)

    binary = args.binary
    if binary:
        if not Path(binary).is_file():
            raise CollectorError(f"--binary is not a file: {binary}")
        binary_relpath = os.path.relpath(Path(binary).resolve(), root)
    else:
        binary_relpath = find_binary(root, output_dir)
        binary = str(Path(root, binary_relpath))
    binary_sha = sha256_file(binary)

    digest = fixture_digest(root)

    scratch = output_dir / "scratch"
    scratch.mkdir(parents=True, exist_ok=True)

    records = []
    raw_logs = {}
    zst_manifest = {}

    for metric, threshold in budget.items():
        header = (
            f"# vb-j3e3s benchmark evidence raw log for {metric}\n"
            f"# binary: {binary_relpath} sha256:{binary_sha}\n"
            f"# commit: {commit}\n"
        ).encode("utf-8")

        crit_home = scratch / "criterion-home" / f"criterion-{metric}"
        crit = criterion_run(binary, metric, root, crit_home,
                             str(scratch / "gate" / f"{metric}-criterion-{time.time_ns()}"))

        perf_home = scratch / "criterion-home" / f"perf-{metric}"
        perf_csv = scratch / "perf" / f"{metric}.csv"
        perf_csv.parent.mkdir(parents=True, exist_ok=True)
        perf = perf_run(binary, metric, root, perf_home, perf_csv,
                        str(scratch / "gate" / f"{metric}-perf-{time.time_ns()}"))

        ht_home = scratch / "criterion-home" / f"heaptrack-{metric}"
        ht_base = scratch / "heaptrack" / f"ht-{metric}"
        ht_base.parent.mkdir(parents=True, exist_ok=True)
        ht = heaptrack_run(binary, metric, root, ht_home, ht_base,
                           str(scratch / "gate" / f"{metric}-heaptrack-{time.time_ns()}"))

        raw_log_bytes = b"".join([
            header,
            b"=== criterion stdout ===\n", crit["stdout"],
            b"=== criterion stderr ===\n", crit["stderr"],
            b"=== perf stdout ===\n", perf["stdout"],
            b"=== perf stderr ===\n", perf["stderr"],
            b"=== heaptrack stdout ===\n", ht["record_stdout"],
            b"=== heaptrack stderr ===\n", ht["record_stderr"],
            b"=== heaptrack_print stdout ===\n", ht["print_stdout"],
            b"=== heaptrack_print stderr ===\n", ht["print_stderr"],
        ])
        check_no_forbidden_markers(raw_log_bytes.decode("utf-8", "replace"),
                                   f"raw log for {metric}")

        raw_log_relpath = f"{RAW_LOG_DIR_RELPATH}/{metric}.log"
        record_line = build_record(
            metric, threshold, crit["mean_ns"], crit["sample_count"],
            crit["p50_ns"], crit["p95_ns"], crit["p99_ns"],
            perf["instruction_count"], ht["allocation_count"],
            ht["bytes_allocated"], commit, meta, binary_relpath, digest,
            raw_log_relpath,
        )
        records.append(record_line)
        raw_logs[metric] = raw_log_bytes
        zst_manifest[metric] = {
            "path": str(ht["zst"]),
            "sha256": sha256_file(ht["zst"]),
            "allocation_count": ht["allocation_count"],
            "bytes_allocated": ht["bytes_allocated"],
        }

    # re-verify source is still frozen before publishing
    check_commit(root, args.commit)
    check_clean(root)

    atomic_write(scratch / "heaptrack-manifest.json",
                 (json.dumps(zst_manifest, indent=2) + "\n").encode("utf-8"))

    publish(root, records, raw_logs)

    print(f"collected {len(records)} records")
    print(f"evidence: {EVIDENCE_JSONL_RELPATH}")
    print(f"heaptrack manifest: {scratch / 'heaptrack-manifest.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
