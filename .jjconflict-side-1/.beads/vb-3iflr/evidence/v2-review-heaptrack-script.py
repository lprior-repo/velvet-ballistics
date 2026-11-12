#!/usr/bin/env python3
"""Verify heaptrack traces match manifest by parsing raw traces."""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path("/home/lewis/src/isoloated/velvet-ballistics-vb-j3e3s-local3090-measurement-review")
MANIFEST = ROOT / ".beads/vb-j3e3s/evidence/actual-collection-v2/heaptrack-manifest.json"

def parse_heaptrack_trace(text):
    """Decode heaptrack v3 .zst trace into exact counts."""
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
                raise ValueError(f"truncated version header at line {lineno}")
            try:
                major = int(tok[2])
            except ValueError:
                raise ValueError(f"bad version header at line {lineno}: {raw!r}")
            if major != 3:
                raise ValueError(f"unknown heaptrack format major {major}")
        elif op == "a":
            if len(tok) < 3:
                raise ValueError(f"truncated allocation record at line {lineno}: {raw!r}")
            try:
                sizes.append(int(tok[1], 16))
            except ValueError:
                raise ValueError(f"bad allocation size at line {lineno}: {raw!r}")
        elif op in ("+", "-"):
            if len(tok) < 2:
                raise ValueError(f"truncated event record at line {lineno}: {raw!r}")
            try:
                idx = int(tok[1], 16)
            except ValueError:
                raise ValueError(f"bad event index at line {lineno}: {raw!r}")
            if idx < 0 or idx >= len(sizes):
                raise ValueError(f"out-of-range allocation-info index {idx} at line {lineno}")
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
                    raise ValueError(f"negative live heap at line {lineno}")
    if major is None:
        raise ValueError("heaptrack trace missing v3 version header")
    if count == 0:
        raise ValueError("heaptrack trace recorded zero allocations")
    return count, total, peak

def decompress_zst(path):
    """Decompress .zst file and return text."""
    result = subprocess.run(
        ["zstd", "-d", "-c", str(path)],
        capture_output=True,
        check=True
    )
    return result.stdout.decode("utf-8")

# Load manifest
manifest = json.loads(MANIFEST.read_text())

print("=== Heaptrack Trace Verification ===")
print()

for name, info in manifest.items():
    path = ROOT / info["preserved_repository_path"]
    print(f"--- {name} ---")
    print(f"  File: {path.name}")
    print(f"  Expected allocs: {info['allocation_count']}")
    print(f"  Expected bytes:  {info['bytes_allocated']}")
    
    try:
        text = decompress_zst(path)
        allocs, total, peak = parse_heaptrack_trace(text)
        
        print(f"  Parsed allocs:   {allocs}")
        print(f"  Parsed bytes:    {total}")
        print(f"  Parsed peak:     {peak}")
        
        allocs_match = allocs == info["allocation_count"]
        bytes_match = total == info["bytes_allocated"]
        
        print(f"  Allocations match: {allocs_match}")
        print(f"  Bytes match:       {bytes_match}")
        
        if not allocs_match:
            print(f"  ERROR: allocation count mismatch!")
        if not bytes_match:
            print(f"  ERROR: bytes allocated mismatch!")
            
    except Exception as e:
        print(f"  ERROR: {e}")
    
    print()

print("Verification complete.")
