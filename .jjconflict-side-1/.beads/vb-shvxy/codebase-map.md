# Codebase Map: vb-shvxy State 2 Explore

## Scope
- Bead: `vb-shvxy` — Global blocker: restore formal verifier tooling lanes.
- Isolated workspace: `/home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-shvxy`.
- Prior capped evidence context: `/home/lewis/isolated/velvet-ballistics-main-review/vb-ttyc/.beads/vb-ttyc/evidence`.
- Production Rust was not edited.

## Raw discovery evidence
- `pwd -P` returned the isolated workspace path.
- `git status --short --branch` returned branch `fresh/vb-shvxy` with untracked bead artifacts only.
- Current fresh workspace has `scripts/kani-list.sh` and `scripts/flux-check-package.sh` present; prior State 12 attempt 7 logs show those scripts were absent in the capped `vb-ttyc` workspace.
- Current fresh workspace has no `tools/` directory and no `tools/tla2tools.jar`; `command -v tlc` resolves `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`; `TLA2TOOLS_JAR` is unset.
- Tool versions observed: `cargo-kani 0.67.0`, `cargo-flux 4d329f2 (2026-05-23)`, `cargo-fuzz 0.13.1`.
- `cargo fuzz list` succeeds and lists current registered targets, but does not list prior `vb_ttyc_compat_metadata`.
- `rustup run nightly-2026-04-28 cargo -Z unstable-options config get build.target` reports `build.target` unset in this workspace.

## Blocker map

### 1. Kani listing wrapper
- Current file: `scripts/kani-list.sh` lines 1-66.
- Current behavior: requires one or more package names; writes per-package `kani-list.json` under `.evidence/kani-list`.
- Prior blocker evidence: `state12-attempt7/command-004-POB-vb-ttyc-002.log`, `008`, `012`, `017`, `021`, `026`, `031`, `035` all show `bash: scripts/kani-list.sh: No such file or directory`.
- Fresh workspace compatibility risk: prior commands use `KANI_FEATURES=vb_runtime/kani-artifact-version-barrier bash scripts/kani-list.sh vb_runtime --harness ...`; current script does not document or parse `--harness`, and `crates/vb_runtime/Cargo.toml` lines 25-34 does not define `kani-artifact-version-barrier`.
- Probe result: `KANI_FEATURES=vb_runtime/kani-artifact-version-barrier bash scripts/kani-list.sh vb_runtime --harness vb_ttyc_001_kani` fails before harness parsing because `vb_runtime` lacks that feature.
- Downstream scope: `scripts/kani-list.sh`, `crates/vb_runtime/Cargo.toml`, any proof obligations that cite `vb_runtime/kani-artifact-version-barrier`.
- Risk tags: `formal-tooling`, `kani`, `feature-gate-drift`, `proof-command-compatibility`.

### 2. Flux package wrapper
- Current file: `scripts/flux-check-package.sh` lines 1-21.
- Current behavior: runs `cargo flux -p <package> --message-format human` and rejects unsupported selectors `--lib`, `--test`, `--tests`, `--benches`, `--all-targets`.
- Prior blocker evidence: `state12-attempt7/command-005`, `009`, `013`, `018`, `022`, `027`, `032`, `036` all show `bash: scripts/flux-check-package.sh: No such file or directory`.
- Downstream scope: `scripts/flux-check-package.sh`, Flux proof commands in formal reports/obligations.
- Risk tags: `formal-tooling`, `flux`, `proof-command-compatibility`.

### 3. TLA/TLC runner path
- Missing file: `tools/tla2tools.jar` is absent because `tools/` is absent.
- Current alternate runner: `.moon/tasks/tlc.yml` lines 19-25, 65-71, 94-100 allow `tlc` on PATH or `TLA2TOOLS_JAR`.
- Conflicting legacy script: `scripts/run-tlc-checks.sh` lines 1-11 hardcodes a user-local mise jar path and pipes output through `tail -3`.
- Prior blocker evidence: `state12-attempt7/command-015`, `029`, `038`, `041` run `java -jar tools/tla2tools.jar ...` and fail with `Unable to access jarfile tools/tla2tools.jar`.
- Downstream scope: `scripts/run-tlc-checks.sh`, `.moon/tasks/tlc.yml`, proof commands that hardcode `tools/tla2tools.jar`, `verification/tla/**` specs/configs.
- Risk tags: `formal-tooling`, `tla-plus`, `environment-portability`, `raw-evidence-loss`.

### 4. Proptest filters that execute zero tests
- Prior blocker evidence: `state12-attempt7/command-006`, `010`, and `037` show commands like `cargo test -p vb_runtime --test vb_ttyc_artifact_compatibility_properties vb_ttyc_001_proptest -- --nocapture` exiting 0 with `running 0 tests`, `0 passed`, `1 filtered out`.
- Current search: no current `*.rs` file contains `vb_ttyc_00[1-8]_proptest`; fresh main does not contain the capped bead's proptest test file.
- Current repository has many proptest uses, but no discovered generic script/gate that parses cargo-test output and fails on zero applicable tests.
- Downstream scope: proptest command generation/validation layer, formal verifier evidence parser, and any restored `vb_ttyc_*_proptest` test names.
- Risk tags: `formal-tooling`, `proptest`, `vacuous-test-evidence`, `evidence-parser`.

### 5. cargo-fuzz sanitizer/musl issue
- Prior blocker evidence: `state12-attempt7/command-024`, `040`, `042`, `043` show `cargo fuzz run vb_ttyc_compat_metadata ...` building with target `x86_64-unknown-linux-musl` and `-Zsanitizer=address`, then failing the fuzz script build.
- Prior ledger context: `verification-ledger.jsonl` line 106 records the exact blocker as `sanitizer is incompatible with statically linked libc`; lines 108-110 record successful fuzz runs when `--target x86_64-unknown-linux-gnu` is supplied.
- Current config evidence: `.moon/tasks/all.yml` lines 452-470 uses `cargo fuzz build --target x86_64-unknown-linux-gnu` and executes compiled fuzz binaries from `fuzz/target/x86_64-unknown-linux-gnu/...`.
- Current direct-command risk: `.cargo/config.toml` does not set `build.target`; direct `cargo fuzz run <target>` commands can still inherit an external/default target and reproduce the musl sanitizer failure unless proof commands specify `--target x86_64-unknown-linux-gnu` or tooling wraps it.
- Current target inventory: `fuzz/Cargo.toml` has registered fuzz bins; `cargo fuzz list` succeeds, but `vb_ttyc_compat_metadata` is absent in fresh main.
- Downstream scope: `fuzz/Cargo.toml`, `.moon/tasks/all.yml` fuzz-smoke task, proof command generation for cargo-fuzz, possibly `.cargo/config.toml` or a fuzz wrapper.
- Risk tags: `formal-tooling`, `cargo-fuzz`, `sanitizer`, `target-triple`, `environment-portability`.

### 6. Loom cfg/dependency wiring
- Current cfg allowance: root `Cargo.toml` line 69 allows `cfg(loom)` under `unexpected_cfgs`.
- Current dependency: `crates/vb_runtime/Cargo.toml` lines 19-23 has `loom = "0.7"` only as a dev-dependency.
- Current model wiring: `crates/vb_runtime/src/models/mod.rs` lines 3-7 gates `loom` and `sync` modules behind `#[cfg(loom)]`; `crates/vb_runtime/src/models/sync.rs` re-exports `loom` types under `#[cfg(loom)]`.
- Current xtask runner: `xtask/src/loom.rs` lines 29-44 runs `cargo test -p vb_runtime <model>` with `RUSTFLAGS="--cfg loom"`.
- Prior blocker evidence: `state12-attempt7/command-039` runs `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --test vb_ttyc_snapshot_consistency_loom ...` and fails with unresolved module/crate `loom` from `crates/vb_runtime/src/models/sync.rs` and model files.
- Likely scope: integration tests compiled as separate crates do not get `vb_runtime` dev-dependencies transitively; if `#[cfg(loom)]` exposes production library modules that import the external `loom` crate, `loom` must be available to the library build under that cfg, not only to package tests.
- Downstream scope: `crates/vb_runtime/Cargo.toml`, `crates/vb_runtime/src/models/**`, `xtask/src/loom.rs`, any `crates/vb_runtime/tests/*loom*.rs` restored from capped bead.
- Risk tags: `formal-tooling`, `loom`, `concurrency`, `cfg-wiring`, `dependency-boundary`.

## Existing relevant files read or located
- `Cargo.toml` lines 1-101: workspace membership, dependency pins, lint cfg allowance.
- `crates/vb_runtime/Cargo.toml` lines 1-37: runtime dependencies/features/dev-dependencies.
- `fuzz/Cargo.toml` lines 1-497: cargo-fuzz manifest and registered bins.
- `.cargo/config.toml` lines 1-10: no target triple configured.
- `.moon/tasks/all.yml` lines 452-470: fuzz-smoke uses GNU target.
- `.moon/tasks/tlc.yml` lines 14-116: TLC tasks prefer PATH `tlc` or `TLA2TOOLS_JAR`.
- `.moon/tasks/kani.yml` lines 14-58: existing Kani Moon tasks still use direct `cargo kani`, not `scripts/kani-list.sh`.
- `scripts/kani-list.sh`, `scripts/flux-check-package.sh`, `scripts/run-tlc-checks.sh`.
- `xtask/src/loom.rs`, `xtask/Cargo.toml`, `crates/vb_runtime/src/models/mod.rs`, `crates/vb_runtime/src/models/sync.rs`.

## Open questions for downstream agents
- Should global verifier commands standardize on `tlc`/`TLA2TOOLS_JAR`, or vendor/download `tools/tla2tools.jar`? Current master instructions mention `tools/tla2tools.jar` in failed proof commands, but Moon tasks already support PATH-based TLC.
- Should `scripts/kani-list.sh` support harness pass-through, or should proof commands call it only for inventory before separate `cargo kani --harness` execution?
- Should the missing `vb_runtime/kani-artifact-version-barrier` feature be restored, or should capped `vb-ttyc` obligations be rewritten to existing `vb_runtime` feature names?
- What is the canonical place to fail closed on `running 0 tests`: formal-verifier command wrapper, evidence parser, or a cargo-test helper script?
- Should Loom use a real cargo feature, a cfg-only lane with optional dependency, or dev-only package tests only? The current cfg-only approach failed for an integration-test command in prior evidence.

## Recommended downstream owners
- `rust-contract`: model verifier command contracts and fail-closed evidence semantics; no runtime product behavior changes expected.
- `proof-planner`/`proof-plan-reviewer`: convert each blocker into explicit tooling obligations, including zero-test and target-triple fail-closed rules.
- `proof-writer`/`functional-rust`: implement only tooling/config/script changes after plan approval; avoid production Rust behavior changes.
- `formal-verifier`: rerun each lane with exact raw evidence, including negative zero-test detector fixtures.
