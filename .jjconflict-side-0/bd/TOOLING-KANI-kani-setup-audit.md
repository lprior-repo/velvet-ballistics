# TOOLING-KANI setup-audit / kani-setup

## Verdict

BLOCKED. Kani is installed, but repository Kani lanes fail during setup/codegen before selected harness execution. I did not attempt bead proof closure and did not run CBMC verification.

No setup patch was applied. The observed failures are not a single low-risk convention fix: `vb_core` globally wires a set of stale diagnostic harness modules under `#[cfg(kani)]`, so selecting any `vb_core` harness, or any downstream crate depending on `vb_core`, compiles unrelated broken Kani modules first.

## Commands Run

- `bd show TOOLING-KANI`
- Exit: non-zero
- Summary: `Error fetching TOOLING-KANI: no issue found matching "TOOLING-KANI"`; Dolt server auto-started on port `43055`.

- `cargo kani --version`
- Exit: 0
- Output: `cargo-kani 0.67.0`

- `cargo kani list --format json`
- Exit: 1
- Output: `Kani Rust Verifier 0.67.0 (cargo plugin)` then `error: No supported targets were found.`
- Diagnosis: root workspace is pure virtual, but `Cargo.toml` has `default-members = ["."]`; Kani list does not accept package selection in this installed CLI form, so workspace inventory cannot be produced from the root.

- `cargo kani list --format json -p vb_core`
- Exit: 1
- Output: `error: unexpected argument '-p' found`

- `cargo kani -p vb_core list --format json`
- Exit: 1
- Output: `error: unexpected argument '--format' found`

- `cargo kani list --help`
- Exit: 0
- Summary: `cargo-kani list` supports `--format`, but no package selector is available on the `list` subcommand.

- `cargo kani -p vb_core --harness kani_symbolic_code_determinism --only-codegen`
- Exit: 101
- Resource control: `--only-codegen`, 120s timeout, no CBMC solver run, no cgroup needed.
- Summary: failed compiling `vb_core` before target harness execution with 26 compile errors.

- `cargo kani -p vb_runtime --harness command_queue_bounds --only-codegen`
- Exit: 101
- Resource control: `--only-codegen`, 120s timeout, no CBMC solver run, no cgroup needed.
- Summary: failed compiling dependency `vb_core` before target harness execution with the same 26 compile errors.

- `rustc --version --verbose`
- Exit: 0
- Output: `rustc 1.97.0-nightly (52b6e2c20 2026-04-27)`, host `x86_64-unknown-linux-gnu`, LLVM `22.1.2`.

- `cargo --version`
- Exit: 0
- Output: `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)`

## Root Causes

1. Root Kani inventory is broken for this pure virtual workspace.

`cargo kani list --format json` from the workspace root reports `No supported targets were found.` The root `Cargo.toml` declares a virtual workspace and `default-members = ["."]`, but `.` is not a package. The installed Kani `list` subcommand also rejected both tested package-selection forms, so root-level inventory cannot currently be scoped to a package.

2. Targeted Kani package selection still globally compiles unrelated `cfg(kani)` modules.

`crates/vb_core/src/lib.rs` exposes `#[cfg(kani)] pub mod kani;`. `crates/vb_core/src/kani/mod.rs` then unconditionally wires ten diagnostic harness modules. A command selecting one harness still compiles every child module under `vb_core::kani`, so stale modules block unrelated target harnesses.

3. `vb_core::kani` contains mutually incompatible local models and duplicate impls.

The codegen failures include:

- Private tuple constructor use for `kani::kani_symbolic_code_validation::SymbolicCode` in `kani_determinism.rs`, `kani_diagnostic_constructor.rs`, `kani_reverse_lookup.rs`, and `kani_serde_roundtrip.rs`.
- Private helper imports: `numeric_to_symbolic` from `kani_reverse_lookup.rs` and `is_supported_code` from `kani_is_supported_code.rs`.
- Duplicate inherent `DiagnosticCode::symbolic_code` definitions across `kani_determinism.rs`, `kani_diagnostic_constructor.rs`, and `kani_reverse_lookup.rs`.
- Const-unstable `Option::map` / `Option::and_then` inside const functions in `kani_symbolic_code_validation.rs` and `kani_from_str_compat.rs`.
- Lifetime escape in `kani_serde_roundtrip.rs`: `deserialize_symbolic_code(&str)` attempts to construct `SymbolicCode` requiring `&'static str` from a non-static borrow.

4. Downstream Kani lanes inherit the `vb_core` compile blocker.

The `vb_runtime` targeted codegen command did not reach `command_queue_bounds`; it failed while compiling dependency `vb_core`. This matches the reported symptom that Kani lanes fail before target harness execution.

## Harness Inventory Notes

Source scan found many Kani harness surfaces across crates, including `vb_core`, `vb_compile`, `vb_storage`, `vb_runtime`, `vb_validate`, `vb_yaml`, `vb_ipc`, `vb_boundary_inventory`, and `workspace_tests`. Because `cargo kani list --format json` failed, no authoritative Kani JSON harness inventory was produced in this audit.

Notable setup convention issue: Kani harness files are often direct `#[cfg(kani)]` crate modules rather than package-feature-isolated harness groups. This makes unrelated broken harnesses compile globally under `cfg(kani)`.

## Why No Patch Was Applied

A safe tiny patch would need to improve isolation without changing proof semantics or hiding obligations. The available fixes are larger than that:

- Changing root `default-members` affects Cargo default behavior for the workspace.
- Gating `vb_core::kani` child modules behind new Cargo features would require a repository-wide convention and command updates for existing Kani obligations.
- Repairing the diagnostic harness model requires consolidating duplicated `DiagnosticCode`/`SymbolicCode` model impls, replacing const-unstable APIs, and fixing lifetime/private-field boundaries across several files.

Applying only one compiler suggestion, such as making `SymbolicCode(pub &'static str)`, would not unblock Kani and would weaken the model boundary.

## Recommended Next Setup Beads

1. Create a Kani inventory wrapper task.
   Implement a repo script or xtask that runs Kani inventory from package directories or supported package manifests and writes a merged JSON inventory. Acceptance: root command produces machine-readable harness inventory without running CBMC.

2. Introduce Kani feature isolation conventions.
   Add package features such as `kani-diagnostic-codes`, `kani-runtime`, or per-bead feature groups, then gate bulky/stale harness module groups with `#[cfg(all(kani, feature = "..."))]`. Acceptance: `cargo kani -p vb_runtime --features <runtime-kani-feature> --harness command_queue_bounds --only-codegen` does not compile unrelated `vb_core::kani` diagnostic modules.

3. Repair or quarantine `vb_core/src/kani` diagnostic modules as one coherent unit.
   Consolidate duplicate model impls, expose helper functions only within the module group, remove const-unstable `Option::{map,and_then}`, and fix the `&str` to `&'static str` construction path. Acceptance: `cargo kani -p vb_core --features <diagnostic-kani-feature> --harness kani_symbolic_code_determinism --only-codegen` reaches codegen.

4. Add CI smoke lanes for Kani setup only.
   Add bounded `--only-codegen` package/harness smoke commands with timeouts to catch compile-before-harness failures without invoking CBMC.

## Non-Claims

- No Kani proof was executed to `VERIFICATION:- SUCCESSFUL`.
- No CBMC solver run was started.
- No harness correctness, bound adequacy, or non-vacuity claim is made.
- `--only-codegen` was used strictly as setup/compile evidence, not proof evidence.
