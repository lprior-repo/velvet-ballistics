#!/usr/bin/env python3
"""Immutable runner for cargo audit gate."""
import subprocess
import sys
import time
import json

def run(cmd, timeout=180):
    """Run command, capture stdout/stderr, return exit code."""
    start = time.time()
    result = subprocess.run(
        cmd, shell=True, capture_output=True, text=True, timeout=timeout
    )
    elapsed = time.time() - start
    return result.returncode, result.stdout, result.stderr, elapsed

def main():
    evidence_dir = ".beads/vb-9ucmq/evidence"
    cmd = "cargo audit --quiet"
    
    print(f"Running: {cmd}")
    rc, stdout, stderr, elapsed = run(cmd)
    
    # Write raw output
    with open(f"{evidence_dir}/audit-stdout.txt", "w") as f:
        f.write(stdout)
    with open(f"{evidence_dir}/audit-stderr.txt", "w") as f:
        f.write(stderr)
    
    # Also run JSON version for machine parsing
    rc_json, stdout_json, stderr_json, elapsed_json = run("cargo audit --quiet --json", timeout=180)
    
    if rc_json == 0:
        with open(f"{evidence_dir}/audit.json", "w") as f:
            f.write(stdout_json)
    else:
        with open(f"{evidence_dir}/audit-stderr-json.txt", "w") as f:
            f.write(stderr_json)
    
    print(f"Exit code: {rc}")
    print(f"Elapsed: {elapsed:.2f}s")
    print("Output written to .beads/vb-9ucmq/evidence/")
    
    return rc

if __name__ == "__main__":
    sys.exit(main())