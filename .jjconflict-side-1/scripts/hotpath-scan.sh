#!/usr/bin/env bash
set -euo pipefail

ROOT="$(pwd -P)"
if [[ ! -f "$ROOT/Cargo.toml" || ! -d "$ROOT/crates" ]]; then
  printf '%s\n' "InvalidInvocation: run from repository root" >&2
  exit 64
fi

python3 - <<'PY'
from pathlib import Path
import sys

ROOT = Path.cwd()
HOT_ROOTS = [
    ROOT / "crates/vb_core/src",
    ROOT / "crates/vb_runtime/src",
    ROOT / "crates/vb_storage/src",
    ROOT / "crates/vb_ipc/src",
]
TOKENS = ["HashMap", "IndexMap", "IndexSet", "BTreeMap", "std::sync::mpsc", "mpsc::channel", "channel("]
COLD_PARTS = {"diagnostic", "diagnostics", "fixture", "fixtures", "harness", "kani", "loom", "proof", "property", "proptest", "proptests", "support", "test", "tests", "verification"}
ALLOW_PATH = ROOT / "scripts/hotpath-scan.allow"

def fail(message: str, code: int) -> None:
    print(message, file=sys.stderr)
    raise SystemExit(code)

def path_tokens(rel: str) -> set[str]:
    normalized = rel.replace("/", " ").replace(".", " ").replace("_", " ").replace("-", " ")
    return set(normalized.split())

def is_cold(rel: str) -> bool:
    return bool(path_tokens(rel) & COLD_PARTS)

def strip_comment(line: str) -> str:
    return line.split("//", 1)[0]

def load_allow() -> set[tuple[str, str]]:
    allowed: set[tuple[str, str]] = set()
    if not ALLOW_PATH.exists():
        return allowed
    for number, raw in enumerate(ALLOW_PATH.read_text().splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("|")
        if len(parts) != 6:
            fail(f"MalformedException: scripts/hotpath-scan.allow:{number} expected path|token|owner=...|reviewed_by=...|test=...|reason=...", 3)
        path, token = parts[0], parts[1]
        if "*" in path or not path.startswith("crates/") or not path.endswith(".rs"):
            fail(f"OverbroadException: scripts/hotpath-scan.allow:{number} path must be exact crates/*/src/*.rs", 3)
        if token not in TOKENS:
            fail(f"UnknownTokenException: scripts/hotpath-scan.allow:{number} token must be one of {','.join(TOKENS)}", 3)
        if not parts[2].startswith("owner=") or not parts[3].startswith("reviewed_by=") or not parts[4].startswith("test=") or not parts[5].startswith("reason="):
            fail(f"MalformedException: scripts/hotpath-scan.allow:{number} missing owner/reviewed_by/test/reason", 3)
        allowed.add((path, token))
    return allowed

allowed = load_allow()
violations: list[tuple[str, int, str, str]] = []
justified = 0
classified = 0

for hot_root in HOT_ROOTS:
    if not hot_root.exists():
        continue
    for source in sorted(hot_root.rglob("*.rs")):
        rel = source.relative_to(ROOT).as_posix()
        cold = is_cold(rel)
        classified += 1
        print(f"ClassifiedPath|{'cold' if cold else 'hot'}|{rel}")
        if cold:
            continue
        for line_no, raw in enumerate(source.read_text().splitlines(), start=1):
            text = strip_comment(raw)
            for token in TOKENS:
                if token not in text:
                    continue
                if (rel, token) in allowed:
                    justified += 1
                    print(f"JustifiedException|{token}|{rel}|line={line_no}")
                else:
                    violations.append((rel, line_no, token, " ".join(text.split())))

for rel, line_no, token, text in violations:
    print(f"ViolationFound|{token}|{rel}|line={line_no}|{text}")

print(f"ScanSummary|hot_roots=vb_core,vb_runtime,vb_storage,vb_ipc|classified={classified}|violations={len(violations)}|justified={justified}")
raise SystemExit(0 if not violations else 2)
PY
