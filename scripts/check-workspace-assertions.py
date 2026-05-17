#!/usr/bin/env python3
"""Sharp workspace architecture assertions for velvet-ballastics."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path


EXPECTED_MEMBERS = frozenset(
    {
        "crates/vb_boundary_inventory",
        "crates/vb_core",
        "crates/vb_yaml",
        "crates/vb_validate",
        "crates/vb_expr",
        "crates/vb_compile",
        "crates/vb_storage",
        "crates/vb_runtime",
        "crates/vb_doc",
        "crates/vb_ipc",
        "crates/vb_codegen",
        "crates/vb_ui_makepad",
        "crates/vb_ui_snapshot",
        "crates/vb_proof_kernels",
        "crates/vb_cli",
        "crates/workspace_tests",
        "crates/vb_benchmark",
        "fuzz",
        "xtask",
    }
)

EXPECTED_EXCLUDES = frozenset({"target/miri-tmp", "crates/vb_ui", "fuzz"})
BOUNDARY_CRATES = frozenset({"vb_core", "vb_runtime", "vb_storage", "vb_ipc"})
FORBIDDEN_UI_DEPENDENCIES = frozenset(
    {"vb_ui", "vb_ui_makepad", "vb_ui_model", "vb_ui_snapshot", "makepad-widgets", "makepad-draw"}
)
FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES = frozenset(
    {"serde_json", "saphyr", "saphyr-parser", "serde-saphyr"}
)
DEPENDENCY_TABLES = ("dependencies", "dev-dependencies", "build-dependencies")


def load_toml(path: Path) -> dict[str, object]:
    with path.open("rb") as handle:
        loaded = tomllib.load(handle)
    return loaded


def report(message: str) -> None:
    print(message, file=sys.stderr)


def normalized_strings(value: object) -> set[str]:
    if not isinstance(value, list):
        return set()
    return {item for item in value if isinstance(item, str)}


def check_workspace_members(root: Path, failures: list[str]) -> None:
    cargo_toml = root / "Cargo.toml"
    data = load_toml(cargo_toml)
    workspace = data.get("workspace")
    if not isinstance(workspace, dict):
        failures.append("Cargo.toml: missing [workspace] table")
        return

    actual_members = normalized_strings(workspace.get("members"))
    missing_members = sorted(EXPECTED_MEMBERS - actual_members)
    extra_members = sorted(actual_members - EXPECTED_MEMBERS)
    if missing_members:
        failures.append(f"Cargo.toml: workspace.members missing {missing_members}")
    if extra_members:
        failures.append(f"Cargo.toml: workspace.members unexpected {extra_members}")

    actual_excludes = normalized_strings(workspace.get("exclude"))
    missing_excludes = sorted(EXPECTED_EXCLUDES - actual_excludes)
    if missing_excludes:
        failures.append(f"Cargo.toml: workspace.exclude missing {missing_excludes}")


def dependency_names(manifest: dict[str, object]) -> set[str]:
    names: set[str] = set()
    for table_name in DEPENDENCY_TABLES:
        table = manifest.get(table_name)
        if isinstance(table, dict):
            names.update(name for name in table if isinstance(name, str))
    return names


def check_forbidden_dependencies(root: Path, failures: list[str]) -> None:
    for crate in sorted(BOUNDARY_CRATES):
        manifest_path = root / "crates" / crate / "Cargo.toml"
        if not manifest_path.exists():
            failures.append(f"{manifest_path.relative_to(root)}: missing boundary crate manifest")
            continue

        manifest = load_toml(manifest_path)
        names = dependency_names(manifest)

        ui_hits = sorted(names & FORBIDDEN_UI_DEPENDENCIES)
        if ui_hits:
            failures.append(
                f"{manifest_path.relative_to(root)}: forbidden UI dependency in boundary crate {crate}: {ui_hits}"
            )

        format_hits = sorted(names & FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES)
        if format_hits:
            failures.append(
                f"{manifest_path.relative_to(root)}: forbidden runtime format dependency in {crate}: {format_hits}"
            )


def check_generated_boundaries(root: Path, failures: list[str]) -> None:
    generated_roots = sorted((root / "crates").glob("*/src/generated"))
    for generated_root in generated_roots:
        for source in sorted(generated_root.rglob("*.rs")):
            text = source.read_text(encoding="utf-8")
            for forbidden in sorted(FORBIDDEN_UI_DEPENDENCIES | FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES):
                if forbidden in text:
                    failures.append(
                        f"{source.relative_to(root)}: forbidden generated boundary token {forbidden}"
                    )


def main() -> int:
    root = Path.cwd()
    failures: list[str] = []

    check_workspace_members(root, failures)
    check_forbidden_dependencies(root, failures)
    check_generated_boundaries(root, failures)

    for failure in failures:
        report(failure)

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
