#!/usr/bin/env python3
"""Sharp workspace architecture assertions for velvet-ballastics."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path


ACTIVE_MEMBERS = frozenset(
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
        "crates/workspace_tests",
    }
)

DEFERRED_MEMBERS = frozenset(
    {
        "crates/vb_codegen",
        "crates/vb_ui_makepad",
        "crates/vb_ui_snapshot",
        "crates/vb_proof_kernels",
        "crates/vb_cli",
        "crates/vb_verification",
        "crates/vb_benchmark",
        "fuzz",
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
EXPECTED_PACKAGE_NAMES = {
    "crates/vb_boundary_inventory": "vb_boundary_inventory",
    "crates/vb_core": "vb_core",
    "crates/vb_yaml": "vb_yaml",
    "crates/vb_validate": "vb_validate",
    "crates/vb_expr": "vb_expr",
    "crates/vb_compile": "vb_compile",
    "crates/vb_storage": "vb_storage",
    "crates/vb_runtime": "vb_runtime",
    "crates/vb_doc": "vb_doc",
    "crates/vb_ipc": "vb_ipc",
    "crates/vb_codegen": "vb_codegen",
    "crates/vb_ui_makepad": "vb_ui_makepad",
    "crates/vb_ui_snapshot": "vb_ui_snapshot",
    "crates/vb_proof_kernels": "vb_proof_kernels",
    "crates/vb_cli": "velvet-ballastics",
    "crates/vb_verification": "vb_verification",
    "crates/workspace_tests": "velvet-ballastics-workspace-tests",
    "crates/vb_benchmark": "vb_benchmark",
    "fuzz": "velvet-ballastics-fuzz",
}
EXPECTED_BINARIES = {"crates/vb_cli": {"velvet-ballastics"}}
EXPECTED_FEATURES = {
    "crates/vb_core": {"default", "generated", "bench", "volatile", "test-util"},
    "crates/vb_validate": {"default", "verus"},
    "crates/vb_ui_snapshot": {"default", "std", "tokio"},
}
FORBIDDEN_FEATURE_NAMES = {"json", "serde-json", "velvet-ballistics", "velvet_ballistics"}


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
    missing_members = sorted(ACTIVE_MEMBERS - actual_members)
    extra_members = sorted(actual_members - ACTIVE_MEMBERS)
    if missing_members:
        failures.append(f"Cargo.toml: workspace.members missing {missing_members}")
    if extra_members:
        failures.append(f"Cargo.toml: workspace.members unexpected {extra_members}")

    active_deferred = sorted(actual_members & DEFERRED_MEMBERS)
    if active_deferred:
        failures.append(
            f"Cargo.toml: deferred crates must not be active workspace members {active_deferred}"
        )

    actual_excludes = normalized_strings(workspace.get("exclude"))
    missing_excludes = sorted(EXPECTED_EXCLUDES - actual_excludes)
    if missing_excludes:
        failures.append(f"Cargo.toml: workspace.exclude missing {missing_excludes}")


def package_name(manifest: dict[str, object]) -> str | None:
    package = manifest.get("package")
    if not isinstance(package, dict):
        return None
    name = package.get("name")
    if isinstance(name, str):
        return name
    return None


def binary_names(manifest: dict[str, object]) -> set[str]:
    binaries = manifest.get("bin")
    if not isinstance(binaries, list):
        return set()
    names = set()
    for binary in binaries:
        if isinstance(binary, dict):
            name = binary.get("name")
            if isinstance(name, str):
                names.add(name)
    return names


def feature_names(manifest: dict[str, object]) -> set[str]:
    features = manifest.get("features")
    if not isinstance(features, dict):
        return set()
    return {name for name in features if isinstance(name, str)}


def check_crate_names_binaries_and_features(root: Path, failures: list[str]) -> None:
    for member_path in sorted(EXPECTED_PACKAGE_NAMES):
        if member_path in DEFERRED_MEMBERS:
            continue
        manifest_path = root / member_path / "Cargo.toml"
        if not manifest_path.exists():
            failures.append(f"{member_path}/Cargo.toml: missing member manifest")
            continue
        manifest = load_toml(manifest_path)
        expected_name = EXPECTED_PACKAGE_NAMES[member_path]
        actual_name = package_name(manifest)
        if actual_name != expected_name:
            failures.append(
                f"{member_path}/Cargo.toml: package.name expected {expected_name!r}, got {actual_name!r}"
            )

        if member_path in EXPECTED_BINARIES:
            expected_binaries = EXPECTED_BINARIES[member_path]
            actual_binaries = binary_names(manifest)
            if actual_binaries != expected_binaries:
                failures.append(
                    f"{member_path}/Cargo.toml: bin names expected {sorted(expected_binaries)!r}, got {sorted(actual_binaries)!r}"
                )

        expected_features = EXPECTED_FEATURES.get(member_path)
        actual_features = feature_names(manifest)
        if expected_features is not None and actual_features != expected_features:
            failures.append(
                f"{member_path}/Cargo.toml: features expected {sorted(expected_features)!r}, got {sorted(actual_features)!r}"
            )
        forbidden_features = sorted(actual_features & FORBIDDEN_FEATURE_NAMES)
        if forbidden_features:
            failures.append(
                f"{member_path}/Cargo.toml: forbidden feature names {forbidden_features!r}"
            )


def dependency_names(manifest: dict[str, object]) -> set[str]:
    names: set[str] = set()
    for table_name in DEPENDENCY_TABLES:
        table = manifest.get(table_name)
        if isinstance(table, dict):
            for name, dependency in table.items():
                if isinstance(name, str):
                    names.add(name)
                if isinstance(dependency, dict):
                    package = dependency.get("package")
                    if isinstance(package, str):
                        names.add(package)
                    path = dependency.get("path")
                    if isinstance(path, str):
                        path_alias = path.rstrip("/\\").rsplit("/", maxsplit=1)[-1]
                        if path_alias:
                            names.add(path_alias)
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
    check_crate_names_binaries_and_features(root, failures)
    check_forbidden_dependencies(root, failures)
    check_generated_boundaries(root, failures)

    for failure in failures:
        report(failure)

    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
