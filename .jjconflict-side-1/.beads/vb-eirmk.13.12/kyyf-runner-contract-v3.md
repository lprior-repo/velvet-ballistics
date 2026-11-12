# Contract scope extension V3: shared prebuilt CLI selection

This successor corrects the relative-path escape rejection in V1. Preserve `.beads/vb-eirmk.13.12/kyyf-runner-contract.md` unchanged; this V3 supersedes only that rejection. Trusted configured paths are not confined to `WORKSPACE_ROOT`. This contract addresses test-runner selection and does not establish which process wrote a prior CLI hash change.

## Shared private test helper

Add `crates/workspace_tests/tests/support/prebuilt_cli_path.rs` and share it from the NF2U and KYYF integration-test targets. `configured_prebuilt_cli() -> Option<OsString>` is the sole environment lookup for `CARGO_BIN_EXE_velvet-ballistics`. `resolve_prebuilt_cli(path: OsString, root: &Path) -> PathBuf` is pure path policy. Runners must not duplicate environment lookup or path resolution. Keep each runner's `None` Cargo command builder local because their prefixes differ. Add no public application API.

## Explicit configured path

An absent environment variable yields `None`. Every supplied value, including empty, invalid, non-UTF-8, or unlaunchable values, remains `Some` and never selects Cargo.

For `Some(path)`, preserve OS bytes without Unicode conversion. Return an absolute path unchanged. Resolve every relative path exactly as `root.join(path)`, lexically, preserving `..` components. Thus trusted `../configured-cli` is authorized and resolves to `root.join("../configured-cli")`; this is not a confinement or security boundary. Do not reject escapes, canonicalize, probe the filesystem, search `PATH`, or introduce a new error API.

Both runners execute the resolved explicit path directly. A launch failure uses the existing visible error path and never falls back to Cargo. Preserve each runner's existing child working-directory behavior, including KYYF's inherited current directory. Preserve all existing argument order and output/error mapping.

## Exact absent-variable behavior

When `configured_prebuilt_cli()` returns `None`, retain each runner's exact legacy Cargo executable and argument vector. NF2U keeps `cargo run --quiet --manifest-path <CARGO_MANIFEST_DIR>/../../Cargo.toml -p velvet-ballistics --` followed by its original arguments. KYYF keeps `env!("CARGO") run --quiet -p velvet-ballistics --bin velvet-ballistics -- <command_name> <run_arg> --db <db_path>`. Preserve the current directory behavior of each runner.

## Required CODE and TEST evidence

First add a pure KYYF command-inspection regression that is red against the old unconditional-Cargo builder: with a distinctive explicit path, assert the built program is the resolved path and its argv is the original KYYF argv. It must fail before any spawn or Cargo execution. Then implement shared selection and make that same check green.

Test the pure resolver with a bare relative name, `../configured-cli`, an absolute missing path, a non-UTF-8 relative path, and an empty supplied value. Assert exact `PathBuf`/OS-byte results; do not normalize or reject `..`. Test absence selecting each unchanged legacy builder. For explicit paths, inspect program and arguments before any spawn; if testing a launch error, verify the direct path first and retain the visible error/no-fallback behavior.

Run all five NF2U scenarios and preserve their inputs and assertions. Preserve all six KYYF replay/events/inspect invocations and comparisons, including command inputs, JSON/report decoding, stdout/stderr/status checks, semantic comparisons, history fixtures, and evidence parsing. Do not loosen or remove any existing assertion.

The selected-run criterion hashes the actual prebuilt CLI immediately before and after the full selected Moon invocation. A stable hash supports the criterion; a changed hash keeps the bead open. This contract does not identify the writer of a changed binary.

## Boundaries

No production CLI change, public API, dependency, schema, gate/config edit, database operation, tracker mutation, skipped case, or weakened assertion. Do not execute the Cargo fallback as the baseline-red oracle. This contract is not implementation, native acceptance, or proof of whole-suite hash stability.
