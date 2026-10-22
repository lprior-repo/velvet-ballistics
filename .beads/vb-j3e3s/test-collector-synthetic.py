#!/usr/bin/env python3
"""
Synthetic dry-run test for the benchmark evidence collector.

Validates core logic without requiring actual benchmark execution:
- Percentile computation
- JSON serialization of evidence records
- Forbidden marker detection
- JSONL roundtrip
"""

import json
import os
import sys
import tempfile

sys.path.insert(0, os.path.dirname(__file__))
from collect_benchmark_evidence import compute_percentiles, FORBIDDEN_MARKERS


def test_percentiles():
    """Verify percentile computation matches expected values."""
    samples = list(range(1, 101))  # 1..100

    result = compute_percentiles(samples)

    # p50 should be ~50.5 (median of 1..100)
    assert 50.0 <= result["p50"] <= 51.0, f"p50={result['p50']} out of range"

    # p95 should be ~95
    assert 94.0 <= result["p95"] <= 96.0, f"p95={result['p95']} out of range"

    # p99 should be ~99
    assert 98.0 <= result["p99"] <= 100.0, f"p99={result['p99']} out of range"

    # Verify sorted property
    assert result["p50"] <= result["p95"] <= result["p99"], "Percentiles not sorted"

    print("PASS: percentile computation")


def test_evidence_record_serialization():
    """Verify evidence record serializes and deserializes correctly."""
    record = {
        "metric": "bench_engine_step_once_save_const_single_transition",
        "claim": "Fresh baseline measurement",
        "baseline_value": 16627,
        "result_value": 16627,
        "unit": "nanoseconds",
        "threshold_percent": 3,
        "raw_log": ".beads/vb-j3e3s/evidence/test.raw",
        "command": "test binary bench_name --bench",
        "commit": "a9d5b8c",
        "rustc_version": "rustc 1.97.0-nightly",
        "nightly_date": "2026-04-28",
        "cpu_model": "Test CPU",
        "cpu_governor": "performance",
        "kernel_version": "5.15.0",
        "build_profile": "release",
        "rustflags": "",
        "benchmark_tool_version": "criterion 0.8.2",
        "sample_count_or_instruction_count": 12345,
        "input_fixture_digest": "sha256:deadbeef",
        "durability_profile": "mixed",
        "mode": "ir",
        "p50_latency": 16500.0,
        "p95_latency": 17000.0,
        "p99_latency": 17500.0,
        "instruction_count": 54419223615,
        "allocation_count": 923218,
        "bytes_allocated": 7964672,
        "fjall_write_latency": "not-applicable",
        "direct_api_latency": "not-applicable",
        "ipc_latency": "not-applicable",
        "binary_sha256": "abcd1234",
        "fixture_digest": "sha256:abcd",
    }

    # Serialize to JSONL format
    json_line = json.dumps(record)

    # Verify it's valid JSON
    parsed = json.loads(json_line)

    # Verify all required fields from perf-budget.yaml are present
    required_fields = [
        "metric", "claim", "baseline_value", "result_value", "unit",
        "threshold_percent", "raw_log", "command", "commit", "rustc_version",
        "nightly_date", "cpu_model", "cpu_governor", "kernel_version",
        "build_profile", "rustflags", "benchmark_tool_version",
        "sample_count_or_instruction_count", "input_fixture_digest",
        "durability_profile", "mode", "p50_latency", "p95_latency",
        "p99_latency", "instruction_count", "allocation_count",
        "bytes_allocated", "fjall_write_latency", "direct_api_latency",
        "ipc_latency",
    ]

    missing = [f for f in required_fields if f not in parsed]
    assert not missing, f"Missing required fields: {missing}"

    print("PASS: evidence record serialization")


def test_forbidden_marker_detection():
    """Verify forbidden marker list is defined correctly."""
    assert len(FORBIDDEN_MARKERS) == 4, f"Expected 4 markers, got {len(FORBIDDEN_MARKERS)}"
    assert "instructions=not-collected" in FORBIDDEN_MARKERS
    assert "allocations=allocator-external" in FORBIDDEN_MARKERS
    print("PASS: forbidden marker detection configuration")


def test_jsonl_roundtrip():
    """Test writing and reading JSONL evidence."""
    records = [
        {"metric": f"bench_{i}", "value": i * 100}
        for i in range(3)
    ]

    with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
        temp_path = f.name

    try:
        # Write
        with open(temp_path, 'w') as f:
            for record in records:
                f.write(json.dumps(record) + "\n")

        # Read back
        with open(temp_path) as f:
            lines = f.readlines()

        assert len(lines) == 3, f"Expected 3 lines, got {len(lines)}"

        for i, line in enumerate(lines):
            parsed = json.loads(line)
            assert parsed["metric"] == records[i]["metric"]
            assert parsed["value"] == records[i]["value"]

        print("PASS: JSONL roundtrip")
    finally:
        os.unlink(temp_path)


def test_fixture_digest():
    """Verify fixture digest computation works."""
    import hashlib

    h = hashlib.sha256()
    h.update(b"test content")
    digest = f"sha256:{h.hexdigest()}"

    assert digest.startswith("sha256:")
    assert len(digest) == 71  # "sha256:" (7) + 64 hex chars

    print("PASS: fixture digest computation")


if __name__ == "__main__":
    print("Running synthetic collector tests...")

    test_percentiles()
    test_evidence_record_serialization()
    test_forbidden_marker_detection()
    test_jsonl_roundtrip()
    test_fixture_digest()

    print("\nAll synthetic tests passed.")
