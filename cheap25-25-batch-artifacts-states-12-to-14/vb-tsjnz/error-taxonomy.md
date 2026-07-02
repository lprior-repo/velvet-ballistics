# Error Taxonomy — vb-tsjnz

- bead_id: `vb-tsjnz`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- capture: 2026-07-01

This file enumerates every failure class the patch can produce, at two layers: **A — patch-text errors** (the Edit itself is wrong) and **B — cargo-build errors** (cargo rejects the file or the build fails because the workspace lints now enforce against `vb_queue_semantics/src/lib.rs`). Layer B is further subdivided into `B1 — cargo` (manifest/dependency resolution) and `B2 — rustc+clippy` (build/lint engine).

Naming convention: errors are referenced as `VBTSJNZ/<Layer>/<Variant>` in other artifacts.

## Layer A — Patch-Text Errors (Edit-Time)

These are caught by the holzman-rust implementer inspecting the diff; they should never reach `cargo`.

### A1 `ManifestPatchError`

```text
enum ManifestPatchError {
    HardcodedVersionKept { line: usize, observed: String },
    MissingLintsBlock { file: PathBuf },
    LintsBlockShapeWrong { file: PathBuf, reason: &'static str },
    OutOfScopeFieldTouched { file: PathBuf, field_or_table: String },
    UnexpectedSiblingShapeDivergence { sibling: PathBuf, target: PathBuf },
    InternalBlankLinesDisturbed { where_: &'static str },
}
```

| Variant | Triggered by | Recovery |
| --- | --- | --- |
| `HardcodedVersionKept { line: 3, observed: "0.1.0" }` | Edit missed line 3 | Re-edit line 3 to `version.workspace = true`. |
| `MissingLintsBlock` | Edit did not append trailing table | Append `[lints]\nworkspace = true` after the `[dependencies]` block. |
| `LintsBlockShapeWrong { reason: "multi-key" \| "sub-table" \| "string-quoted" }` | Block shape diverges from sibling pattern | Match `vb_core/Cargo.toml:33-34` shape exactly. |
| `OutOfScopeFieldTouched { field_or_table: "[dependencies]" \| "crates/vb_queue_semantics/src/lib.rs" \| "publish" \| ... }` | Edit bled past its scope | Restore untouched field by reverse edit. |
| `UnexpectedSiblingShapeDivergence` | The target's `[lints]` block has trailing whitespace, mixed line endings, or a key order not present in siblings | Adjust to byte-identical sibling shape. |
| `InternalBlankLinesDisturbed { where_: "post-[package]" \| "pre-[lints]" }` | Edit added/removed blank lines | Restore original blank-line pattern; only modify the targeted lines. |

### A2 `PreFlightError`

```text
enum PreFlightError {
    NotInIsolatedWorkspace { pwd: PathBuf, expected: PathBuf },
    SiblingFileMissing { path: PathBuf },
    CargoNotFound,
    WorkspaceLintsTableEmpty,
}
```

| Variant | Recovery |
| --- | --- |
| `NotInIsolatedWorkspace` | Stop. Move into the JJ workspace at `~/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`. |
| `SiblingFileMissing` | Halt; a sibling reference crate was deleted; this bead cannot match a non-existent pattern. |
| `CargoNotFound` | Install toolchain; `rust-toolchain.toml` pins nightly per governance. |
| `WorkspaceLintsTableEmpty` | Workspace policy itself is broken; abort bead, escalate. |

## Layer B1 — Cargo Manifest Errors (Resolution Time)

If the Edit is textually valid but semantically wrong, cargo emits errors when the workspace is loaded.

### B1 `ManifestResolutionError`

```text
enum ManifestResolutionError {
    WorkspacePackageUndeclared { key: "version" },
    VersionFieldTypeMismatch { observed_type: Type },
    VersionWorkspaceBoolFalse { line: usize },
    LintsWorkspaceBoolFalse { line: usize },
    DuplicatePackageKey { key: String },
    UnknownManifestKey { key: String },
}
```

These are emitted by `cargo metadata` / `cargo check` BEFORE source compilation. Any non-zero exit at this layer is `B1/*` and the patch is **incorrect**, not the source.

## Layer B2 — Rust-Build / Clippy Errors (Compile / Lint Time)

This is the most likely failure surface for vb-tsjnz because adding workspace lints converts previously-accepted source into build-breaking source.

### B2 `BuildLintTrip`

```text
enum BuildLintTrip {
    RustLint { name: &'static str, file: PathBuf, line: usize },
    ClippyLint { name: &'static str, file: PathBuf, line: usize, level: LintLevel },
    CompileError { file: PathBuf, line: usize, message: String },
    UndefinedForbiddenPattern { kind: ForbiddenPattern, file: PathBuf, line: usize },
}

enum LintLevel { Allow, Warn, Deny, Forbid }

enum ForbiddenPattern {
    Unwrap,           // unwrap_used = "forbid"
    Expect,           // expect_used = "forbid"
    Panic,            // panic = "forbid"
    PanicInResultFn,  // panic_in_result_fn = "forbid"
    Todo,             // todo = "forbid"
    Unimplemented,    // unimplemented = "forbid"
    Dbg,              // dbg_macro = "forbid"
    StringSlice,      // string_slice = "forbid"
    GetUnwrap,        // get_unwrap = "deny"
    IndexingSlice,    // indexing_slicing = "deny"
    AsConversion,     // as_conversions = "deny"
    ArithmeticSideEffect, // arithmetic_side_effects = "deny"
    LetUnderscoreMustUse, // let_underscore_must_use = "deny"
    AwaitHoldingLock, // await_holding_lock = "deny"
    LargeStackArray,  // large_stack_arrays = "warn" (warning-only)
    LargeTypeByValue, // large_types_passed_by_value = "warn"
    ResultLargeErr,   // result_large_err = "warn"
}
```

Note: `await_holding_lock` and `arithmetic_side_effects` are `deny` (warning-grade; will print). `unwrap_used`, `expect_used`, `panic`, `panic_in_result_fn`, `todo`, `unimplemented`, `dbg_macro`, `string_slice` are `forbid` (compile error). The hierarchy matters: a `forbid` lint becomes a hard compile error; a `deny` lint becomes a hard compile error under `-D warnings`. Both should be treated as compile-blocking in this bead.

### B2-FAILURE-HANDLING-POLICY

If any `B2` error fires, the patch MUST be considered a failure of **verification**, not of design:

1. Holzman-rust MUST NOT modify the workspace lint policy to silence the lint.
2. Holzman-rust MUST NOT introduce `#[allow(...)]` annotations on `vb_queue_semantics` source without a separate waiver-bearing bead.
3. Holzman-rust MUST report `LintFailure` to the controller and hand off the source-cleanup as `NextBead` (this is exactly the case the scout flagged: "UNCONFIRMED whether the 423-line lib.rs is clean against workspace lints").

### B3 `TestFailure`

```text
enum TestFailure {
    WorkspaceAssertionMissing { crate: String },
    QualityGateTrip { reason: String },
}
```

These are downstream gates; they enumerate `vb_queue_semantics` already, so a green build is necessary for them to run, but a green build does not guarantee they pass. The `TestFailure` cases are catchable separately.

## Layer C — Semantic Drift Errors (Post-Landing Audit)

Caught by black-hat reviewer after landing.

```text
enum DriftError {
    VersionLiteralReintroduced,
    LintsBlockRemoved,
    LintsBlockShapeMutated,
    OutOfScopeEditAdmitted,
    ExceptionFileModified,
}
```

Any of these means the bead has regressed; the post-landing audit is the only catch point for these.

## Discouraged Recovery Moves (Forbidden Repairs)

The following are **forbidden** under Holzman-Rust policy and would amount to the A rule "no Loop Oscillations":

- Disabling a workspace lint to silence a deny trip.
- Adding a `#[allow(...)]` to `vb_queue_semantics` source without a waiver ledger entry.
- Lowering the lint priority at workspace root.
- Mutating `contracts/` or other contract artifacts to retroactively permit the failure.

## Recovery Allowed

- Re-edit the patch text to restore the canonical sibling shape.
- Re-run cargo check / clippy.
- Open a follow-up bead (`NextBead`) ONLY if a `B2` lint trip is genuine and unrelated to the metadata patch itself (e.g. an honest pre-existing `unwrap()` in `vb_queue_semantics/src/lib.rs`); the patch then lands or fails based on whether the source can be made lint-clean in scope.
