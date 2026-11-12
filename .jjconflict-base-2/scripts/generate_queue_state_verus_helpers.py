#!/usr/bin/env python3
"""Generate/check Verus queue helper bodies from production helper definitions.

The production helper route in crates/vb_queue_semantics/src/lib.rs is the
source of truth for PF-vb-8mdp.8-S6-BRIDGE-001. This script mechanically
extracts the accepted helper bodies and rewrites only the marked helper region
inside verification/verus/vb_8mdp_8/queue_state_shared_source.rs.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PRODUCTION = ROOT / "crates" / "vb_queue_semantics" / "src" / "lib.rs"
VERUS = ROOT / "verification" / "verus" / "vb_8mdp_8" / "queue_state_shared_source.rs"

HELPERS = (
    "helper_valid_capacity",
    "helper_queue_is_full",
    "helper_enqueue_accepts",
    "helper_command_pop_is_pop_front",
    "helper_shard_tick_is_pop_front",
    "helper_runtime_queue_full_maps",
)

SIGNATURES = {
    "helper_valid_capacity": "pub fn helper_valid_capacity(capacity: usize) -> (accepted: bool)\n    ensures accepted == valid_capacity(capacity as int),",
    "helper_queue_is_full": "pub fn helper_queue_is_full(capacity: usize, len: usize) -> (full: bool)\n    ensures full == queue_is_full_len(len as int, capacity as int),",
    "helper_enqueue_accepts": "pub fn helper_enqueue_accepts(capacity: usize, len: usize) -> (accepted: bool)\n    requires valid_capacity(capacity as int), len <= capacity,\n    ensures accepted == !queue_is_full_len(len as int, capacity as int),",
    "helper_command_pop_is_pop_front": "pub fn helper_command_pop_is_pop_front(capacity: usize, len: usize) -> (pop_front: bool)\n    requires valid_capacity(capacity as int), len <= capacity,\n    ensures pop_front == (len as int > 0),",
    "helper_shard_tick_is_pop_front": "pub fn helper_shard_tick_is_pop_front(capacity: usize, len: usize) -> (pop_front: bool)\n    requires valid_capacity(capacity as int), len <= capacity,\n    ensures pop_front == (len as int > 0),",
    "helper_runtime_queue_full_maps": "pub fn helper_runtime_queue_full_maps(depth: usize, capacity: usize) -> (is_queue_full: bool)\n    requires valid_capacity(capacity as int), depth <= capacity,\n    ensures is_queue_full == runtime_queue_full_error_transition(depth as int, capacity as int, 0),",
}

BODY_RE = re.compile(
    r"pub const fn (?P<name>helper_[A-Za-z0-9_]+)\([^)]*\) -> bool \{\n(?P<body>.*?)\n\}",
    re.DOTALL,
)
CAPACITY_RE = re.compile(
    r"pub const SHARED_QUEUE_CAPACITY_MAX: usize = (?P<value>[0-9][0-9_]*);"
)
CONSTANT_REGION_RE = re.compile(
    r"// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib\.rs constants\.\n"
    r".*?"
    r"// END GENERATED FROM crates/vb_queue_semantics/src/lib\.rs constants\.",
    re.DOTALL,
)
REGION_RE = re.compile(
    r"// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib\.rs helper route\.\n"
    r".*?"
    r"// END GENERATED FROM crates/vb_queue_semantics/src/lib\.rs helper route\.",
    re.DOTALL,
)


def production_source() -> str:
    return PRODUCTION.read_text(encoding="utf-8")


def production_capacity_max(text: str) -> str:
    match = CAPACITY_RE.search(text)
    if match is None:
        raise RuntimeError("missing production constant: SHARED_QUEUE_CAPACITY_MAX")
    value = match.group("value").replace("_", "")
    if int(value) <= 0:
        raise RuntimeError("SHARED_QUEUE_CAPACITY_MAX must be positive")
    return value


def production_bodies(text: str, capacity_max: str) -> dict[str, str]:
    found = {match.group("name"): match.group("body").strip() for match in BODY_RE.finditer(text)}
    missing = [name for name in HELPERS if name not in found]
    if missing:
        raise RuntimeError(f"missing production helpers: {', '.join(missing)}")
    return {name: normalize_body(found[name], capacity_max) for name in HELPERS}


def normalize_body(body: str, capacity_max: str) -> str:
    return body.replace("SHARED_QUEUE_CAPACITY_MAX", f"{capacity_max}usize")


def render_constants(capacity_max: str) -> str:
    return "\n\n".join(
        [
            "// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib.rs constants.",
            "// Regenerate/check with: python3 scripts/generate_queue_state_verus_helpers.py --check",
            f"pub open spec fn max_queue_capacity() -> int {{ {capacity_max} }}",
            "// END GENERATED FROM crates/vb_queue_semantics/src/lib.rs constants.",
        ]
    )


def render_helpers(bodies: dict[str, str]) -> str:
    parts = [
        "// BEGIN GENERATED FROM crates/vb_queue_semantics/src/lib.rs helper route.",
        "// Regenerate/check with: python3 scripts/generate_queue_state_verus_helpers.py --check",
    ]
    for name in HELPERS:
        parts.append(f"{SIGNATURES[name]}\n{{\n    {bodies[name]}\n}}")
    parts.append("// END GENERATED FROM crates/vb_queue_semantics/src/lib.rs helper route.")
    return "\n\n".join(parts)


def replace_single_region(
    text: str, pattern: re.Pattern[str], new_region: str, region_name: str
) -> str:
    replacement_count = len(pattern.findall(text))
    if replacement_count != 1:
        raise RuntimeError(f"expected exactly one {region_name} region, found {replacement_count}")
    return pattern.sub(new_region, text)


def replace_regions() -> str:
    production_text = production_source()
    capacity_max = production_capacity_max(production_text)
    bodies = production_bodies(production_text, capacity_max)
    text = VERUS.read_text(encoding="utf-8")
    text = replace_single_region(text, CONSTANT_REGION_RE, render_constants(capacity_max), "constant")
    return replace_single_region(text, REGION_RE, render_helpers(bodies), "helper")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail if generated region is stale")
    args = parser.parse_args()

    try:
        next_text = replace_regions()
        current_text = VERUS.read_text(encoding="utf-8")
        if args.check:
            if current_text != next_text:
                print(f"STALE: {VERUS} differs from production helper source", file=sys.stderr)
                return 1
            print(f"fresh: {VERUS} is generated from {PRODUCTION}")
            return 0
        VERUS.write_text(next_text, encoding="utf-8")
        print(f"generated: {VERUS} from {PRODUCTION}")
        return 0
    except OSError as error:
        print(f"I/O error: {error}", file=sys.stderr)
        return 2
    except RuntimeError as error:
        print(f"generation error: {error}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
