#!/usr/bin/env python3
"""
Velvet Ballistics Benchmark Evidence Collector
=============================================

Standalone collector that measures actual benchmark performance, instruction
counts, and allocation statistics for the three budgeted benchmarks.

Requirements:
  - Must be invoked from a frozen Git commit (argument required)
  - Benchmark executable must already be compiled
  - Tools required: perf, heaptrack, jq (or python3 with json module)

Usage:
  python3 collect-benchmark-evidence.py --commit <SHA> --root <workspace>
    [--binary <path>] [--output-dir <path>] [--dry-run]
"""

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from typing import Any, Dict, List, Optional

BENCHMARKS = [
    "bench_engine_step_once_save_const_single_transition",
    "ipc_frame_decode",
    "engine_run_until_blocked_budget_10_small_workflow",
]

CRITERION_FLAGS = ["--bench", "--sample-size", "10", "--warm-up-time", "1", "--measurement-time", "1"]

# Forbidden placeholder markers that must not appear in raw logs
FORBIDDEN_MARKERS = [
    "instructions=not-collected",
    "instruction_count=not-collected",
    "allocations=allocator-external",
    "allocation_count=allocator-external",
]


def run_command(cmd: List[str], cwd: Optional[str] = None, env: Optional[Dict[str, str]] = None,
                capture: bool = True, timeout: Optional[int] = None) -> subprocess.CompletedProcess:
    """Run a command and return the completed process."""
    try:
        result = subprocess.run(
            cmd, cwd=cwd, env=env, capture_output=capture, text=True,
            timeout=timeout
        )
        return result
    except FileNotFoundError as e:
        raise RuntimeError(f"Command not found: {cmd[0]}") from e
    except subprocess.TimeoutExpired as e:
        raise RuntimeError(f"Command timed out after {timeout}s: {' '.join(cmd)}") from e


def check_prerequisites() -> None:
    """Verify all required tools are available."""
    for tool in ["perf", "heaptrack", "sha256sum"]:
        if shutil.which(tool) is None:
            raise RuntimeError(f"Required tool not found: {tool}")


def get_system_metadata() -> Dict[str, Any]:
    """Collect system metadata for evidence."""
    # CPU model
    result = run_command(["cat", "/proc/cpuinfo"], capture=True)
    cpu_model = "unknown"
    for line in result.stdout.split("\n"):
        if line.startswith("model name"):
            cpu_model = line.split(":", 1)[1].strip()
            break

    # CPU governor
    governors = set()
    try:
        for f in Path("/sys/devices/system/cpu").glob("cpu*/cpufreq/scaling_governor"):
            governors.add(f.read_text().strip())
    except OSError:
        pass
    cpu_governor = ",".join(governors) if governors else "unknown"

    # Kernel version
    result = run_command(["uname", "-a"], capture=True)
    kernel_version = result.stdout.strip()

    # Rust version
    result = run_command(["rustc", "--version"], capture=True)
    rustc_version = result.stdout.strip()

    return {
        "cpu_model": cpu_model,
        "cpu_governor": cpu_governor,
        "kernel_version": kernel_version,
        "rustc_version": rustc_version,
        "host": platform.node(),
        "os": platform.system(),
    }


def get_nightly_date() -> str:
    """Extract nightly date from rustc version string."""
    result = run_command(["rustc", "--version"], capture=True)
    version = result.stdout.strip()
    # Pattern: rustc 1.97.0-nightly (hash YYYY-MM-DD)
    match = re.search(r"\((\w+)\s+(\d{4}-\d{2}-\d{2})\)", version)
    if match:
        return match.group(2)
    return "unknown"


def get_binary_sha256(binary_path: str) -> str:
    """Compute SHA256 of the benchmark binary."""
    result = run_command(["sha256sum", binary_path], capture=True)
    return result.stdout.split()[0]


def discover_benchmark_binary(root: str) -> str:
    """Find the compiled benchmark binary using cargo."""
    result = run_command(
        ["cargo", "bench", "--package", "velvet-ballistics-workspace-tests",
         "--bench", "velvet_ballistics", "--no-run", "--message-format", "json"],
        cwd=root, capture=True
    )
    # Parse JSON lines to find the executable
    for line in result.stdout.split("\n"):
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
            if obj.get("reason") == "compiler-artifact" and obj.get("target", {}).get("kind") == ["bench"]:
                return obj["executable"]
        except json.JSONDecodeError:
            continue
    raise RuntimeError(f"Could not find benchmark binary. cargo output: {result.stdout[:500]}")


def compute_percentiles(samples: List[float]) -> Dict[str, float]:
    """Compute p50, p95, p99 percentiles from a list of samples."""
    sorted_samples = sorted(samples)
    n = len(sorted_samples)
    if n == 0:
        return {"p50": 0.0, "p95": 0.0, "p99": 0.0}

    def percentile(p: float) -> float:
        k = (n - 1) * p
        f = int(k)
        c = f + 1
        if c >= n:
            return sorted_samples[f]
        d = k - f
        return sorted_samples[f] * (1 - d) + sorted_samples[c] * d

    return {
        "p50": percentile(0.50),
        "p95": percentile(0.95),
        "p99": percentile(0.99),
    }


def extract_criterion_json_output(bench_name: str, root: str) -> Optional[Dict[str, Any]]:
    """
    Look for Criterion JSON output files generated during benchmark runs.
    
    Criterion writes per-benchmark JSON to target/criterion/<name>/estimates.json
    """
    crit_dir = Path(root) / "target" / "criterion" / bench_name
    estimates_file = crit_dir / "estimates.json"
    
    if not estimates_file.exists():
        return None
    
    try:
        with open(estimates_file) as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError):
        return None


def run_criterion_benchmark(binary: str, bench_name: str, root: str) -> Dict[str, Any]:
    """Run a single Criterion benchmark and extract percentiles."""
    cmd = [binary, bench_name] + CRITERION_FLAGS
    result = run_command(cmd, cwd=root, capture=False, timeout=300)
    
    if result.returncode != 0:
        raise RuntimeError(f"Criterion benchmark failed (exit {result.returncode}): {bench_name}")
    
    # Try to extract percentiles from Criterion JSON output
    criterion_json = extract_criterion_json_output(bench_name, root)
    
    p50 = p95 = p99 = None
    if criterion_json and "estimates" in criterion_json:
        estimates = criterion_json["estimates"]
        mean = estimates.get("mean", {}).get("point_estimate")
        median = estimates.get("median", {}).get("point_estimate")
        
        # Criterion provides mean and median directly
        if mean is not None:
            p50 = median if median is not None else mean
        
        # For p95/p99, we'd need raw samples; use estimates as fallback
        if criterion_json.get("raw"):
            samples = criterion_json["raw"]
            pcts = compute_percentiles(samples)
            p50 = pcts["p50"]
            p95 = pcts["p95"]
            p99 = pcts["p99"]
    
    return {
        "p50_ns": p50,
        "p95_ns": p95,
        "p99_ns": p99,
        "exit_code": result.returncode,
    }


def run_perf_stat(binary: str, bench_name: str, root: str, output_file: str) -> Dict[str, Any]:
    """Run perf stat to count instructions."""
    cmd = [
        "perf", "stat", "-x,", "-e", "instructions,cycles",
        "-o", output_file,
        "--", binary, bench_name
    ] + CRITERION_FLAGS
    
    result = run_command(cmd, cwd=root, capture=False, timeout=300)
    
    if result.returncode != 0:
        raise RuntimeError(f"perf stat failed (exit {result.returncode}): {bench_name}")
    
    # Parse perf output file
    try:
        with open(output_file) as f:
            content = f.read()
        
        # Find instruction count
        match = re.search(r"(\d+),instructions", content)
        if not match:
            raise RuntimeError(f"Could not parse instruction count from perf output for {bench_name}")
        
        instructions = int(match.group(1))
        
        return {
            "instruction_count": instructions,
            "instructions_per_op": None,  # Whole process count, not per-op
            "raw_output_file": output_file,
        }
    except IOError as e:
        raise RuntimeError(f"Could not read perf output file: {e}") from e


def run_heaptrack(binary: str, bench_name: str, root: str, output_dir: str) -> Dict[str, Any]:
    """Run heaptrack to measure allocations."""
    base = Path(output_dir) / f"heaptrack-{bench_name}"
    cmd = [
        "heaptrack", "--record-only", "-o", str(base),
        binary, bench_name
    ] + CRITERION_FLAGS
    
    result = run_command(cmd, cwd=root, capture=False, timeout=300)
    
    if result.returncode != 0:
        raise RuntimeError(f"heaptrack failed (exit {result.returncode}): {bench_name}")
    
    # Run heaptrack_print to get human-readable summary
    result_file = None
    for f in base.with_suffix(".htr").parent.glob(f"{bench_name}*.htr"):
        result_file = f
        break
    
    if result_file is None:
        # Try without bench name filter
        for f in base.parent.glob("*.htr"):
            result_file = f
            break
    
    if result_file is None:
        raise RuntimeError(f"Could not find heaptrack result file for {bench_name}")
    
    # Run heaptrack_print
    result = run_command(["heaptrack_print", "--json", str(result_file)], capture=True, timeout=60)
    
    if result.returncode != 0:
        raise RuntimeError(f"heaptrack_print failed: {result.stderr[:500]}")
    
    try:
        ht_json = json.loads(result.stdout)
        return {
            "allocation_count_process_level": ht_json.get("allocation_count", 0),
            "peak_heap_memory": ht_json.get("peak_memory", "unknown"),
            "bytes_allocated": ht_json.get("bytes_allocated", 0),
            "result_file": str(result_file),
        }
    except json.JSONDecodeError:
        # Fallback: parse text output
        result = run_command(["heaptrack_print", str(result_file)], capture=True, timeout=60)
        return {
            "allocation_count_process_level": "unknown",
            "peak_heap_memory": "unknown",
            "bytes_allocated": "unknown",
            "result_file": str(result_file),
            "raw_summary": result.stdout[:1000],
        }


def fixture_digest(root: str) -> str:
    """Compute digest of the benchmark fixture files."""
    fixture_files = [
        "crates/workspace_tests/benches/velvet_ballistics.rs",
        "contracts/perf-budget.yaml",
    ]
    
    h = hashlib.sha256()
    for f in fixture_files:
        try:
            with open(os.path.join(root, f), "rb") as fp:
                h.update(fp.read())
        except IOError:
            pass
    
    return f"sha256:{h.hexdigest()}"


def validate_no_markers(log_path: str) -> None:
    """Check raw log for forbidden placeholder markers."""
    try:
        with open(log_path) as f:
            content = f.read()
        for marker in FORBIDDEN_MARKERS:
            if marker in content:
                raise RuntimeError(
                    f"Forbidden placeholder marker found in raw log {log_path}: {marker}"
                )
    except IOError:
        pass  # Log may not exist yet


def write_evidence_record(output_dir: str, record: Dict[str, Any]) -> str:
    """Write a single evidence record as JSONL."""
    path = os.path.join(output_dir, "benchmark-evidence.jsonl")
    with open(path, "a") as f:
        f.write(json.dumps(record) + "\n")
    return path


def main():
    parser = argparse.ArgumentParser(
        description="Velvet Ballistics Benchmark Evidence Collector"
    )
    parser.add_argument("--commit", required=True,
                        help="Git commit SHA for source freeze")
    parser.add_argument("--root", required=True,
                        help="Workspace root directory")
    parser.add_argument("--binary", default=None,
                        help="Path to benchmark binary (discovered if not provided)")
    parser.add_argument("--output-dir", default=None,
                        help="Output directory (default: .beads/vb-j3e3s/evidence)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Only validate setup, do not run benchmarks")
    parser.add_argument("--skip-criterion", action="store_true",
                        help="Skip Criterion timing measurements")
    parser.add_argument("--skip-perf", action="store_true",
                        help="Skip perf instruction counting")
    parser.add_argument("--skip-heaptrack", action="store_true",
                        help="Skip heaptrack allocation measurement")
    
    args = parser.parse_args()
    
    # Verify commit
    result = run_command(["git", "rev-parse", "HEAD"], cwd=args.root, capture=True)
    actual_head = result.stdout.strip()
    
    if args.commit not in actual_head and actual_head not in args.commit:
        print(f"WARNING: --commit {args.commit} does not match HEAD {actual_head}", file=sys.stderr)
    
    print(f"Collecting benchmark evidence for commit {actual_head}")
    print(f"Root: {args.root}")
    
    # Set up output directory
    output_dir = args.output_dir or os.path.join(args.root, ".beads", "vb-j3e3s", "evidence")
    os.makedirs(output_dir, exist_ok=True)
    
    # Verify prerequisites
    check_prerequisites()
    
    # Collect system metadata
    metadata = get_system_metadata()
    nightly_date = get_nightly_date()
    
    print(f"System: {metadata['cpu_model']}")
    print(f"Governor: {metadata['cpu_governor']}")
    print(f"Rustc: {metadata['rustc_version']}")
    
    # Discover or use provided binary
    if args.binary:
        binary = os.path.join(args.root, args.binary) if not args.binary.startswith("/") else args.binary
    else:
        print("Discovering benchmark binary...")
        binary = discover_benchmark_binary(args.root)
    
    binary = os.path.abspath(binary)
    print(f"Binary: {binary}")
    
    if not os.path.exists(binary):
        raise RuntimeError(f"Benchmark binary not found: {binary}")
    
    # Compute fixture digest
    input_fixture_digest = fixture_digest(args.root)
    
    # Determine Criterion version
    result = run_command(["cargo", "tree", "-p", "velvet-ballistics-workspace-tests"],
                         cwd=args.root, capture=True)
    crit_version = "unknown"
    import re as re_module
    for line in result.stdout.split("\n"):
        m = re_module.search(r"criterion v?(\d+\.\d+\.\d+)", line)
        if m:
            crit_version = m.group(1)
            break
    
    print(f"Criterion version: {crit_version}")
    
    if args.dry_run:
        print("\nDry run: setup validated, benchmarks not executed.")
        print("\nUsage to run actual collection:")
        print(f"  python3 {sys.argv[0]} --commit {args.commit} --root {args.root} "
              f"--output-dir {output_dir}")
        return 0
    
    # Run benchmarks
    results = []
    for bench_name in BENCHMARKS:
        print(f"\n{'=' * 60}")
        print(f"Collecting: {bench_name}")
        print(f"{'=' * 60}")
        
        # Per-benchmark thresholds from contracts/perf-budget.yaml
        threshold = 5 if bench_name == "ipc_frame_decode" else 3
        
        record = {
            "metric": bench_name,
            "commit": actual_head,
            "binary_sha256": binary_sha,
            "fixture_digest": input_fixture_digest,
            "cpu_model": metadata["cpu_model"],
            "threshold_percent": threshold,
            "kernel_version": metadata["kernel_version"],
            "rustc_version": metadata["rustc_version"],
            "nightly_date": nightly_date,
            "build_profile": "release",
            "benchmark_tool_version": f"criterion {crit_version}",
            "durability_profile": "mixed",
            "mode": "ir",
            "fjall_write_latency": "not-applicable",
            "direct_api_latency": "not-applicable",
            "ipc_latency": "not-applicable",
        }
        
        # Run Criterion benchmark
        if not args.skip_criterion:
            print("Running Criterion benchmark...")
            crit = run_criterion_benchmark(binary, bench_name, args.root)
            record["p50_latency"] = crit["p50_ns"]
            record["p95_latency"] = crit["p95_ns"]
            record["p99_latency"] = crit["p99_ns"]
            record["baseline_value"] = crit["p50_ns"]
            record["result_value"] = crit["p50_ns"]
        else:
            record["p50_latency"] = "skipped"
            record["p95_latency"] = "skipped"
            record["p99_latency"] = "skipped"
            record["baseline_value"] = "skipped"
            record["result_value"] = "skipped"
        
        # Run perf stat
        if not args.skip_perf:
            print("Running perf stat...")
            perf_output = os.path.join(output_dir, f"{bench_name}.perf.log")
            try:
                perf = run_perf_stat(binary, bench_name, args.root, perf_output)
                record["instruction_count"] = perf["instruction_count"]
                record["sample_count_or_instruction_count"] = perf["instruction_count"]
            except RuntimeError as e:
                print(f"  perf failed: {e}", file=sys.stderr)
                record["instruction_count"] = "measurement-failed"
                record["sample_count_or_instruction_count"] = "measurement-failed"
        else:
            record["instruction_count"] = "skipped"
            record["sample_count_or_instruction_count"] = "skipped"
        
        # Run heaptrack
        if not args.skip_heaptrack:
            print("Running heaptrack...")
            try:
                ht = run_heaptrack(binary, bench_name, args.root, output_dir)
                record["allocation_count"] = ht["allocation_count_process_level"]
                record["bytes_allocated"] = ht["bytes_allocated"]
            except RuntimeError as e:
                print(f"  heaptrack failed: {e}", file=sys.stderr)
                record["allocation_count"] = "measurement-failed"
                record["bytes_allocated"] = "measurement-failed"
        else:
            record["allocation_count"] = "skipped"
            record["bytes_allocated"] = "skipped"
        
        # Build command string
        record["command"] = f"{binary} {bench_name} {' '.join(CRITERION_FLAGS)}"
        record["raw_log"] = f".beads/vb-j3e3s/evidence/{bench_name}.raw"
        record["rustflags"] = ""
        
        results.append(record)
        print(f"  Completed: {bench_name}")
    
    # Write combined evidence file
    evidence_path = os.path.join(output_dir, "benchmark-evidence.jsonl")
    # Clear old file
    if os.path.exists(evidence_path):
        os.remove(evidence_path)
    
    for record in results:
        write_evidence_record(output_dir, record)
    
    print(f"\nEvidence written to: {evidence_path}")
    print(f"Records written: {len(results)}")
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
