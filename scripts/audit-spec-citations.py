"""Audit production line citations in verification/verus/*.rs spec files.

For each `path.rs:LINE[-LINE][, ...]` citation found in doc comments,
resolve the path against the repo root and check that every cited line
exists and that the cited span contains at least one identifier drawn from
the citing comment context. Reports per-file PASS/FAIL plus totals.
Exit 0 iff zero failures.

Usage: python3 scripts/audit-spec-citations.py <repo-root> [file ...]

Build-output directories (target/) are excluded from the short-path
index, so generated files can never resolve a spec citation.

External-dependency citations (cargo-registry paths such as
`~/.cargo/registry/src/<index-hash>/fjall-3.1.4/src/batch/mod.rs:100-182`)
are resolved against $CARGO_HOME (default ~/.cargo) and the cited crate
version must match the pin in <repo-root>/Cargo.lock. A registry path
whose source is absent locally, or whose version is not the pinned one,
fails instead of passing silently.
"""
import os
import re
import sys
from pathlib import Path

CARGO_HOME = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
REGISTRY_PREFIX = "/.cargo/registry/src/"
CRATE_DIR_RE = re.compile(r"^(.+)-(\d+\.\d+\.\d+.*)$")

STOP = {
    "types", "rs", "crates", "vb_runtime", "vb_core", "vb_storage", "vb_ipc",
    "src", "shard", "production", "production_inner", "mirror", "variant",
    "field", "struct", "enum", "fn", "pub", "const", "the", "at", "is", "in",
    "of", "and", "per", "line", "lines", "range", "run", "admission", "parts",
    "ipc", "runtime", "transitions", "verus", "verification", "extern",
    "spec", "proof", "model", "file", "see", "for", "with", "from", "this",
    "that", "into", "over", "under", "when", "where", "which", "also",
}

CITE_RE = re.compile(r"([A-Za-z0-9_./-]+\.rs):([0-9][0-9,\- ]*)")
IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def parse_ranges(spec):
    out = []
    for part in spec.replace(" ", "").split(","):
        if not part:
            continue
        if "-" in part:
            a, b = part.split("-", 1)
            if not a or not b:
                continue
            out.append((int(a), int(b)))
        else:
            out.append((int(part), int(part)))
    return out


def resolve_registry(raw_path):
    """Resolve a cargo-registry citation against $CARGO_HOME.

    Returns the registry file as a Path, "MISSING_REGISTRY" when the
    path names registry source that is not present locally, or None
    when the path is not a registry citation at all. The index-hash
    directory varies by cargo version, so a literal miss falls back
    to globbing across index-hash directories for the same
    <crate>-<version>/... tail.
    """
    if not raw_path.startswith(REGISTRY_PREFIX):
        return None
    rel = raw_path[len("/.cargo/"):]
    cand = CARGO_HOME / rel
    if cand.is_file():
        return cand
    parts = rel.split("/")
    if len(parts) >= 4:
        tail = "/".join(parts[3:])
        hits = [h for h in sorted((CARGO_HOME / "registry" / "src").glob("*/" + tail))
                if h.is_file()]
        if len(hits) == 1:
            return hits[0]
    return "MISSING_REGISTRY"


def registry_pin_error(root, path):
    """Check a resolved registry file's crate version against Cargo.lock.

    Returns an error string, or None when the cited version is the
    pinned one. A missing Cargo.lock or an unparsable crate directory
    is an error, not a silent pass.
    """
    try:
        crate_dir = path.relative_to(CARGO_HOME / "registry" / "src").parts[1]
    except ValueError:
        return f"registry path outside $CARGO_HOME registry: {path}"
    m = CRATE_DIR_RE.match(crate_dir)
    if not m:
        return f"unparsable registry crate directory: {crate_dir}"
    name, version = m.group(1), m.group(2)
    lock = root / "Cargo.lock"
    if not lock.is_file():
        return f"no Cargo.lock to verify registry pin for {name} {version}"
    text = lock.read_text()
    mm = re.search(rf'name = "{re.escape(name)}"\nversion = "([^"]+)"', text)
    if not mm:
        return f"crate {name} not pinned in Cargo.lock (cited {version})"
    if mm.group(1) != version:
        return (f"registry cite pins {name} {version} but Cargo.lock "
                f"pins {mm.group(1)}")
    return None


def resolve(root, raw_path, index):
    if raw_path.startswith(REGISTRY_PREFIX):
        return resolve_registry(raw_path)
    if raw_path.startswith(("crates/", "verification/", "contracts/", "specs/")):
        return raw_path
    if "/" in raw_path:
        cands = [p for p in index.get(raw_path.rsplit("/", 1)[-1], [])
                 if p.endswith("/" + raw_path)]
        if len(cands) == 1:
            return cands[0]
        cand = "verification/verus/" + raw_path
        if (root / cand).exists():
            return cand
        return cands[0] if cands else None
    hits = index.get(raw_path, [])
    if len(hits) == 1:
        return hits[0]
    return None


def components(tokens):
    out = set()
    for t in tokens:
        out.add(t)
        for part in t.split("_"):
            if len(part) > 2 and part not in STOP:
                out.add(part)
    return out


def main():
    root = Path(sys.argv[1])
    files = sys.argv[2:]
    if not files:
        files = sorted(str(p.relative_to(root)) for p in (root / "verification/verus").glob("*.rs"))
    index = {}
    for p in root.rglob("*.rs"):
        if ".git" in p.parts or "target" in p.parts:
            continue
        index.setdefault(p.name, []).append(str(p.relative_to(root)))
    total_checks = 0
    total_failures = []
    per_file = {}
    for rel_file in files:
        target = root / rel_file
        lines = target.read_text().splitlines()
        checks = 0
        failures = []
        seen = set()
        for i, line in enumerate(lines):
            if not line.lstrip().startswith("//"):
                continue
            for m in CITE_RE.finditer(line):
                raw_path, spec = m.group(1), m.group(2).strip()
                rel = resolve(root, raw_path, index)
                if rel is None:
                    failures.append(f"line {i+1}: ambiguous/unresolvable short path {raw_path}")
                    continue
                if rel == "MISSING_REGISTRY":
                    failures.append(f"line {i+1}: cargo-registry source not present locally: {raw_path}")
                    continue
                if isinstance(rel, Path):
                    pin_err = registry_pin_error(root, rel)
                    if pin_err:
                        failures.append(f"line {i+1}: {pin_err}")
                        continue
                fpath = rel if isinstance(rel, Path) else root / rel
                if not fpath.exists():
                    failures.append(f"line {i+1}: cited file missing: {rel}")
                    continue
                flines = fpath.read_text().splitlines()
                ctx_lines = [lines[j] for j in range(max(0, i - 3), min(len(lines), i + 6))]
                ctx = " ".join(ctx_lines)
                tokens = {t for t in IDENT_RE.findall(ctx) if t not in STOP and len(t) > 2}
                tokens |= {t[6:] for t in list(tokens) if t.startswith("Mirror") and len(t) > 6}
                tokens = components(tokens)
                tokens |= {t.lower() for t in list(tokens)}
                for a, b in parse_ranges(spec):
                    key = (rel, a, b)
                    if key in seen:
                        continue
                    seen.add(key)
                    checks += 1
                    if a < 1 or b > len(flines) or a > b:
                        failures.append(
                            f"line {i+1}: {rel}:{a}-{b} out of range (file has {len(flines)} lines)"
                        )
                        continue
                    span = "\n".join(flines[a - 1:b])
                    span_tokens = set(IDENT_RE.findall(span))
                    span_tokens = components(span_tokens)
                    span_lower = {t.lower() for t in span_tokens}
                    hit = [t for t in tokens if t in span_tokens or t.lower() in span_lower]
                    if not hit:
                        failures.append(
                            f"line {i+1}: {rel}:{a}-{b} span contains none of context idents; "
                            f"span starts: {flines[a-1].strip()[:90]!r}"
                        )
        per_file[rel_file] = (checks, failures)
        total_checks += checks
        total_failures.extend(f"{rel_file}: {f}" for f in failures)
    for rel_file, (checks, failures) in per_file.items():
        print(f"{rel_file}: {checks} checks, {len(failures)} failures")
    print(f"TOTAL: {total_checks} checks, {len(total_failures)} failures")
    for f in total_failures:
        print("FAIL " + f)
    return 1 if total_failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
