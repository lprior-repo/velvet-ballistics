# Relative prebuilt CLI path clarification V2

This successor corrects the relative-escape rejection in `relative-path-resolution-notes.md`, which remains unchanged. Configured paths are trusted inputs; resolution is lexical path selection, not confinement.

For `Some(path)`, preserve the supplied OS bytes without Unicode conversion or filesystem probing. If absolute, return it unchanged. Otherwise return exactly `WORKSPACE_ROOT.join(path)`, retaining `..` components. In particular, `../configured-cli` is allowed and resolves to `WORKSPACE_ROOT.join("../configured-cli")`. Do not reject parent components, canonicalize, probe the filesystem, search `PATH`, or add an error API. Empty, invalid, non-UTF-8, or unlaunchable supplied values remain explicit `Some` inputs and never select Cargo. An explicit launch failure remains visible and does not fall back.

Use one shared private test helper: `configured_prebuilt_cli() -> Option<OsString>` performs the sole environment lookup, and pure `resolve_prebuilt_cli(path: OsString, root: &Path) -> PathBuf` implements the lexical rule. Both NF2U and KYYF runners use it. `None` preserves each runner's exact existing Cargo command and argv. Preserve each runner's existing child working-directory behavior, including KYYF's inherited current directory.

The pure command-selection regression asserts the program and original argv before any spawn; its baseline-red form must fail against the old Cargo builder without executing Cargo. Keep all five NF2U scenarios and all six KYYF comparisons and their assertions intact. These notes authorize no source change by themselves and report no native test result.
